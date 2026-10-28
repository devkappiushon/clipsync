//! The Windows adapters (contract §13.6, "Adaptador Windows"): `WinCapture` and `WinEmulate` over
//! `clipsync-win`, and the `self_test` the onboarding shows. Every decision they make lives in
//! `winmath` and `wincore`, which test on every system; this module only calls the system.
mod capture;
mod emulate;

pub use capture::WinCapture;
pub use emulate::WinEmulate;

use crate::SelfTestOutcome;
use clipsync_win::{Hooks, Verdict};

/// The capture stopped for good after the hooks died too many times (no key, no code in the text).
pub(super) const HOOKS_GAVE_UP: &str = "Os ganchos de teclado e mouse deste computador pararam de responder ao ClipSync várias vezes em poucos segundos: o controle foi desligado. Reinicie o módulo de input para voltar a usar.";

/// Brings the hooks up and down, reads the monitors and checks the desktop. It reads no key (the
/// callback passes everything without looking), moves no cursor and presses nothing. The result is
/// `Experimental` until the compatibility matrix records the roteiro on a real machine.
pub fn self_test() -> SelfTestOutcome {
    let monitors = match clipsync_win::monitors() {
        Ok(monitors) => monitors,
        Err(e) => {
            return SelfTestOutcome::Unavailable(format!(
                "Não foi possível ler os monitores: {e}."
            ));
        }
    };
    if monitors.is_empty() {
        return SelfTestOutcome::Unavailable("Nenhum monitor foi detectado nesta sessão.".into());
    }
    if !clipsync_win::input_desktop_is_default() {
        return SelfTestOutcome::Unavailable(
            "A área de trabalho que recebe a entrada é protegida (tela bloqueada ou aviso do sistema).".into(),
        );
    }
    match Hooks::start(Box::new(|_| Verdict::Pass), Box::new(|_| {})) {
        Ok(hooks) => drop(hooks),
        Err(e) => {
            return SelfTestOutcome::Unavailable(format!(
                "Não foi possível instalar os ganchos de teclado e mouse: {e}."
            ));
        }
    }
    SelfTestOutcome::Experimental(
        "captura e emulação do Windows prontas, ainda sem prova nesta máquina".into(),
    )
}
