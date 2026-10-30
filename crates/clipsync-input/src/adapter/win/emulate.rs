//! `WinEmulate` (name `"win-sendinput"`): `SendInput` behind the checks of contract §13.6,
//! "Adaptador Windows" item 2. Motion is absolute, computed from the delta, so the system's pointer
//! acceleration does not act twice. Nothing here logs a key (§13.2 rule 2e).
use super::super::winmath::{self, Rect};
use super::super::{EmulateAdapter, InputError, Zone};
use crate::keymap;
use async_trait::async_trait;
use clipsync_proto::{Edge, InputEvent};
use clipsync_win::{Monitor, Synth, Watcher};
#[cfg(not(test))]
use clipsync_win::{Notice, WinError};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Wheel units per logical pixel of smooth scroll: 120 for every 15 px (to tune at the roteiro).
const WHEEL_PER_PX: f32 = 120.0 / 15.0;

#[derive(Default)]
struct State {
    session_locked: AtomicBool,
    monitors_changed: AtomicBool,
}

pub struct WinEmulate {
    watcher: Option<Watcher>,
    state: Arc<State>,
    monitors: Vec<Monitor>,
    motion_rest: (f32, f32),
    scroll_rest: (f32, f32),
    // Códigos possivelmente pressionados; um lote parcial não revela seu estado físico.
    keys: HashSet<u32>,
    buttons: HashSet<u32>,
    cleanup_pending: bool,
    #[cfg(test)]
    backend: tests::Backend,
}

impl Default for WinEmulate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(test))]
fn io_err(call: &str, e: &WinError) -> InputError {
    InputError::Other(format!("{call}: {e}"))
}

impl WinEmulate {
    pub fn new() -> WinEmulate {
        WinEmulate {
            watcher: None,
            state: Arc::new(State::default()),
            monitors: Vec::new(),
            motion_rest: (0.0, 0.0),
            scroll_rest: (0.0, 0.0),
            keys: HashSet::new(),
            buttons: HashSet::new(),
            cleanup_pending: false,
            #[cfg(test)]
            backend: tests::Backend::default(),
        }
    }

    /// Before every batch: with the screen locked or the secure desktop up, nothing is injected.
    fn ensure_unlocked(&self) -> Result<(), InputError> {
        if self.state.session_locked.load(Ordering::Relaxed) || !self.desktop_is_default() {
            return Err(InputError::Locked);
        }
        Ok(())
    }

    fn refresh_monitors(&mut self) {
        if self.monitors.is_empty() || self.state.monitors_changed.swap(false, Ordering::AcqRel) {
            #[cfg(test)]
            {
                self.monitors = self.backend.monitors.clone();
            }
            #[cfg(not(test))]
            {
                self.monitors = clipsync_win::monitors().unwrap_or_default();
            }
        }
    }

    fn rects(&self) -> Vec<Rect> {
        self.monitors.iter().map(Rect::of).collect()
    }

    /// Scale of the monitor under `at`; 100% when none.
    fn scale_at(&self, at: (i32, i32)) -> f64 {
        let dpi = self
            .monitors
            .iter()
            .find(|m| {
                let r = Rect::of(m);
                let (x, y) = (i64::from(at.0), i64::from(at.1));
                x >= i64::from(r.x)
                    && x < i64::from(r.x) + i64::from(r.width)
                    && y >= i64::from(r.y)
                    && y < i64::from(r.y) + i64::from(r.height)
            })
            .map_or(0, |m| m.dpi);
        winmath::scale_of(dpi)
    }

    fn desktop_is_default(&self) -> bool {
        #[cfg(test)]
        {
            self.backend.desktop_default
        }
        #[cfg(not(test))]
        {
            clipsync_win::input_desktop_is_default()
        }
    }

    fn cursor_pos(&self) -> Result<(i32, i32), InputError> {
        #[cfg(test)]
        {
            self.backend
                .cursor
                .ok_or_else(|| InputError::Other("cursor indisponível no teste".into()))
        }
        #[cfg(not(test))]
        {
            clipsync_win::cursor_pos().map_err(|e| io_err("GetCursorPos", &e))
        }
    }

    fn virtual_rect(&self) -> Rect {
        #[cfg(test)]
        {
            self.backend.screen
        }
        #[cfg(not(test))]
        {
            let (x, y, w, h) = clipsync_win::virtual_screen();
            Rect {
                x,
                y,
                width: w.max(0) as u32,
                height: h.max(0) as u32,
            }
        }
    }

    fn move_abs(&self, x: i32, y: i32) -> Synth {
        let (nx, ny) = winmath::normalize_abs(self.virtual_rect(), x, y);
        Synth::MoveAbs { nx, ny }
    }

    fn reset_remainders(&mut self) {
        self.motion_rest = (0.0, 0.0);
        self.scroll_rest = (0.0, 0.0);
    }

    fn send(&mut self, batch: &[Synth]) -> Result<(), InputError> {
        if batch.is_empty() {
            return Ok(());
        }
        #[cfg(test)]
        let result = self.backend.send(batch);
        #[cfg(not(test))]
        let result = clipsync_win::send(batch)
            .map(drop)
            .map_err(|e| io_err("SendInput", &e));
        if result.is_err() {
            self.cleanup_pending = true;
            self.reset_remainders();
        }
        result
    }

    /// Smooth scroll in logical px to wheel units, keeping what does not fill a whole unit.
    fn wheel_units(rest: &mut f32, px: f32) -> i32 {
        let total = *rest + px * WHEEL_PER_PX;
        let whole = total.trunc();
        *rest = total - whole;
        whole as i32
    }
}

#[async_trait]
impl EmulateAdapter for WinEmulate {
    fn name(&self) -> &'static str {
        "win-sendinput"
    }

    async fn start(&mut self) -> Result<(), InputError> {
        #[cfg(not(test))]
        {
            let state = Arc::clone(&self.state);
            let watcher = Watcher::start(Box::new(move |notice| match notice {
                Notice::SessionLocked => state.session_locked.store(true, Ordering::Relaxed),
                Notice::SessionUnlocked => state.session_locked.store(false, Ordering::Relaxed),
                Notice::DisplayChanged => state.monitors_changed.store(true, Ordering::Release),
                // The secure desktop is checked before every batch; a lost hook is not this adapter's.
                Notice::DesktopSwitched | Notice::HooksLost => {}
            }))
            .map_err(|e| io_err("Watcher", &e))?;
            self.watcher = Some(watcher);
        }
        self.refresh_monitors();
        Ok(())
    }

    async fn emit(&mut self, events: &[InputEvent]) -> Result<(), InputError> {
        if self.cleanup_pending {
            self.release_all().await?;
        }
        if let Err(error) = self.ensure_unlocked() {
            self.reset_remainders();
            return Err(error);
        }
        self.refresh_monitors();
        let rects = self.rects();
        let mut cursor = match self.cursor_pos() {
            Ok(cursor) => cursor,
            Err(error) => {
                self.reset_remainders();
                return Err(error);
            }
        };
        let mut desired_keys = self.keys.clone();
        let mut desired_buttons = self.buttons.clone();
        let mut possible_keys = self.keys.clone();
        let mut possible_buttons = self.buttons.clone();
        let mut batch = Vec::with_capacity(events.len());
        for event in events {
            match *event {
                InputEvent::Motion { dx, dy } => {
                    let scale = self.scale_at(cursor);
                    let (at, rest) =
                        winmath::emulate_motion(cursor, self.motion_rest, (dx, dy), scale, &rects);
                    self.motion_rest = rest;
                    if at != cursor {
                        cursor = at;
                        batch.push(self.move_abs(at.0, at.1));
                    }
                }
                InputEvent::Button { code, pressed } => {
                    if let Some(button) = keymap::button_to_windows(code) {
                        if pressed {
                            desired_buttons.insert(code);
                            possible_buttons.insert(code);
                        } else {
                            desired_buttons.remove(&code);
                        }
                        batch.push(Synth::Button {
                            button,
                            down: pressed,
                        });
                    }
                }
                InputEvent::Scroll { dx, dy } => {
                    // down is positive in the protocol, up is positive for the wheel
                    let vertical = -Self::wheel_units(&mut self.scroll_rest.1, dy);
                    let horizontal = Self::wheel_units(&mut self.scroll_rest.0, dx);
                    if vertical != 0 {
                        batch.push(Synth::Wheel {
                            delta: vertical,
                            horizontal: false,
                        });
                    }
                    if horizontal != 0 {
                        batch.push(Synth::Wheel {
                            delta: horizontal,
                            horizontal: true,
                        });
                    }
                }
                InputEvent::ScrollDiscrete { dx, dy } => {
                    if dy != 0 {
                        batch.push(Synth::Wheel {
                            delta: -dy,
                            horizontal: false,
                        });
                    }
                    if dx != 0 {
                        batch.push(Synth::Wheel {
                            delta: dx,
                            horizontal: true,
                        });
                    }
                }
                InputEvent::Key { code, pressed } => {
                    // a key with no counterpart on Windows is dropped
                    if let Some(key) = keymap::to_windows(code) {
                        if pressed {
                            desired_keys.insert(code);
                            possible_keys.insert(code);
                        } else {
                            desired_keys.remove(&code);
                        }
                        batch.push(Synth::Key { key, up: !pressed });
                    }
                }
            }
        }
        let sent = self.send(&batch);
        if sent.is_ok() {
            self.keys = desired_keys;
            self.buttons = desired_buttons;
        } else {
            // Não adivinhar qual prefixo foi aceito nem repetir o lote falho.
            self.keys = possible_keys;
            self.buttons = possible_buttons;
        }
        sent
    }

    async fn release_all(&mut self) -> Result<(), InputError> {
        let mut batch = Vec::new();
        for &code in &self.keys {
            if let Some(key) = keymap::to_windows(code) {
                batch.push(Synth::Key { key, up: true });
            }
        }
        for &code in &self.buttons {
            if let Some(button) = keymap::button_to_windows(code) {
                batch.push(Synth::Button {
                    button,
                    down: false,
                });
            }
        }
        self.reset_remainders();
        // Letting go is always allowed, even on a locked screen: it injects nothing that acts.
        self.send(&batch)?;
        self.keys.clear();
        self.buttons.clear();
        self.cleanup_pending = false;
        Ok(())
    }

    async fn enter(
        &mut self,
        zone: &Zone,
        _zones: &[Zone],
        edge: Edge,
        pos: f32,
    ) -> Result<(), InputError> {
        if self.cleanup_pending {
            self.release_all().await?;
        }
        if let Err(error) = self.ensure_unlocked() {
            self.reset_remainders();
            return Err(error);
        }
        let (x, y) = winmath::inside_point(zone, edge, pos);
        self.motion_rest = (0.0, 0.0);
        self.send(&[self.move_abs(x, y)])
    }

    /// The KLID of this machine's keyboard layout, for the diagnostic only.
    fn keyboard_layout(&self) -> Option<String> {
        #[cfg(test)]
        {
            None
        }
        #[cfg(not(test))]
        {
            clipsync_win::keyboard_layout()
        }
    }

    fn cursor(&self) -> Option<(i32, i32)> {
        self.cursor_pos().ok()
    }

    async fn stop(&mut self) -> Result<(), InputError> {
        let released = self.release_all().await;
        self.watcher = None;
        released
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clipsync_win::{WinButton, WinKey};
    use std::collections::VecDeque;

    pub(super) enum Outcome {
        Complete,
        Partial(usize),
    }

    // Instância isolada: nenhum teste deste módulo alcança input, desktop ou watcher reais.
    pub(super) struct Backend {
        pub outcomes: VecDeque<Outcome>,
        pub batches: Vec<Vec<Synth>>,
        pub inserted: Vec<Synth>,
        pub desktop_default: bool,
        pub monitors: Vec<Monitor>,
        pub cursor: Option<(i32, i32)>,
        pub screen: Rect,
    }

    impl Default for Backend {
        fn default() -> Self {
            Self {
                outcomes: VecDeque::new(),
                batches: Vec::new(),
                inserted: Vec::new(),
                desktop_default: true,
                monitors: vec![Monitor {
                    friendly: "Teste".into(),
                    device: "Teste".into(),
                    path: "teste".into(),
                    x: 0,
                    y: 0,
                    width: 1000,
                    height: 1000,
                    dpi: 96,
                    primary: true,
                }],
                cursor: Some((100, 100)),
                screen: Rect {
                    x: 0,
                    y: 0,
                    width: 1000,
                    height: 1000,
                },
            }
        }
    }

    impl Backend {
        pub fn send(&mut self, batch: &[Synth]) -> Result<(), InputError> {
            self.batches.push(batch.to_vec());
            let count = match self.outcomes.pop_front() {
                Some(Outcome::Complete) => batch.len(),
                Some(Outcome::Partial(count)) => count.min(batch.len()),
                None => return Err(InputError::Other("envio não previsto no teste".into())),
            };
            self.inserted.extend_from_slice(&batch[..count]);
            if count == batch.len() {
                Ok(())
            } else {
                Err(InputError::Other("envio parcial simulado".into()))
            }
        }
    }

    fn adapter(outcomes: impl IntoIterator<Item = Outcome>) -> WinEmulate {
        let mut adapter = WinEmulate::new();
        adapter.backend.outcomes.extend(outcomes);
        adapter
    }

    fn key(code: u32, pressed: bool) -> InputEvent {
        InputEvent::Key { code, pressed }
    }

    fn button(code: u32, pressed: bool) -> InputEvent {
        InputEvent::Button { code, pressed }
    }

    fn key_synth(code: u8, up: bool) -> Synth {
        Synth::Key {
            key: WinKey::Scan {
                code,
                extended: false,
            },
            up,
        }
    }

    fn left(down: bool) -> Synth {
        Synth::Button {
            button: WinButton::Left,
            down,
        }
    }

    fn zone() -> Zone {
        Zone {
            index: 0,
            name: Some("Teste".into()),
            x: 0,
            y: 0,
            width: 1000,
            height: 1000,
            scale: 1.0,
        }
    }

    // Remover um up antes da confirmação apagaria os dois códigos anteriores.
    #[tokio::test]
    async fn failed_up_retains_previous_downs() -> Result<(), InputError> {
        for prefix in 0..2 {
            let mut adapter = adapter([Outcome::Complete, Outcome::Partial(prefix)]);
            EmulateAdapter::emit(&mut adapter, &[key(30, true), button(0x110, true)]).await?;
            assert!(
                EmulateAdapter::emit(&mut adapter, &[key(30, false), button(0x110, false)])
                    .await
                    .is_err()
            );
            assert!(adapter.keys == HashSet::from([30]));
            assert!(adapter.buttons == HashSet::from([0x110]));
            assert!(adapter.cleanup_pending);
        }
        Ok(())
    }

    // Mesmo um prefixo que aceitou os ups não autoriza adivinhar o estado de um lote parcial.
    #[tokio::test]
    async fn partial_batch_retains_every_attempted_down() {
        let events = [
            key(30, true),
            key(30, false),
            key(48, true),
            key(48, false),
            button(0x110, true),
            button(0x110, false),
            button(0x111, true),
            button(0x111, false),
        ];
        for prefix in 0..events.len() {
            let mut adapter = adapter([Outcome::Partial(prefix)]);
            assert!(EmulateAdapter::emit(&mut adapter, &events).await.is_err());
            assert!(adapter.keys == HashSet::from([30, 48]));
            assert!(adapter.buttons == HashSet::from([0x110, 0x111]));
            assert!(adapter.cleanup_pending);
            assert_eq!(adapter.backend.batches.len(), 1);
        }
    }

    // A união precisa incluir pendências anteriores que não aparecem neste lote.
    #[tokio::test]
    async fn partial_batch_keeps_old_and_new_codes() -> Result<(), InputError> {
        let mut adapter = adapter([Outcome::Complete, Outcome::Partial(1)]);
        EmulateAdapter::emit(&mut adapter, &[key(30, true), button(0x110, true)]).await?;
        assert!(
            EmulateAdapter::emit(
                &mut adapter,
                &[key(30, false), key(48, true), button(0x111, true)]
            )
            .await
            .is_err()
        );
        assert!(adapter.keys == HashSet::from([30, 48]));
        assert!(adapter.buttons == HashSet::from([0x110, 0x111]));
        assert!(adapter.cleanup_pending);
        Ok(())
    }

    // drain() faria a segunda tentativa esquecer todos os ups.
    #[tokio::test]
    async fn release_retries_all_ups_until_confirmed() -> Result<(), InputError> {
        let mut adapter = adapter([Outcome::Complete, Outcome::Partial(1), Outcome::Complete]);
        EmulateAdapter::emit(
            &mut adapter,
            &[key(30, true), key(48, true), button(0x110, true)],
        )
        .await?;
        assert!(EmulateAdapter::release_all(&mut adapter).await.is_err());
        assert!(adapter.keys == HashSet::from([30, 48]));
        assert!(adapter.buttons == HashSet::from([0x110]));
        assert!(adapter.cleanup_pending);
        EmulateAdapter::release_all(&mut adapter).await?;
        assert!(adapter.keys.is_empty() && adapter.buttons.is_empty());
        assert!(!adapter.cleanup_pending);
        let first = &adapter.backend.batches[1];
        let second = &adapter.backend.batches[2];
        assert_eq!(first.len(), 3);
        assert_eq!(second.len(), 3);
        for event in [key_synth(30, true), key_synth(48, true), left(false)] {
            assert!(first.contains(&event) && second.contains(&event));
        }
        EmulateAdapter::release_all(&mut adapter).await?;
        assert_eq!(adapter.backend.batches.len(), 3);
        Ok(())
    }

    // Sem o gate, um down novo atravessaria a limpeza que ainda falha.
    #[tokio::test]
    async fn emit_waits_for_cleanup() -> Result<(), InputError> {
        let mut adapter = adapter([
            Outcome::Partial(1),
            Outcome::Partial(0),
            Outcome::Complete,
            Outcome::Complete,
        ]);
        assert!(
            EmulateAdapter::emit(&mut adapter, &[key(30, true), key(30, false)])
                .await
                .is_err()
        );
        assert!(
            EmulateAdapter::emit(&mut adapter, &[key(48, true)])
                .await
                .is_err()
        );
        assert_eq!(adapter.backend.batches.len(), 2);
        assert_eq!(adapter.backend.batches[1], vec![key_synth(30, true)]);
        assert!(adapter.keys == HashSet::from([30]));
        assert!(adapter.cleanup_pending);
        EmulateAdapter::emit(&mut adapter, &[key(48, true)]).await?;
        assert_eq!(adapter.backend.batches.len(), 4);
        assert_eq!(adapter.backend.batches[2], vec![key_synth(30, true)]);
        assert_eq!(adapter.backend.batches[3], vec![key_synth(48, false)]);
        assert!(adapter.keys == HashSet::from([48]));
        assert!(!adapter.cleanup_pending);
        Ok(())
    }

    // enter também injeta MoveAbs e não pode contornar a pendência de limpeza.
    #[tokio::test]
    async fn enter_waits_for_cleanup() -> Result<(), InputError> {
        let mut adapter = adapter([
            Outcome::Partial(1),
            Outcome::Partial(0),
            Outcome::Complete,
            Outcome::Complete,
        ]);
        let zone = zone();
        assert!(
            EmulateAdapter::emit(&mut adapter, &[key(30, true), key(30, false)])
                .await
                .is_err()
        );
        assert!(
            EmulateAdapter::enter(&mut adapter, &zone, &[], Edge::Left, 0.5)
                .await
                .is_err()
        );
        assert_eq!(adapter.backend.batches.len(), 2);
        assert_eq!(adapter.backend.batches[1], vec![key_synth(30, true)]);
        assert!(adapter.cleanup_pending);
        EmulateAdapter::enter(&mut adapter, &zone, &[], Edge::Left, 0.5).await?;
        assert_eq!(adapter.backend.batches.len(), 4);
        assert_eq!(adapter.backend.batches[2], vec![key_synth(30, true)]);
        assert!(matches!(
            adapter.backend.batches[3].as_slice(),
            [Synth::MoveAbs { .. }]
        ));
        assert_eq!(
            adapter
                .backend
                .inserted
                .iter()
                .filter(|event| matches!(event, Synth::MoveAbs { .. }))
                .count(),
            1
        );
        assert!(adapter.keys.is_empty() && adapter.buttons.is_empty());
        assert!(!adapter.cleanup_pending);
        Ok(())
    }

    // Em sucesso, a ordem down/up confirma somente os códigos que ficaram pressionados.
    #[tokio::test]
    async fn complete_batch_commits_desired_state_and_order() -> Result<(), InputError> {
        let mut adapter = adapter([Outcome::Complete]);
        EmulateAdapter::emit(
            &mut adapter,
            &[
                key(30, true),
                key(30, false),
                key(48, true),
                button(0x110, true),
                button(0x110, false),
                button(0x111, true),
            ],
        )
        .await?;
        assert!(adapter.keys == HashSet::from([48]));
        assert!(adapter.buttons == HashSet::from([0x111]));
        assert!(!adapter.cleanup_pending);
        assert_eq!(
            adapter.backend.batches[0],
            vec![
                key_synth(30, false),
                key_synth(30, true),
                key_synth(48, false),
                left(true),
                left(false),
                Synth::Button {
                    button: WinButton::Right,
                    down: true
                },
            ]
        );
        Ok(())
    }

    // Restos de um lote falho não podem influenciar a próxima operação após a limpeza.
    #[tokio::test]
    async fn failed_motion_and_scroll_reset_remainders_without_replay() -> Result<(), InputError> {
        let mut adapter = adapter([Outcome::Partial(0), Outcome::Complete]);
        assert!(
            EmulateAdapter::emit(
                &mut adapter,
                &[
                    InputEvent::Motion { dx: 1.25, dy: 0.5 },
                    InputEvent::Scroll { dx: 0.1, dy: 0.1 },
                ]
            )
            .await
            .is_err()
        );
        assert_eq!(adapter.motion_rest, (0.0, 0.0));
        assert_eq!(adapter.scroll_rest, (0.0, 0.0));
        assert!(adapter.cleanup_pending);
        EmulateAdapter::emit(&mut adapter, &[key(48, true)]).await?;
        assert_eq!(adapter.backend.batches.len(), 2);
        assert_eq!(adapter.backend.batches[1], vec![key_synth(48, false)]);
        assert!(!adapter.cleanup_pending);
        Ok(())
    }

    #[tokio::test]
    async fn failed_enter_resets_both_remainders() {
        let mut adapter = adapter([Outcome::Partial(0)]);
        adapter.motion_rest = (0.25, 0.5);
        adapter.scroll_rest = (0.75, 0.125);
        assert!(
            EmulateAdapter::enter(&mut adapter, &zone(), &[], Edge::Left, 0.5)
                .await
                .is_err()
        );
        assert_eq!(adapter.motion_rest, (0.0, 0.0));
        assert_eq!(adapter.scroll_rest, (0.0, 0.0));
        assert!(adapter.cleanup_pending);
    }

    #[tokio::test]
    async fn failed_stop_keeps_pending_release_for_retry() -> Result<(), InputError> {
        let mut adapter = adapter([Outcome::Complete, Outcome::Partial(0), Outcome::Complete]);
        EmulateAdapter::emit(&mut adapter, &[key(30, true)]).await?;
        assert!(EmulateAdapter::stop(&mut adapter).await.is_err());
        assert!(adapter.keys == HashSet::from([30]));
        assert!(adapter.cleanup_pending);
        EmulateAdapter::stop(&mut adapter).await?;
        assert!(adapter.keys.is_empty());
        assert!(!adapter.cleanup_pending);
        assert_eq!(adapter.backend.batches[1], vec![key_synth(30, true)]);
        assert_eq!(adapter.backend.batches[2], vec![key_synth(30, true)]);
        Ok(())
    }

    #[tokio::test]
    async fn locked_desktop_blocks_new_input_but_allows_release() -> Result<(), InputError> {
        let mut adapter = adapter([Outcome::Complete, Outcome::Complete]);
        EmulateAdapter::emit(&mut adapter, &[key(30, true)]).await?;
        adapter.backend.desktop_default = false;
        assert!(matches!(
            EmulateAdapter::emit(&mut adapter, &[key(48, true)]).await,
            Err(InputError::Locked)
        ));
        assert!(matches!(
            EmulateAdapter::enter(&mut adapter, &zone(), &[], Edge::Left, 0.5).await,
            Err(InputError::Locked)
        ));
        assert_eq!(adapter.backend.batches.len(), 1);
        EmulateAdapter::release_all(&mut adapter).await?;
        assert_eq!(adapter.backend.batches[1], vec![key_synth(30, true)]);
        Ok(())
    }
}
