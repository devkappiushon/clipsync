//! Fronteira JSON da organização. Identidades hex e bordas minúsculas só na UI.
use crate::events::{LayoutChanged, LayoutReason, UiEvent};
use crate::{Ctx, UiError, api, layout};
use clipsync_proto as proto;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Key {
    pub peer: String,
    pub uid: proto::MonitorUid,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub key: Key,
    pub x: f64,
    pub y: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stamp {
    pub peer: String,
    pub connection: u64,
    pub rev: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveInventory {
    pub stamp: Stamp,
    pub inventory: proto::MonitorInventory,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub key: Key,
    pub edge: layout::Edge,
    pub from: f32,
    pub to: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub id: String,
    pub a: Endpoint,
    pub b: Endpoint,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Organization {
    pub positions: Vec<Position>,
    pub links: Vec<Link>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationRequest {
    pub base_rev: u64,
    pub inventories: Vec<Stamp>,
    pub positions: Vec<Position>,
    pub replace_link_ids: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationPreview {
    pub base_rev: u64,
    pub inventories: Vec<Stamp>,
    pub positions: Vec<Position>,
    pub effective_links: Vec<Link>,
    pub preserved_link_ids: Vec<String>,
    pub replaced_link_ids: Vec<String>,
    pub unresolved_link_ids: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confirmation {
    Confirmed,
    Unconfirmed,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationApplied {
    pub layout_rev: u64,
    pub activation: Confirmation,
    pub durability: Confirmation,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Offer {
    pub from_peer_id: String,
    pub from_name: String,
    pub connection: u64,
    pub rev: u64,
    pub positions: Vec<Position>,
    pub links: Vec<Link>,
    pub legacy_links: Vec<layout::Link>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreArgs {
    pub base_rev: u64,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopyArgs {
    pub to_peer_ids: Vec<String>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerArgs {
    pub from_peer_id: String,
    pub connection: u64,
    pub rev: u64,
    pub base_rev: u64,
    pub accept: bool,
}
impl From<proto::MonitorKey> for Key {
    fn from(v: proto::MonitorKey) -> Self {
        Self {
            peer: v.peer.to_hex(),
            uid: v.uid,
        }
    }
}
impl From<proto::MonitorPosition> for Position {
    fn from(v: proto::MonitorPosition) -> Self {
        Self {
            key: v.key.into(),
            x: v.x,
            y: v.y,
        }
    }
}
impl From<proto::InventoryStamp> for Stamp {
    fn from(v: proto::InventoryStamp) -> Self {
        Self {
            peer: v.peer.to_hex(),
            connection: v.connection,
            rev: v.rev,
        }
    }
}
impl From<proto::LiveInventory> for LiveInventory {
    fn from(v: proto::LiveInventory) -> Self {
        Self {
            stamp: v.stamp.into(),
            inventory: v.inventory,
        }
    }
}
impl From<proto::MonitorEndpoint> for Endpoint {
    fn from(v: proto::MonitorEndpoint) -> Self {
        Self {
            key: v.key.into(),
            edge: v.edge.into(),
            from: v.from,
            to: v.to,
        }
    }
}
impl From<proto::MonitorLink> for Link {
    fn from(v: proto::MonitorLink) -> Self {
        Self {
            id: v.id,
            a: v.a.into(),
            b: v.b.into(),
        }
    }
}
impl From<proto::OrganizationPreview> for OrganizationPreview {
    fn from(v: proto::OrganizationPreview) -> Self {
        Self {
            base_rev: v.base_rev,
            inventories: v.inventories.into_iter().map(Into::into).collect(),
            positions: v.positions.into_iter().map(Into::into).collect(),
            effective_links: v.effective_links.into_iter().map(Into::into).collect(),
            preserved_link_ids: v.preserved_link_ids,
            replaced_link_ids: v.replaced_link_ids,
            unresolved_link_ids: v.unresolved_link_ids,
        }
    }
}
impl From<proto::OrganizationApplied> for OrganizationApplied {
    fn from(v: proto::OrganizationApplied) -> Self {
        Self {
            layout_rev: v.layout_rev,
            activation: if v.activation == proto::ActivationState::Confirmed {
                Confirmation::Confirmed
            } else {
                Confirmation::Unconfirmed
            },
            durability: if v.durability == proto::DurabilityState::Confirmed {
                Confirmation::Confirmed
            } else {
                Confirmation::Unconfirmed
            },
        }
    }
}
impl OrganizationRequest {
    pub(crate) fn control(self) -> Result<proto::OrganizationRequest, UiError> {
        if self.positions.len() > 32
            || self.inventories.len() > 16
            || self.replace_link_ids.len() > 64
        {
            return Err(UiError::bad_request("organização excede o limite"));
        }
        let positions = self
            .positions
            .into_iter()
            .map(|p| {
                if !p.x.is_finite()
                    || !p.y.is_finite()
                    || p.x.abs() > 1_000_000.
                    || p.y.abs() > 1_000_000.
                {
                    return Err(UiError::bad_request("posição inválida"));
                }
                Ok(proto::MonitorPosition {
                    key: proto::MonitorKey {
                        peer: api::peer_id(&p.key.peer, "positions.key.peer")?,
                        uid: p.key.uid,
                    },
                    x: p.x,
                    y: p.y,
                })
            })
            .collect::<Result<Vec<_>, UiError>>()?;
        let inventories = self
            .inventories
            .into_iter()
            .map(|s| {
                Ok(proto::InventoryStamp {
                    peer: api::peer_id(&s.peer, "inventories.peer")?,
                    connection: s.connection,
                    rev: s.rev,
                })
            })
            .collect::<Result<Vec<_>, UiError>>()?;
        Ok(proto::OrganizationRequest {
            base_rev: self.base_rev,
            inventories,
            positions,
            replace_link_ids: self.replace_link_ids,
        })
    }
}
pub(crate) fn failure(code: &str, rev: Option<u64>) -> UiError {
    let code = match code {
        "inventory_changed" => "stale_inventory",
        "inventory_missing" => "inventory_required",
        "unsupported_version" => "protocol_required",
        "offer_changed" => "stale_offer",
        c => c,
    };
    let message = match code {
        "identity_ambiguous" => "Tela não identificada com segurança",
        "stale_inventory" => "As telas mudaram. Atualize a prévia antes de aplicar.",
        "stale_rev" => "O layout mudou. Atualize a prévia antes de aplicar.",
        "inventory_required" => {
            "Prepare as telas nesta máquina e no notebook para obter geometria observada."
        }
        "protocol_required" => "A organização por monitor exige protocolo v4 nas duas máquinas.",
        "backup_conflict" => "O backup existente é diferente. A organização não foi gravada.",
        "advanced_link_conflict" => {
            "Há conflito com uma ligação avançada. Revise as ligações preservadas."
        }
        "unsupported_origin" => "Esta jornada precisa começar no desktop Windows.",
        "stale_offer" => "A oferta mudou. Confira a oferta atual antes de responder.",
        _ => "Não foi possível concluir a organização. Confira as telas e tente uma nova prévia.",
    };
    UiError::new(code, message).with_details(serde_json::json!({"current_rev":rev}))
}
async fn ask(ctx: &Ctx, cmd: proto::ModuleCommand) -> Result<proto::ModuleReply, UiError> {
    layout::require_input(ctx)?;
    match ctx.ask_module(proto::ModuleKind::Input, cmd).await? {
        proto::ModuleReply::OrganizationFailure(e) => Err(failure(&e.code, e.current_rev)),
        proto::ModuleReply::Error(_) => Err(failure("inventory_required", None)),
        reply => Ok(reply),
    }
}
pub(crate) fn emit(ctx: &Ctx, rev: u64) {
    ctx.emit(UiEvent::LayoutChanged(LayoutChanged {
        rev,
        reason: LayoutReason::Saved,
    }));
}
pub(crate) async fn prepare(ctx: &Ctx) -> Result<(), UiError> {
    match ask(ctx, proto::ModuleCommand::InputPrepareInventory).await? {
        proto::ModuleReply::Done => Ok(()),
        _ => Err(UiError::internal("resposta inesperada ao preparar telas")),
    }
}
pub(crate) async fn preview(
    ctx: &Ctx,
    args: OrganizationRequest,
) -> Result<OrganizationPreview, UiError> {
    let request = args.control()?;
    preflight(ctx, &request).await?;
    match ask(ctx, proto::ModuleCommand::InputPreviewOrganization(request)).await? {
        proto::ModuleReply::OrganizationPreview(v) => Ok(v.into()),
        _ => Err(UiError::internal("resposta inesperada da prévia")),
    }
}
async fn preflight(ctx: &Ctx, request: &proto::OrganizationRequest) -> Result<(), UiError> {
    layout::require_input(ctx)?;
    let status = crate::status::agent_status(ctx).await;
    if let Some(status) = status {
        for s in &request.inventories {
            if s.peer != status.id
                && status
                    .peers
                    .iter()
                    .any(|p| p.id == s.peer && p.remote_version < 4)
            {
                return Err(failure("protocol_required", Some(request.base_rev)));
            }
        }
    }
    let state = match ask(ctx, proto::ModuleCommand::InputState).await? {
        proto::ModuleReply::InputState(s) => s,
        _ => return Err(failure("inventory_required", Some(request.base_rev))),
    };
    if state.inventory_problem.as_deref() == Some("identity_ambiguous") {
        return Err(failure("identity_ambiguous", Some(request.base_rev)));
    }
    if request.inventories.is_empty()
        || request.inventories.iter().any(|s| {
            state
                .inventories
                .iter()
                .find(|v| v.stamp.peer == s.peer)
                .is_none_or(|v| v.inventory.source != proto::GeometrySource::Observed)
        })
    {
        return Err(failure("inventory_required", Some(request.base_rev)));
    }
    Ok(())
}
async fn applied(ctx: &Ctx, cmd: proto::ModuleCommand) -> Result<OrganizationApplied, UiError> {
    match ask(ctx, cmd).await? {
        proto::ModuleReply::OrganizationApplied(v) => {
            emit(ctx, v.layout_rev);
            Ok(v.into())
        }
        _ => Err(UiError::internal("resposta inesperada da aplicação")),
    }
}
pub(crate) async fn apply(
    ctx: &Ctx,
    args: OrganizationRequest,
) -> Result<OrganizationApplied, UiError> {
    let request = args.control()?;
    preflight(ctx, &request).await?;
    applied(ctx, proto::ModuleCommand::InputApplyOrganization(request)).await
}
pub(crate) async fn restore(ctx: &Ctx, base_rev: u64) -> Result<OrganizationApplied, UiError> {
    applied(ctx, proto::ModuleCommand::InputRestoreLayoutV1 { base_rev }).await
}
pub(crate) async fn copy(ctx: &Ctx, args: CopyArgs) -> Result<layout::CopyReply, UiError> {
    for peer in &args.to_peer_ids {
        api::peer_id(peer, "to_peer_ids")?;
    }
    layout::copy_monitors(ctx, args.to_peer_ids).await
}
pub(crate) async fn answer_offer(
    ctx: &Ctx,
    args: AnswerArgs,
) -> Result<OrganizationApplied, UiError> {
    let from = api::peer_id(&args.from_peer_id, "from_peer_id")?;
    match ask(
        ctx,
        proto::ModuleCommand::InputAnswerOfferV4 {
            from,
            connection: args.connection,
            rev: args.rev,
            base_rev: args.base_rev,
            accept: args.accept,
        },
    )
    .await?
    {
        proto::ModuleReply::OrganizationApplied(v) if args.accept => {
            emit(ctx, v.layout_rev);
            Ok(v.into())
        }
        proto::ModuleReply::Done if !args.accept => {
            let l = layout::get(ctx).await?;
            Ok(OrganizationApplied {
                layout_rev: l.rev,
                activation: Confirmation::Unconfirmed,
                durability: Confirmation::Unconfirmed,
            })
        }
        _ => Err(UiError::internal("resposta inesperada da oferta")),
    }
}
