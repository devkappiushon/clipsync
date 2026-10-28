//! Portuguese text of the screens (contract §12.4: `UiError.message` comes ready to show).
//! Owned by Rivet. Codes are stable identifiers and live in `api::code`; text lives here.

pub const BAD_REQUEST: &str = "pedido inválido";
pub const NOT_INITIALIZED: &str = "este dispositivo ainda não foi configurado";
pub const AGENT_NOT_RUNNING: &str = "o agente não está rodando";
pub const AGENT_EXTERNAL: &str =
    "o agente deste perfil está rodando fora do app (clipsync run); pare por lá";
pub const UNAVAILABLE: &str = "indisponível nesta versão";
pub const NO_USABLE_MODULE: &str = "nenhum módulo ligado funciona neste sistema: ligue o clipboard";
pub const AGENT_START_TIMEOUT: &str = "o agente não respondeu ao subir";
pub const AGENT_STOPPED_ITSELF: &str = "o agente parou sozinho";
pub const UNEXPECTED_REPLY: &str = "resposta inesperada do agente";

// 1 · onboarding
pub const INVALID_NAME: &str =
    "nome inválido: use de 1 a 64 bytes, sem caracteres de controle ou invisíveis";
pub const NO_MODULE: &str = "ligue pelo menos um módulo";
pub const RECREATE_CONFIRM: &str = "este dispositivo já está configurado: recriar troca a identidade, e os pares terão de parear de novo";
pub const SELF_TEST_UNSUPPORTED: &str = "sem adaptador nesta plataforma";
pub const SELF_TEST_MODULE_OFF: &str = "módulo desligado";
pub const SELF_TEST_MISSING: &str = "teste automático indisponível nesta versão";

pub fn unknown_command(cmd: &str) -> String {
    format!("comando desconhecido: {cmd}")
}

pub fn module_build(detail: &str) -> String {
    format!("não foi possível preparar o módulo: {detail}")
}

pub fn agent_failed(detail: &str) -> String {
    format!("o agente não subiu: {detail}")
}

pub fn agent_died(detail: &str) -> String {
    format!("o agente parou: {detail}")
}

pub fn io(detail: &str) -> String {
    format!("erro de leitura ou gravação: {detail}")
}

pub fn config(detail: &str) -> String {
    format!("configuração inválida: {detail}")
}

pub fn internal(detail: &str) -> String {
    format!("erro interno: {detail}")
}
