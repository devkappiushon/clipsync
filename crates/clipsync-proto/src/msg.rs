//! User-facing Portuguese text owned by clipsync-proto.

use crate::wire::RejectReason;

/// Reply of an agent whose module is disabled: `ModuleReply::Error(MODULE_DISABLED)`.
pub const MODULE_DISABLED: &str = "módulo desligado neste dispositivo";

impl RejectReason {
    pub fn pt(self) -> &'static str {
        match self {
            RejectReason::ModuleDisabled => "módulo desligado",
            RejectReason::NotAuthorized => "sem permissão",
            RejectReason::Paused => "pausado",
            RejectReason::TooLarge => "grande demais",
            RejectReason::Invalid => "mensagem inválida",
            RejectReason::Duplicate => "duplicado",
            RejectReason::Stale => "mais antigo que o conteúdo atual",
            RejectReason::Expired => "expirado",
            RejectReason::ApplyFailed => "falha ao aplicar",
            RejectReason::Locked => "bloqueado",
            RejectReason::VersionMismatch => "versão incompatível",
        }
    }
}
