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
pub const RECREATE_CONFIRM: &str = "este dispositivo já está configurado: recriar troca a identidade e apaga a lista de dispositivos pareados, e os pares terão de parear de novo";
pub const SELF_TEST_UNSUPPORTED: &str = "sem adaptador nesta plataforma";
pub const SELF_TEST_MODULE_OFF: &str = "módulo desligado";
pub const SELF_TEST_MISSING: &str = "teste automático indisponível nesta versão";

// 2 · dispositivos
pub const UNKNOWN_PEER: &str = "dispositivo não encontrado entre os pareados";
pub const MODULE_OFF: &str = "o módulo dessa permissão está desligado neste dispositivo";
pub const FULL_TRUST: &str = "permitir que esse dispositivo controle o teclado e o mouse daqui é confiança total: confirme para continuar";
pub const NAME_MISMATCH: &str = "o nome digitado não confere com o nome do dispositivo";

// 3 · pareamento
pub const NO_ATTEMPT: &str = "nenhum pareamento em andamento com esse número";
pub const WRONG_STEP: &str = "essa ação não vale nesta etapa do pareamento";
pub const PAIR_BUSY: &str = "já existe um pareamento em andamento: conclua ou cancele antes";
pub const BAD_ADDR: &str = "endereço inválido: use IP:PORTA, por exemplo 192.168.0.10:53122";
pub const PAIR_DECLINED_PEER: &str = "o outro dispositivo recusou o pareamento";
pub const PAIR_NO_DECISION: &str = "sem decisão em 2 minutos: pareamento recusado";
pub const PAIR_EXPIRED: &str = "nenhum dispositivo se conectou dentro do prazo";
pub const PAIR_NO_ANSWER: &str = "o outro dispositivo não respondeu: confira o endereço e a porta";
pub const PAIR_PEER_CLOSED: &str = "o outro dispositivo encerrou a tentativa";
pub const PAIR_TIMEOUT: &str = "o outro dispositivo não respondeu a tempo";

// notificações do sistema: fatos, nunca conteúdo de clipboard (§12.2 regra 7)
pub const NOTIFY_CONNECTED: &str = "Dispositivo conectado";
pub const NOTIFY_BLOCK: &str = "Bloqueio";

pub fn notify_connected(name: &str) -> String {
    format!("{name} está conectado")
}

pub fn notify_block(name: Option<&str>, reason: &str) -> String {
    match name {
        Some(n) => format!("{n}: {reason}"),
        None => reason.to_string(),
    }
}

pub const NOTIFY_REMOVED: &str = "Dispositivo removido";

pub fn notify_removed(name: &str) -> String {
    format!("{name} removeu este dispositivo")
}

// 5 · clipboard
pub const CLIPBOARD_OFF: &str = "o módulo clipboard está desligado neste dispositivo";
pub const NOTIFY_ITEM: &str = "Item recebido";

pub fn notify_item(from: &str, bytes: usize) -> String {
    format!("{from} enviou {bytes} bytes para a caixa de recebidos")
}

pub fn apply_failed(why: &str) -> String {
    format!("não foi possível escrever no clipboard: {why}")
}

// 6 · diagnóstico
pub const DIAG_CHECK: &str = "verificação";
pub const DIAG_TIMEOUT: &str = "não respondeu em 5 s";
pub const NO_DOWNLOADS: &str = "não foi possível achar a pasta de downloads";
pub const REDACTED_IP: &str = "<ip>";
pub const REDACTED_DEVICE: &str = "<este-dispositivo>";

pub fn redacted_peer(n: usize) -> String {
    format!("<par-{n}>")
}

pub fn redacted_id(n: usize) -> String {
    format!("<id-par-{n}>")
}

pub fn bind(detail: &str) -> String {
    format!("não foi possível abrir a porta do pareamento: {detail}")
}

pub fn pair_failed(detail: &str) -> String {
    format!("o pareamento falhou: {detail}")
}

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
