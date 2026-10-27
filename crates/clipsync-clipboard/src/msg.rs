//! User-facing text (pt-BR). Code, identifiers and comments stay in English.
pub const NO_DISPLAY: &str = "a ponte de clipboard exige XWayland ativo (DISPLAY)";
pub const BRIDGE_ITEM: &str = "ponte XWayland (DISPLAY)";
pub const WATCH_HEADER: &str = "observação: eventos (XFixes via XWayland)";
pub const NOTHING_TO_DO: &str = "nada a fazer: use --watch <duração> e/ou --write";
pub const EMPTY_IGNORED: &str = "vazio (ignorado, não sincroniza)";

pub fn watch_line(mime: &str, bytes: usize, latency_ms: u128) -> String {
    format!("mime={mime} bytes={bytes} latência_leitura={latency_ms} ms")
}
pub fn write_line(wrote: bool, identical: bool, echo_dropped: bool, new_copy: bool) -> String {
    let yn = |b| if b { "ok" } else { "falha" };
    format!(
        "escrita: {} · leitura idêntica: {} · evento da própria escrita: {} · nova cópia disparada: {}",
        yn(wrote),
        yn(identical),
        if echo_dropped {
            "descartado (eco)"
        } else {
            "NÃO descartado"
        },
        if new_copy { "sim" } else { "não" },
    )
}
pub fn sent_to(n: usize) -> String {
    format!("oferta enviada a {n} par(es)")
}
pub fn received_from(peer: &str, outcome: &str) -> String {
    format!("texto recebido de {peer}: {outcome}")
}
pub fn blocked_too_large(bytes: usize, max: usize) -> String {
    format!("cópia local ignorada: {bytes} bytes acima do limite de {max}")
}
pub fn adapter_read_failed(detail: &str) -> String {
    format!("leitura do clipboard falhou: {detail}")
}
pub const NO_PEER_REACHED: &str = "nenhum par alcançado (desconectado ou sem permissão de envio)";
pub const EMPTY_TEXT: &str = "texto vazio não é enviado";
pub const TOO_LARGE: &str = "texto acima do limite configurado";
pub const UNSUPPORTED_COMMAND: &str = "comando não suportado pelo módulo clipboard";
pub const APPLIED: &str = "aplicado";
pub const IN_INBOX: &str = "na caixa de recebidos";
pub const STALE: &str = "antigo, só na caixa";
pub const REFUSED: &str = "recusado";
