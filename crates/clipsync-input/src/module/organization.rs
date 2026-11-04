//! Monitor inventory, v4 sessions and the local layout transaction.
use super::*;
use crate::{
    inventory::{InventoryCatalog, ObservedMonitors},
    layout_document::{LayoutDocument, LegacyBinding, Organization},
    layout_geometry::{
        CompiledOrganization, InventoryView, OrganizationDraft, compile_organization,
    },
    layout_storage::LayoutStorage,
    monitor_identities::IdentityStore,
    route_session::{LocalReturn, RoutePlan, RouteSession},
    route_track::RouteTrack,
};
use clipsync_proto::{
    ActivationState, DurabilityState, EnterRoutes, ExitRoute, GeometrySource, InventoryRevisions,
    InventoryStamp, LayoutOfferV4, MonitorInventory, MonitorLink, OrganizationApplied,
    OrganizationFailure, OrganizationPreview, OrganizationRequest, PendingMonitorOffer, RouteId,
    SessionRoutes, Validate,
};

#[derive(Default)]
pub(super) struct OrganizationRuntime {
    pub catalog: InventoryCatalog,
    pub cleanup: capture::DeferredCleanup,
    pub preparation: Option<Preparation>,
    pub inventory_blocked: bool,
    pub document: Option<LayoutDocument>,
    pub problem: Option<String>,
    pub pending: Option<PendingMonitorOffer>,
    pub observed: Option<ObservedMonitors>,
    pub aliases: Vec<LegacyBinding>,
    pub suspended: bool,
    pub ingress_suspended: bool,
    pub remote_epochs: HashMap<PeerId, u64>,
    pub plans: HashMap<usize, RoutePlan>,
    /// Why the last route build came out empty (contract §17.2); `None` after a build with targets.
    pub route_reason: Option<&'static str>,
    /// Links of this machine considered in the last route build, for `routes.built`.
    pub route_links: usize,
}
pub(super) struct Prepared {
    cap: Option<Box<dyn CaptureAdapter>>,
    result: Result<ObservedMonitors, InputError>,
    reply: Option<oneshot::Sender<ModuleReply>>,
}
pub(super) struct Preparation {
    cancel: Option<oneshot::Sender<()>>,
    pub future: std::pin::Pin<Box<dyn std::future::Future<Output = Prepared> + Send>>,
    authority: clipsync_proto::HostLifecycle,
    peers: Vec<(PeerId, Option<clipsync_proto::PeerInputContext>)>,
}
fn fail(code: impl Into<String>, rev: Option<u64>) -> OrganizationFailure {
    OrganizationFailure {
        code: code.into(),
        current_rev: rev,
    }
}
impl State {
    pub(super) fn current_rev(&self) -> u64 {
        self.layout.as_ref().map_or(0, |l| l.rev)
    }
    pub(super) fn profile(&self) -> Result<&Path, OrganizationFailure> {
        self.layout_path
            .as_deref()
            .and_then(Path::parent)
            .ok_or_else(|| fail("profile_unavailable", None))
    }
    pub(super) fn inventory_valid(&self, stamps: &[InventoryStamp]) -> bool {
        if !self.organization.catalog.matches(stamps) {
            return false;
        }
        stamps.iter().all(|s| {
            if s.peer == self.host.local_id() {
                self.cap.as_ref().is_none_or(|c| {
                    c.known_monitor_observation().ok().is_some_and(|o| {
                        self.organization
                            .observed
                            .as_ref()
                            .is_some_and(|old| crate::inventory::same_observation(old, &o))
                    })
                })
            } else {
                self.host.peer_input_context(s.peer).is_some_and(|c| {
                    c.connected
                        && c.connection == s.connection
                        && self.organization.remote_epochs.get(&s.peer) == Some(&c.input_epoch)
                })
            }
        })
    }
    pub(super) fn local_inventory_valid(&self, rev: u64) -> bool {
        self.inventory_valid(&[InventoryStamp {
            peer: self.host.local_id(),
            connection: 0,
            rev,
        }])
    }
    async fn known_inventory(&mut self) -> Result<ObservedMonitors, InputError> {
        if let Some(cap) = &self.cap {
            return cap.known_monitor_observation();
        }
        let c = self
            .cont
            .as_ref()
            .ok_or_else(crate::inventory::unavailable)?;
        let (reply, rx) = oneshot::channel();
        c.ctl
            .send(Ctl::KnownInventory { reply })
            .map_err(|_| crate::inventory::unavailable())?;
        tokio::time::timeout(crate::network::SEND_WAIT, rx)
            .await
            .map_err(|_| crate::inventory::unavailable())?
            .map_err(|_| crate::inventory::unavailable())?
    }
    pub(super) async fn confirm_local_inventory(&mut self, rev: u64) -> bool {
        self.local_inventory_valid(rev)
            && self.known_inventory().await.ok().is_some_and(|o| {
                self.organization
                    .observed
                    .as_ref()
                    .is_some_and(|old| crate::inventory::same_observation(old, &o))
            })
    }
    pub(super) async fn invalidate_monitor_sessions(&mut self) {
        for inc in self.incoming.values_mut() {
            if inc.routes.is_some() {
                inc.active = false;
            }
        }
        // Pressed state is released before any dependent network operation.
        self.release_all().await;
        self.send_ctl(Ctl::InvalidateRoutes);
    }
    pub(super) async fn observe_inventory(
        &mut self,
        observation: ObservedMonitors,
    ) -> Result<(), InputError> {
        let me = self.host.local_id();
        let current = self.known_inventory().await?;
        if !self.preparation_valid() {
            return Err(InputError::Other(msg::SESSION_CANCELLED.into()));
        }
        if !crate::inventory::same_observation(&current, &observation) {
            return Err(crate::inventory::unavailable());
        }
        if observation.source != GeometrySource::Observed {
            return Err(crate::inventory::unavailable());
        }
        if self
            .organization
            .observed
            .as_ref()
            .is_some_and(|o| crate::inventory::same_observation(o, &observation))
            && self.organization.catalog.get(me).is_some()
        {
            return Ok(());
        }
        self.organization.catalog.invalidate(me);
        self.invalidate_monitor_sessions().await;
        if !self.preparation_valid() {
            return Err(InputError::Other(msg::SESSION_CANCELLED.into()));
        }
        let profile = self
            .profile()
            .map_err(|_| crate::inventory::unavailable())?;
        let snapshot = IdentityStore::new(profile)
            .reconcile(&observation.observations)
            .map_err(|_| crate::inventory::unavailable())?;
        let inventory = MonitorInventory {
            rev: snapshot.rev,
            space: observation.space,
            source: observation.source,
            capabilities: observation.capabilities,
            monitors: snapshot.monitors,
        };
        let monitores = inventory.monitors.len();
        let uids = inventory
            .monitors
            .iter()
            .map(|m| m.uid.to_hex())
            .collect::<Vec<_>>();
        let uids = serde_json::to_string(&uids).unwrap_or_else(|_| "[]".into());
        self.organization.catalog.insert(
            me,
            InventoryStamp {
                peer: me,
                connection: 0,
                rev: inventory.rev,
            },
            inventory,
        )?;
        tracing::info!(evento = "inventory.prepared", monitores, uids = %uids, "local inventory prepared");
        self.organization.aliases = snapshot
            .aliases
            .into_iter()
            .map(|a| LegacyBinding {
                peer: me,
                alias: a.alias,
                uid: a.uid,
            })
            .collect();
        self.organization.observed = Some(observation);
        self.organization.problem = None;
        self.announce_inventory_all();
        Ok(())
    }
    pub(super) async fn refresh_known_inventory(&mut self) {
        if self.organization.inventory_blocked {
            return;
        }
        let observation = self.cap.as_ref().map(|c| c.known_monitor_observation());
        match observation {
            Some(Ok(o)) => {
                if let Err(e) = self.observe_inventory(o).await {
                    self.organization.problem = Some(e.to_string());
                }
            }
            Some(Err(_)) => {
                if self
                    .organization
                    .catalog
                    .get(self.host.local_id())
                    .is_some()
                {
                    self.organization.catalog.invalidate(self.host.local_id());
                    self.invalidate_monitor_sessions().await;
                }
                self.organization.problem =
                    Some("inventário observado indisponível; prepare os monitores".into());
                if self.organization.observed.is_some() {
                    self.begin_preparation(None).await;
                }
            }
            None => {}
        }
        for entry in self.organization.catalog.snapshot() {
            if entry.stamp.peer != self.host.local_id() && !self.inventory_valid(&[entry.stamp]) {
                self.organization.catalog.invalidate(entry.stamp.peer);
                self.invalidate_monitor_sessions().await;
            }
        }
    }
    pub(super) fn preparation_valid(&self) -> bool {
        self.organization.preparation.as_ref().is_none_or(|p| {
            let now = capture::HostCursor::new(self.host.as_ref()).snapshot;
            !self.paused
                && !self.host.paused()
                && now == p.authority
                && p.peers
                    .iter()
                    .all(|(id, old)| self.host.peer_input_context(*id) == *old)
        })
    }
    pub(super) async fn begin_preparation(&mut self, reply: Option<oneshot::Sender<ModuleReply>>) {
        self.cancel_preparation().await;
        self.organization.inventory_blocked = true;
        self.organization.suspended = true;
        self.organization.catalog.invalidate(self.host.local_id());
        for incoming in self.incoming.values_mut() {
            incoming.active = false;
        }
        // Every key/button is released before a portal operation or capture stop can wait/fail.
        if self.emu_started
            && let Err(e) = self.emu.release_all().await
        {
            if let Some(reply) = reply {
                let _ = reply.send(ModuleReply::Error(e.to_string()));
            }
            return;
        }
        if self.paused || self.host.paused() || self.ctl.is_some() {
            if let Some(reply) = reply {
                let _ = reply.send(ModuleReply::Error(msg::SESSION_CANCELLED.into()));
            }
            return;
        }
        let engine = self.cont.as_ref().map(|c| c.ctl.clone());
        let mut cap = self.cap.take();
        let (cancel, cancelled) = oneshot::channel();
        let peers = self
            .host
            .peers()
            .iter()
            .map(|p| (p.id, self.host.peer_input_context(p.id)))
            .collect();
        let authority = capture::HostCursor::new(self.host.as_ref()).snapshot;
        let future = Box::pin(async move {
            let mut reply = reply;
            let result = {
                let work = async {
                    if let Some(ctl) = engine {
                        let (reply, rx) = oneshot::channel();
                        ctl.send(Ctl::PrepareInventory { reply })
                            .map_err(|_| crate::inventory::unavailable())?;
                        rx.await.map_err(|_| crate::inventory::unavailable())?
                    } else if let Some(cap) = cap.as_mut() {
                        // State owns the adapter only without an active producer. The Engine
                        // branch performs its uncancelled stop/join after closing its receiver.
                        cap.release().await?;
                        cap.prepare_monitor_observation().await
                    } else {
                        Err(crate::inventory::unavailable())
                    }
                };
                tokio::select! {
                    biased;
                    _ = cancelled => Err(crate::inventory::unavailable()),
                    _ = async { if let Some(r) = reply.as_mut() { r.closed().await; } else { std::future::pending::<()>().await; } } => Err(crate::inventory::unavailable()),
                    result = work => result,
                }
            };
            Prepared { cap, result, reply }
        });
        self.organization.preparation = Some(Preparation {
            cancel: Some(cancel),
            future,
            authority,
            peers,
        });
    }
    pub(super) async fn finish_preparation(&mut self, mut done: Prepared, cancelled: bool) {
        let valid = !cancelled && self.preparation_valid();
        // Keep the authority/peer epochs until observe_inventory finishes its awaits
        // and publishes synchronously. A ready adapter result is not publication authority.
        if let Some(cap) = done.cap.take() {
            self.cap = Some(cap);
        }
        let result = if valid {
            done.result
        } else {
            Err(crate::inventory::unavailable())
        };
        let result = match result {
            Ok(o) => {
                let result = self.observe_inventory(o).await;
                self.organization.inventory_blocked = result.is_err();
                result
            }
            Err(e) => {
                self.organization.problem = Some(e.to_string());
                Err(e)
            }
        };
        self.organization.preparation = None;
        if let Some(reply) = done.reply {
            let _ = reply.send(match result {
                Ok(()) => ModuleReply::Done,
                Err(e) => ModuleReply::Error(e.to_string()),
            });
        }
    }
    pub(super) async fn cancel_preparation(&mut self) {
        if let Some(p) = self.organization.preparation.as_mut() {
            if let Some(cancel) = p.cancel.take() {
                let _ = cancel.send(());
            }
            let done = (&mut p.future).await;
            self.finish_preparation(done, true).await;
        }
    }
    pub(super) fn announce_inventory(&mut self, info: &PeerInfo) {
        if !info.connected || info.remote_version < 4 || !info.perms.input_in {
            return;
        }
        let Some(local) = self.organization.catalog.get(self.host.local_id()) else {
            return;
        };
        let Some(context) = self
            .host
            .peer_input_context(info.id)
            .filter(|c| c.connected)
        else {
            return;
        };
        let host = self.host.clone();
        let to = info.id;
        let monitores = local.inventory.monitors.len();
        let revisao = local.inventory.rev;
        if !self.network.spawn(async move {
            let sent = host
                .send_reliable_on(
                    to,
                    context.connection,
                    ModuleMsg::Input(InputMsg::InventoryV4(local.inventory)),
                )
                .await;
            match &sent {
                Ok(_) => tracing::info!(evento = "inventory.sent", peer = %to, monitores, revisao, "inventory sent"),
                Err(e) => tracing::info!(evento = "inventory.missing", peer = %to, motivo = "send_failed", erro = %e, "inventory not sent"),
            }
            sent
        }) {
            self.last_block = Some(msg::NETWORK_BUSY.into());
        }
    }
    pub(super) fn announce_inventory_all(&mut self) {
        for p in self.peers.values().cloned().collect::<Vec<_>>() {
            self.announce_inventory(&p);
        }
    }
    pub(super) async fn receive_inventory(
        &mut self,
        from: PeerId,
        context: ReceiveContext,
        inventory: MonitorInventory,
    ) {
        // Inventory permission is transport's outgoing Zones gate, never input_in in reverse.
        if self
            .host
            .peers()
            .iter()
            .all(|p| p.id != from || p.remote_version < 4)
        {
            self.ignored += 1;
            return;
        }
        let monitores = inventory.monitors.len();
        let revisao = inventory.rev;
        let changed =
            self.organization.catalog.get(from).is_none_or(|v| {
                v.inventory != inventory || v.stamp.connection != context.connection
            });
        if changed {
            self.organization.catalog.invalidate(from);
            self.invalidate_monitor_sessions().await;
        }
        self.organization
            .remote_epochs
            .insert(from, context.input_epoch);
        if self
            .organization
            .catalog
            .insert(
                self.host.local_id(),
                InventoryStamp {
                    peer: from,
                    connection: context.connection,
                    rev: inventory.rev,
                },
                inventory,
            )
            .is_err()
        {
            self.ignored += 1;
            return;
        }
        tracing::info!(evento = "inventory.received", peer = %from, monitores, revisao, "inventory received");
        self.sync_links().await;
    }
    pub(super) async fn enter_routes(
        &mut self,
        from: PeerId,
        entry: EnterRoutes,
        context: ReceiveContext,
    ) {
        tracing::info!(evento = "enter.received", session = entry.session, peer = %from, mensagem = "enter_routes", "entry received");
        if self.organization.ingress_suspended || !self.authorized(from, context) {
            self.refuse_on(
                from,
                entry.session,
                context.connection,
                RejectReason::NotAuthorized,
            );
            return;
        }
        let Some(index) = context.stream else {
            self.ignored += 1;
            return;
        };
        let f = self.frontier(from, context.connection);
        if f.closed.is_some_and(|n| index <= n) || f.entered.is_some_and(|n| index <= n) {
            self.ignored += 1;
            return;
        }
        f.entered = Some(index);
        let Some(local) = self.organization.catalog.get(self.host.local_id()) else {
            self.refuse_on_with(
                from,
                entry.session,
                context.connection,
                RejectReason::ApplyFailed,
                "no_local_inventory",
            );
            return;
        };
        // The entry monitor must exist here: the route cannot be tracked without it.
        if !local
            .inventory
            .monitors
            .iter()
            .any(|m| m.uid == entry.monitor)
        {
            self.refuse_on_with(
                from,
                entry.session,
                context.connection,
                RejectReason::ApplyFailed,
                "monitor_not_found",
            );
            return;
        }
        let track = match RouteTrack::new(&local.inventory, &entry) {
            Ok(t) => t,
            Err(_) => {
                self.refuse_on_with(
                    from,
                    entry.session,
                    context.connection,
                    RejectReason::ApplyFailed,
                    "route_invalid",
                );
                return;
            }
        };
        if !self.confirm_local_inventory(entry.inventories.target).await
            || !self.authorized(from, context)
        {
            let motivo = if self.authorized(from, context) {
                "inventory_unconfirmed"
            } else {
                "not_authorized"
            };
            self.refuse_on_with(
                from,
                entry.session,
                context.connection,
                RejectReason::ApplyFailed,
                motivo,
            );
            return;
        }
        let Some(monitor) = local
            .inventory
            .monitors
            .iter()
            .find(|m| m.uid == entry.monitor)
        else {
            self.refuse_on_with(
                from,
                entry.session,
                context.connection,
                RejectReason::ApplyFailed,
                "monitor_not_found",
            );
            return;
        };
        let zone = Zone {
            index: 0,
            name: Some(monitor.label.clone()),
            x: monitor.rect.x,
            y: monitor.rect.y,
            width: monitor.rect.width,
            height: monitor.rect.height,
            scale: monitor.scale,
        };
        if let Some(old) = self.incoming.remove(&from)
            && old.active
        {
            self.release_all().await;
        }
        if !self.confirm_local_inventory(entry.inventories.target).await
            || self.organization.ingress_suspended
            || !self.authorized(from, context)
        {
            return;
        }
        if !self.emu_started {
            tracing::info!(evento = "emulate.start", session = entry.session, peer = %from, adaptador = self.emu.name(), "emulator starting");
            if let Err(e) = self.emu.start().await {
                tracing::info!(evento = "emulate.error", session = entry.session, peer = %from, adaptador = self.emu.name(), erro = %e, "emulator did not start");
                self.release_all().await;
                self.refuse_on_with(
                    from,
                    entry.session,
                    context.connection,
                    RejectReason::ApplyFailed,
                    "emulator_start_failed",
                );
                return;
            }
            self.emu_started = true;
        }
        if !self.confirm_local_inventory(entry.inventories.target).await
            || self.organization.ingress_suspended
            || !self.authorized(from, context)
        {
            self.release_all().await;
            return;
        }
        if let Err(e) = self
            .emu
            .enter(&zone, std::slice::from_ref(&zone), entry.edge, entry.pos)
            .await
        {
            tracing::info!(evento = "emulate.error", session = entry.session, peer = %from, adaptador = self.emu.name(), erro = %e, "entry positioning failed");
            self.release_all().await;
            self.refuse_on_with(
                from,
                entry.session,
                context.connection,
                RejectReason::ApplyFailed,
                "entry_positioning_failed",
            );
            return;
        }
        tracing::info!(evento = "emulate.positioned", session = entry.session, peer = %from, adaptador = self.emu.name(), pos = capture::frac3(entry.pos), "entry positioned");
        if !self.confirm_local_inventory(entry.inventories.target).await
            || self.organization.ingress_suspended
            || !self.authorized(from, context)
        {
            self.release_all().await;
            return;
        }
        let mut incoming = Incoming::new(entry.session, context);
        incoming.routes = Some((track, entry.inventories));
        self.incoming.insert(from, incoming);
    }
    pub(super) fn organization_targets(&mut self, zones: &[Zone]) -> Vec<Target> {
        self.organization.plans.clear();
        self.organization.route_links = 0;
        if self.organization.suspended {
            self.organization.route_reason = Some("suspended");
            return vec![];
        }
        let me = self.host.local_id();
        let Some(local) = self.organization.catalog.get(me) else {
            self.organization.route_reason = Some("no_local_inventory");
            return vec![];
        };
        // Saving a copy is independent of being a supported physical source.
        if !local.inventory.capabilities.positioned_return
            || !local.inventory.capabilities.internal_edges
            || local.inventory.space != clipsync_proto::CoordinateSpace::Physical
        {
            self.organization.problem = Some("origem local sem retorno posicionado".into());
            self.organization.route_reason = Some("local_without_return");
            return vec![];
        }
        let Some(org) = self
            .organization
            .document
            .as_ref()
            .and_then(|d| d.organization.as_ref())
        else {
            self.organization.route_reason = Some("no_organization");
            return vec![];
        };
        let views: Vec<_> = self
            .organization
            .catalog
            .snapshot()
            .into_iter()
            .map(|v| InventoryView {
                peer: v.stamp.peer,
                inventory: v.inventory,
                aliases: vec![],
            })
            .collect();
        let Some(document) = self.organization.document.as_ref() else {
            self.organization.route_reason = Some("no_organization");
            return vec![];
        };
        let (legacy, unresolved) =
            match crate::layout_geometry::resolved_legacy_links(document, &views) {
                Ok(v) => v,
                Err(e) => {
                    self.organization.problem = Some(e.code().into());
                    self.organization.route_reason = Some("unresolved_legacy");
                    return vec![];
                }
            };
        if !unresolved.is_empty() {
            self.organization.problem =
                Some("ligações legadas sem identidade confirmada permanecem inativas".into());
        }
        let mut links = Vec::new();
        let mut why: Option<&'static str> = None;
        for l in org.links.iter().chain(legacy.iter()) {
            let (a, b) = if l.a.key.peer == me {
                (&l.a, &l.b)
            } else if l.b.key.peer == me {
                (&l.b, &l.a)
            } else {
                continue;
            };
            let Some(remote) = self.organization.catalog.get(b.key.peer) else {
                why = Some("no_remote_inventory");
                continue;
            };
            if !self.inventory_valid(&[local.stamp, remote.stamp])
                || !remote.inventory.capabilities.route_receiver
                || self
                    .host
                    .peers()
                    .iter()
                    .all(|p| p.id != b.key.peer || p.remote_version < 4 || !p.connected)
            {
                why = Some("remote_not_ready");
                continue;
            }
            let Some(m) = local.inventory.monitors.iter().find(|m| m.uid == a.key.uid) else {
                why = Some("local_monitor_missing");
                continue;
            };
            if !remote.inventory.monitors.iter().any(|m| m.uid == b.key.uid) {
                why = Some("remote_monitor_missing");
                continue;
            }
            let matches: Vec<_> = zones
                .iter()
                .filter(|z| {
                    z.x == m.rect.x
                        && z.y == m.rect.y
                        && z.width == m.rect.width
                        && z.height == m.rect.height
                        && z.scale == m.scale
                })
                .collect();
            if matches.len() != 1 {
                why = Some("zone_not_found");
                continue;
            }
            links.push((a.clone(), b.clone(), matches[0].index, remote));
        }
        let targets: Vec<_> = links
            .iter()
            .map(|(a, b, index, _)| Target {
                spec: BarrierSpec {
                    zone: ZoneRef::Index(*index),
                    edge: a.edge,
                    from: a.from,
                    to: a.to,
                },
                peer: b.key.peer,
                peer_monitor: None,
                peer_edge: b.edge,
                local: (a.from, a.to),
                remote: (b.from, b.to),
            })
            .collect();
        for (i, (_, b, _, remote)) in links.iter().enumerate() {
            let same: Vec<_> = links
                .iter()
                .enumerate()
                .filter(|(_, (_, e, _, _))| e.key == b.key)
                .collect();
            if same.is_empty() || same.len() > 2 {
                continue;
            }
            let exits: Vec<_> = same
                .iter()
                .enumerate()
                .map(|(r, (_, (_, e, _, _)))| ExitRoute {
                    id: RouteId(r as u8),
                    monitor: e.key.uid,
                    edge: e.edge,
                    from: e.from,
                    to: e.to,
                })
                .collect();
            let returns = same
                .iter()
                .enumerate()
                .map(|(r, (barrier, (a, e, _, _)))| LocalReturn {
                    id: RouteId(r as u8),
                    key: a.key,
                    barrier: *barrier,
                    local: (a.from, a.to),
                    remote: (e.from, e.to),
                })
                .collect();
            let inventories = InventoryRevisions {
                source: local.stamp.rev,
                target: remote.stamp.rev,
            };
            self.organization.plans.insert(
                i,
                RoutePlan {
                    entry: EnterRoutes {
                        session: 1,
                        inventories,
                        monitor: b.key.uid,
                        edge: b.edge,
                        pos: 0.5,
                        routes: SessionRoutes {
                            first: exits[0].clone(),
                            second: exits.get(1).cloned(),
                        },
                    },
                    session: RouteSession {
                        peer: b.key.peer,
                        connection: remote.stamp.connection,
                        session: 1,
                        inventories,
                        returns,
                    },
                },
            );
        }
        // Every target must have a v4 plan; never degrade this document to legacy return.
        if self.organization.plans.len() != targets.len() {
            self.organization.problem =
                Some("unsupported_topology: mais de duas saídas para o monitor receptor".into());
            self.organization.plans.clear();
            self.organization.route_reason = Some("unsupported_topology");
            return vec![];
        }
        self.organization.route_links = links.len();
        self.organization.route_reason = if targets.is_empty() {
            Some(why.unwrap_or("no_links"))
        } else {
            None
        };
        targets
    }
}

impl State {
    fn compile_request(
        &self,
        current: &LayoutDocument,
        request: &OrganizationRequest,
        require_local_origin: bool,
    ) -> Result<CompiledOrganization, OrganizationFailure> {
        let rev = current.legacy.rev;
        if rev != request.base_rev {
            return Err(fail("stale_rev", Some(rev)));
        }
        if request.inventories.iter().any(|s| {
            s.peer != self.host.local_id()
                && self
                    .host
                    .peers()
                    .iter()
                    .all(|p| p.id != s.peer || p.remote_version < 4)
        }) {
            return Err(fail("unsupported_version", Some(rev)));
        }
        if !self.inventory_valid(&request.inventories) {
            return Err(fail("inventory_changed", Some(rev)));
        }
        let peers: std::collections::HashSet<_> =
            request.positions.iter().map(|p| p.key.peer).collect();
        if peers.len() != request.inventories.len()
            || request.inventories.iter().any(|s| !peers.contains(&s.peer))
        {
            return Err(fail("inventory_changed", Some(rev)));
        }
        let views: Vec<_> = request
            .inventories
            .iter()
            .filter_map(|s| self.organization.catalog.get(s.peer))
            .map(|v| InventoryView {
                peer: v.stamp.peer,
                inventory: v.inventory,
                aliases: if v.stamp.peer == self.host.local_id() {
                    self.organization.aliases.clone()
                } else {
                    vec![]
                },
            })
            .collect();
        if require_local_origin
            && views
                .iter()
                .filter(|v| {
                    v.inventory.space == clipsync_proto::CoordinateSpace::Physical
                        && v.inventory.capabilities.internal_edges
                        && v.inventory.capabilities.positioned_return
                })
                .map(|v| v.peer)
                .min()
                != Some(self.host.local_id())
        {
            return Err(fail("unsupported_origin", Some(rev)));
        }
        let mut compilation_base = current.clone();
        for peer in &peers {
            if !compilation_base
                .legacy
                .devices
                .iter()
                .any(|d| d.peer == *peer)
            {
                if *peer != self.host.local_id() && self.host.peers().iter().all(|p| p.id != *peer)
                {
                    return Err(fail("unknown_peer", Some(rev)));
                }
                compilation_base.legacy.devices.push(crate::LayoutDevice {
                    peer: *peer,
                    name: self.peer_name(*peer),
                    x: 0,
                    y: 0,
                    hotkey: None,
                });
            }
        }
        compile_organization(
            &compilation_base,
            &OrganizationDraft {
                positions: request.positions.clone(),
                replace_link_ids: request.replace_link_ids.clone(),
            },
            &views,
        )
        .map_err(|e| fail(e.code(), Some(rev)))
    }
    pub(super) async fn preview_organization(
        &mut self,
        request: OrganizationRequest,
    ) -> Result<OrganizationPreview, OrganizationFailure> {
        let current = LayoutDocument::read(&self.profile()?.join(crate::LAYOUT_FILE))
            .map_err(|e| fail(e.code(), None))?
            .unwrap_or_default();
        let compiled = self.compile_request(&current, &request, true)?;
        Ok(OrganizationPreview {
            base_rev: request.base_rev,
            inventories: request.inventories,
            positions: request.positions,
            effective_links: compiled.effective_links,
            preserved_link_ids: compiled.preserved_link_ids,
            replaced_link_ids: compiled.replaced_link_ids,
            unresolved_link_ids: compiled.unresolved_link_ids,
        })
    }
    pub(super) async fn quiesce_organization(&mut self) -> Result<(), OrganizationFailure> {
        self.organization.suspended = true;
        self.organization.ingress_suspended = true;
        self.organization.plans.clear();
        for inc in self.incoming.values_mut() {
            inc.active = false;
        }
        let deadline = tokio::time::Instant::now() + crate::network::SEND_WAIT;
        if self.emu_started {
            tokio::time::timeout_at(deadline, self.emu.release_all())
                .await
                .map_err(|_| fail("quiesce_timeout", None))?
                .map_err(|_| fail("quiesce_failed", None))?;
        }
        self.cancel_preparation().await;
        if let Some(c) = &self.ctl {
            let _ = c.tx.send(LoopSignal::Stop);
            return Err(fail("quiesce_busy", Some(self.current_rev())));
        }
        if let Some(c) = &self.cont {
            let (reply, rx) = oneshot::channel();
            c.ctl
                .send(Ctl::Quiesce { reply })
                .map_err(|_| fail("quiesce_failed", Some(self.current_rev())))?;
            tokio::time::timeout_at(deadline, rx)
                .await
                .map_err(|_| fail("quiesce_timeout", Some(self.current_rev())))?
                .map_err(|_| fail("quiesce_failed", Some(self.current_rev())))?
                .map_err(|_| fail("quiesce_failed", Some(self.current_rev())))?;
            tokio::time::timeout_at(deadline, self.stop_continuous())
                .await
                .map_err(|_| fail("quiesce_timeout", Some(self.current_rev())))?;
        } else if let Some(cap) = self.cap.as_mut() {
            tokio::time::timeout_at(deadline, cap.stop())
                .await
                .map_err(|_| fail("quiesce_timeout", None))?
                .map_err(|_| fail("quiesce_failed", None))?;
        }

        Ok(())
    }
    async fn activate_document(
        &mut self,
        document: LayoutDocument,
        stamps: &[InventoryStamp],
    ) -> bool {
        self.layout = Some(document.legacy.clone());
        self.organization.document = Some(document);
        if !self.inventory_valid(stamps) {
            return false;
        }
        self.organization.ingress_suspended = false;
        if self
            .organization
            .catalog
            .get(self.host.local_id())
            .is_some_and(|v| !v.inventory.capabilities.positioned_return)
        {
            self.organization.problem =
                Some("cópia salva; origem local sem retorno posicionado".into());
            return false;
        }
        self.organization.suspended = false;
        self.start_continuous().await;
        let ready = if let Some(c) = &self.cont {
            let (reply, rx) = oneshot::channel();
            if c.ctl.send(Ctl::Ready { reply }).is_err() {
                false
            } else {
                matches!(
                    tokio::time::timeout(crate::network::SEND_WAIT, rx).await,
                    Ok(Ok(true))
                )
            }
        } else {
            false
        };
        let local_rev = stamps
            .iter()
            .find(|s| s.peer == self.host.local_id())
            .map_or(0, |s| s.rev);
        if !ready || !self.confirm_local_inventory(local_rev).await || !self.inventory_valid(stamps)
        {
            let _ = self.quiesce_organization().await;
            return false;
        }
        true
    }
    pub(super) async fn apply_organization(
        &mut self,
        request: OrganizationRequest,
    ) -> Result<OrganizationApplied, OrganizationFailure> {
        let storage = LayoutStorage::new(self.profile()?);
        let guard = storage.try_lock().map_err(|e| fail(e.code(), None))?;
        let current = guard
            .read()
            .map_err(|e| fail(e.code(), None))?
            .unwrap_or_default();
        let mut compiled = self.compile_request(&current, &request, true)?;
        self.quiesce_organization().await?;
        // Await boundaries may change geometry or connection even though the module queue is blocked.
        if !self.inventory_valid(&request.inventories) {
            return Err(fail("inventory_changed", Some(current.legacy.rev)));
        }
        compiled.document.legacy.rev = current
            .legacy
            .rev
            .checked_add(1)
            .ok_or_else(|| fail("revision_exhausted", Some(current.legacy.rev)))?;
        let revision = compiled.document.legacy.rev;
        match guard.write(&compiled.document) {
            Err(crate::LayoutError::PublishedUnconfirmed { rev }) => {
                self.layout = Some(compiled.document.legacy.clone());
                self.organization.document = Some(compiled.document);
                return Ok(OrganizationApplied {
                    layout_rev: rev,
                    activation: ActivationState::Unconfirmed,
                    durability: DurabilityState::Unconfirmed,
                });
            }
            Err(e) => return Err(fail(e.code(), Some(current.legacy.rev))),
            Ok(()) => {}
        }
        drop(guard);
        let active = self
            .activate_document(compiled.document, &request.inventories)
            .await;
        Ok(OrganizationApplied {
            layout_rev: revision,
            activation: if active {
                ActivationState::Confirmed
            } else {
                ActivationState::Unconfirmed
            },
            durability: DurabilityState::Confirmed,
        })
    }
    pub(super) async fn restore_layout_v1(
        &mut self,
        base_rev: u64,
    ) -> Result<OrganizationApplied, OrganizationFailure> {
        let storage = LayoutStorage::new(self.profile()?);
        let guard = storage.try_lock().map_err(|e| fail(e.code(), None))?;
        let current = guard
            .read()
            .map_err(|e| fail(e.code(), None))?
            .ok_or_else(|| fail("layout_missing", None))?;
        if current.legacy.rev != base_rev {
            return Err(fail("stale_rev", Some(current.legacy.rev)));
        }
        self.quiesce_organization().await?;
        let rev = match guard.restore_v1(base_rev) {
            Ok(rev) => rev,
            Err(crate::LayoutError::PublishedUnconfirmed { rev }) => {
                // Observe the published v1 without retry, rollback or rearming.
                self.organization.document = guard.read().ok().flatten();
                self.layout = self
                    .organization
                    .document
                    .as_ref()
                    .map(|d| d.legacy.clone());
                return Ok(OrganizationApplied {
                    layout_rev: rev,
                    activation: ActivationState::Unconfirmed,
                    durability: DurabilityState::Unconfirmed,
                });
            }
            Err(e) => return Err(fail(e.code(), Some(base_rev))),
        };
        self.organization.document = guard.read().map_err(|e| fail(e.code(), Some(rev)))?;
        self.layout = self
            .organization
            .document
            .as_ref()
            .map(|d| d.legacy.clone());
        drop(guard);
        self.organization.suspended = false;
        self.organization.ingress_suspended = false;
        self.start_continuous().await;
        Ok(OrganizationApplied {
            layout_rev: rev,
            activation: ActivationState::Unconfirmed,
            durability: DurabilityState::Confirmed,
        })
    }
    pub(super) async fn offer_v4(&mut self, to: Vec<PeerId>, reply: oneshot::Sender<ModuleReply>) {
        if self.offer_job.is_some() {
            let _ = reply.send(ModuleReply::Error(msg::OFFER_BUSY.into()));
            return;
        }
        let Some(document) = self.organization.document.as_ref() else {
            let _ = reply.send(ModuleReply::Error(msg::NO_LAYOUT.into()));
            return;
        };
        let Some(org) = &document.organization else {
            let _ = reply.send(ModuleReply::Error("mapa por monitor ausente".into()));
            return;
        };
        let legacy = layout_offer(&document.legacy);
        let offer = LayoutOfferV4 {
            rev: legacy.rev,
            devices: legacy.devices,
            positions: org.positions.clone(),
            links: org.links.clone(),
            legacy_links: legacy.links,
        };
        if offer.validate().is_err() {
            let _ = reply.send(ModuleReply::Error(
                "mapa não representável em oferta v4".into(),
            ));
            return;
        }
        let mut outcomes = vec![];
        let mut waiting = 0;
        for peer in to {
            let state = match self.peers.get(&peer) {
                None => OfferState::Offline,
                Some(p) if !p.connected => OfferState::Offline,
                Some(p) if p.remote_version < 4 => OfferState::Unsupported,
                Some(_) => {
                    if let Some(context) =
                        self.host.peer_input_context(peer).filter(|c| c.connected)
                        && self.network.has_capacity()
                    {
                        let host = self.host.clone();
                        let offer = offer.clone();
                        let mut completion = OfferCompletion {
                            notes: self.notes.clone(),
                            peer,
                            state: OfferState::Denied,
                        };
                        self.network.spawn(async move {
                            let state = match host
                                .send_reliable_on(
                                    peer,
                                    context.connection,
                                    ModuleMsg::Input(InputMsg::LayoutOfferV4(offer)),
                                )
                                .await
                            {
                                Ok(()) => OfferState::Offered,
                                Err(SendError::RemoteTooOld) => OfferState::Unsupported,
                                Err(SendError::NotConnected) => OfferState::Offline,
                                Err(_) => OfferState::Denied,
                            };
                            completion.set(state);
                        });
                        waiting += 1;
                    }
                    OfferState::Denied
                }
            };
            outcomes.push(OfferOutcome { peer, state });
        }
        if waiting > 0 {
            self.offer_job = Some(OfferJob {
                outcomes,
                reply,
                until: Instant::now() + crate::network::SEND_WAIT + OFFER_GRACE,
                waiting,
                rejected: Default::default(),
            });
        } else {
            let _ = reply.send(ModuleReply::InputOffered(outcomes));
        }
    }
    pub(super) async fn answer_offer_v4(
        &mut self,
        from: PeerId,
        connection: u64,
        rev: u64,
        base_rev: u64,
        accept: bool,
    ) -> Result<Option<OrganizationApplied>, OrganizationFailure> {
        let pending = self
            .organization
            .pending
            .as_ref()
            .filter(|p| p.from == from && p.connection == connection && p.offer.rev == rev)
            .cloned()
            .ok_or_else(|| fail("offer_changed", Some(self.current_rev())))?;
        if self
            .host
            .peer_input_context(from)
            .is_none_or(|c| !c.connected || c.connection != connection)
        {
            return Err(fail("offer_changed", Some(self.current_rev())));
        }
        if !accept {
            self.organization.pending = None;
            return Ok(None);
        }
        let result = self.accept_monitor_offer(&pending, base_rev).await;
        match result {
            Ok(applied) => {
                self.organization.pending = None;
                Ok(Some(applied))
            }
            Err(e) => {
                self.organization.problem = Some(e.code.clone());
                Err(e)
            }
        }
    }
    async fn accept_monitor_offer(
        &mut self,
        pending: &PendingMonitorOffer,
        base_rev: u64,
    ) -> Result<OrganizationApplied, OrganizationFailure> {
        let offer = &pending.offer;
        offer.validate().map_err(|_| fail("invalid_offer", None))?;
        let known: std::collections::HashSet<_> = self
            .host
            .peers()
            .iter()
            .map(|p| p.id)
            .chain(std::iter::once(self.host.local_id()))
            .collect();
        if offer.devices.iter().any(|d| !known.contains(&d.peer)) {
            return Err(fail("unknown_peer", None));
        }
        let storage = LayoutStorage::new(self.profile()?);
        let guard = storage.try_lock().map_err(|e| fail(e.code(), None))?;
        let current = guard
            .read()
            .map_err(|e| fail(e.code(), None))?
            .unwrap_or_default();
        if current.legacy.rev != base_rev {
            return Err(fail("stale_rev", Some(current.legacy.rev)));
        }
        let mut document = current.clone();
        document.legacy.devices = offer
            .devices
            .iter()
            .map(|d| crate::LayoutDevice {
                peer: d.peer,
                name: current
                    .legacy
                    .devices
                    .iter()
                    .find(|v| v.peer == d.peer)
                    .map(|v| v.name.clone())
                    .unwrap_or_else(|| self.peer_name(d.peer)),
                x: d.x,
                y: d.y,
                hotkey: current
                    .legacy
                    .devices
                    .iter()
                    .find(|v| v.peer == d.peer)
                    .and_then(|v| v.hotkey),
            })
            .collect();
        document.legacy.links = offer
            .legacy_links
            .iter()
            .enumerate()
            .map(|(i, l)| crate::Link {
                id: format!("offered-legacy-{i}"),
                a: Endpoint {
                    peer: l.a.peer,
                    monitor: l.a.monitor.clone(),
                    edge: l.a.edge,
                    from: l.a.from,
                    to: l.a.to,
                },
                b: Endpoint {
                    peer: l.b.peer,
                    monitor: l.b.monitor.clone(),
                    edge: l.b.edge,
                    from: l.b.from,
                    to: l.b.to,
                },
            })
            .collect();
        document.organization = Some(Organization {
            positions: offer.positions.clone(),
            links: offer.links.clone(),
            legacy_bindings: current
                .organization
                .as_ref()
                .map(|o| o.legacy_bindings.clone())
                .unwrap_or_default(),
        });
        document.validate().map_err(|e| fail(e.code(), None))?;
        let mut stamps = vec![];
        for p in &offer.positions {
            if !known.contains(&p.key.peer) {
                return Err(fail("unknown_peer", None));
            }
            let inventory = self
                .organization
                .catalog
                .get(p.key.peer)
                .ok_or_else(|| fail("inventory_missing", Some(base_rev)))?;
            if !inventory
                .inventory
                .monitors
                .iter()
                .any(|m| m.uid == p.key.uid)
            {
                return Err(fail("monitor_missing", Some(base_rev)));
            }
            if !stamps.contains(&inventory.stamp) {
                stamps.push(inventory.stamp);
            }
        }
        // Legacy endpoints remain raw and obey the receiver's preserved options.
        // Never validate those by an inferred trim belonging to offered monitor links.
        let legacy_views: Vec<_> = self
            .organization
            .catalog
            .snapshot()
            .into_iter()
            .filter(|v| self.inventory_valid(&[v.stamp]))
            .map(|v| InventoryView {
                peer: v.stamp.peer,
                inventory: v.inventory,
                aliases: vec![],
            })
            .collect();
        crate::layout_geometry::resolved_legacy_links(&document, &legacy_views)
            .map_err(|e| fail(e.code(), Some(base_rev)))?;
        // No trim provenance is needed on the wire: options are integers in the
        // fixed 0..=MAX_DEAD_CORNER_PX domain. A single allowed value must
        // reproduce every effective endpoint of this pair; arbitrary subsets fail.
        let mut pairs = std::collections::BTreeSet::new();
        for l in &offer.links {
            let a = l.a.key.peer.min(l.b.key.peer);
            let b = l.a.key.peer.max(l.b.key.peer);
            pairs.insert((a, b));
        }
        for (a, b) in pairs {
            let positions = offer
                .positions
                .iter()
                .filter(|p| p.key.peer == a || p.key.peer == b)
                .cloned()
                .collect();
            let links: Vec<_> = offer
                .links
                .iter()
                .filter(|l| {
                    (l.a.key.peer == a && l.b.key.peer == b)
                        || (l.a.key.peer == b && l.b.key.peer == a)
                })
                .collect();
            let request = OrganizationRequest {
                base_rev,
                inventories: stamps
                    .iter()
                    .filter(|s| s.peer == a || s.peer == b)
                    .copied()
                    .collect(),
                positions,
                replace_link_ids: links.iter().map(|l| l.id.clone()).collect(),
            };
            let endpoints = |l: &MonitorLink| (l.a.clone(), l.b.clone());
            let mut geometry_base = document.clone();
            geometry_base.legacy.links.clear(); // already validated with local options above
            let matches = (0..=crate::layout::MAX_DEAD_CORNER_PX).any(|dead| {
                geometry_base.legacy.options.dead_corner_px = dead;
                self.compile_request(&geometry_base, &request, false)
                    .is_ok_and(|compiled| {
                        compiled.effective_links.len() == links.len()
                            && compiled.effective_links.iter().all(|l| {
                                links.iter().any(|o| {
                                    endpoints(l) == endpoints(o)
                                        || endpoints(l) == (o.b.clone(), o.a.clone())
                                })
                            })
                    })
            });
            if !matches {
                return Err(fail("offered_geometry_mismatch", Some(base_rev)));
            }
        }
        if !self.inventory_valid(&stamps) {
            return Err(fail("inventory_changed", Some(base_rev)));
        }
        self.quiesce_organization().await?;
        if !self.inventory_valid(&stamps)
            || self
                .host
                .peer_input_context(pending.from)
                .is_none_or(|c| !c.connected || c.connection != pending.connection)
        {
            return Err(fail("inventory_changed", Some(base_rev)));
        }
        document.legacy.rev = base_rev
            .checked_add(1)
            .ok_or_else(|| fail("revision_exhausted", Some(base_rev)))?;
        let rev = document.legacy.rev;
        match guard.write(&document) {
            Err(crate::LayoutError::PublishedUnconfirmed { rev }) => {
                self.layout = Some(document.legacy.clone());
                self.organization.document = Some(document);
                return Ok(OrganizationApplied {
                    layout_rev: rev,
                    activation: ActivationState::Unconfirmed,
                    durability: DurabilityState::Unconfirmed,
                });
            }
            Err(e) => return Err(fail(e.code(), Some(base_rev))),
            Ok(()) => {}
        }
        drop(guard);
        let active = self.activate_document(document, &stamps).await;
        Ok(OrganizationApplied {
            layout_rev: rev,
            activation: if active {
                ActivationState::Confirmed
            } else {
                ActivationState::Unconfirmed
            },
            durability: DurabilityState::Confirmed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserved_legacy_runtime_resolves_only_evidence_and_trims_once() {
        let a = PeerId([1; 32]);
        let b = PeerId([2; 32]);
        let view = |peer, uid| InventoryView {
            peer,
            aliases: vec![],
            inventory: MonitorInventory {
                rev: 1,
                space: clipsync_proto::CoordinateSpace::Logical,
                source: GeometrySource::Observed,
                capabilities: clipsync_proto::MonitorCapabilities {
                    internal_edges: false,
                    positioned_return: false,
                    route_receiver: true,
                },
                monitors: vec![clipsync_proto::MonitorInfo {
                    uid: clipsync_proto::MonitorUid([uid; 16]),
                    label: "one".into(),
                    rect: clipsync_proto::NativeRect {
                        x: 0,
                        y: 0,
                        width: 1000,
                        height: 1000,
                    },
                    scale: 1.0,
                }],
            },
        };
        let views = vec![view(a, 1), view(b, 2)];
        let endpoint = |peer, edge| Endpoint {
            peer,
            monitor: "one".into(),
            edge,
            from: 0.0,
            to: 1.0,
        };
        let mut document = LayoutDocument {
            organization: Some(Organization::default()),
            ..Default::default()
        };
        document.legacy.links.push(crate::Link {
            id: "advanced".into(),
            a: endpoint(a, Edge::Right),
            b: endpoint(b, Edge::Left),
        });
        let (links, missing) =
            crate::layout_geometry::resolved_legacy_links(&document, &views).unwrap();
        assert!(missing.is_empty());
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].a.from, 8.0 / 1000.0);
        let saved = document.clone();
        let (again, _) = crate::layout_geometry::resolved_legacy_links(&document, &views).unwrap();
        assert_eq!(links, again);
        assert_eq!(document, saved);
        document.legacy.links[0].a.monitor = "#0".into();
        let (links, missing) =
            crate::layout_geometry::resolved_legacy_links(&document, &views).unwrap();
        assert!(links.is_empty());
        assert_eq!(missing, vec!["advanced"]);
        document = saved;
        document.organization.as_mut().unwrap().links = again;
        assert!(crate::layout_geometry::resolved_legacy_links(&document, &views).is_err());
    }
}
