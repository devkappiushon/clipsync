//! User-facing text (pt-BR). Code, identifiers and comments stay in English.
pub const LOOPBACK_BUSY: &str = "já existe um teste de entrada em andamento";
pub const UNSUPPORTED_COMMAND: &str = "comando não suportado pelo módulo de entrada";
pub const PEER_UNKNOWN: &str = "par desconhecido ou sem permissão";
pub const PEER_OFFLINE: &str = "par não conectado";
pub const NO_ACTIVATION: &str = "a barreira não foi ativada no tempo do teste";
pub const NO_STATS: &str = "o destino não respondeu com as estatísticas";
pub const EMULATOR_FAILED: &str = "o emulador de entrada não iniciou neste dispositivo";
pub const PORTAL_DENIED: &str = "permissão do portal negada";
pub const UNAVAILABLE_ON_PLATFORM: &str =
    "teclado e mouse: indisponível no Windows nesta versão (adaptador Win32 previsto para 2B)";
pub const NO_ZONE: &str = "nenhum monitor encontrado";
pub const NOTHING_TO_DO: &str = "nada a fazer: use --capture, --emulate ou --loopback";
pub const LOOPBACK_IS_CLI: &str =
    "--loopback é executado pelo agente (clipsync probe input --loopback --to <par>)";
pub const LOOPBACK_NEEDS_TO: &str = "--loopback exige --to <par>";
pub const CONFIRM_EMULATE: &str =
    "Isto move o cursor e digita \"ção\" na janela focada. Continuar? [s/N] ";
pub const ABORTED: &str = "cancelado";
pub const FOCUS_NOW: &str = "foque a janela de teste agora: começa em 5 s";
pub const CAPTURE_HINT: &str =
    "encoste o mouse na metade superior da borda direita; Esc ou o tempo devolve o cursor";

pub fn denied(detail: &str) -> String {
    format!("{PORTAL_DENIED}: {detail}")
}
pub fn adapter_failed(detail: &str) -> String {
    format!("adaptador de entrada falhou: {detail}")
}
pub fn session_closed_by(peer: &str) -> String {
    format!("controle remoto encerrado por {peer}")
}
pub fn control_started(peer: &str) -> String {
    format!("controle remoto iniciado por {peer}")
}
pub fn ignored_count(n: u64) -> String {
    format!("{n} mensagem(ns) ignorada(s) por sessão desconhecida")
}
pub fn zone_line(i: u32, name: &str, w: u32, h: u32, x: i32, y: i32) -> String {
    format!("zona {i}: {name} {w}x{h} em ({x},{y})")
}
pub fn barrier_line(x1: i32, y1: i32, x2: i32, y2: i32) -> String {
    format!("barreira: ({x1},{y1}) até ({x2},{y2})")
}
