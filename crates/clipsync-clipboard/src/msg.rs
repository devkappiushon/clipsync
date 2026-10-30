//! User-facing text (pt-BR). Code, identifiers and comments stay in English.
pub const NO_DISPLAY: &str = "a ponte de clipboard exige XWayland ativo (DISPLAY)";
pub const X11_ONLY_LINUX: &str = "o adaptador x11 só existe no Linux; use adapter = \"auto\"";
pub const NO_ADAPTER: &str = "não há adaptador de clipboard para este sistema";
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
// Logical clock policy (contrato §15.11); never includes clipboard content.
pub const CLOCK_EXHAUSTED: &str =
    "relógio lógico do clipboard esgotado; nenhuma oferta foi enviada";
pub const CLOCK_AHEAD: &str =
    "relógio remoto inválido: a janela lógica permite até 1000000 passos de avanço";
pub const NO_TARGET: &str = "nenhum par autorizado a receber (ou par desconhecido)";
pub const EMPTY_TEXT: &str = "texto vazio não é enviado";
pub const TOO_LARGE: &str = "texto acima do limite configurado";
pub const UNSUPPORTED_COMMAND: &str = "comando não suportado pelo módulo clipboard";
// Caixa de recebidos e envio explícito (contrato §12.6): o crate da UI mapeia por estas consts.
pub const NO_ITEM: &str = "item não encontrado na caixa de recebidos";
pub const WRONG_STATE: &str = "só um item na caixa ou antigo, ainda com texto, pode ser aplicado";
pub const EMPTY_CLIPBOARD: &str = "o clipboard está vazio";
pub fn applied_from_inbox(peer: &str) -> String {
    format!("aplicado da caixa: texto de {peer}")
}
pub const APPLIED: &str = "aplicado";
pub const IN_INBOX: &str = "na caixa de recebidos";
pub const STALE: &str = "antigo, só na caixa";
pub const REFUSED: &str = "recusado";

// Autoteste (contrato §12.6 clipboard item 5): o detalhe vai como está para a tela de onboarding.
// Nunca leva conteúdo do clipboard.
pub const SELF_TEST_PANIC: &str = "o autoteste do clipboard falhou por um erro interno";
pub fn self_test_open_failed(detail: &str) -> String {
    format!("não foi possível abrir o clipboard: {detail}")
}
pub fn self_test_read_failed(detail: &str) -> String {
    format!("a leitura do clipboard falhou: {detail}")
}
pub fn self_test_write_failed(detail: &str) -> String {
    format!("a escrita no clipboard falhou: {detail}")
}
pub const SELF_TEST_MISMATCH: &str =
    "o texto lido do clipboard não confere com o texto de teste escrito";
fn restored_note(had_text: bool) -> &'static str {
    if had_text {
        "o conteúdo anterior foi devolvido"
    } else {
        "o clipboard estava vazio e voltou a ficar vazio"
    }
}
pub fn self_test_validated(had_text: bool, latency_ms: u128) -> String {
    format!(
        "escrita, leitura e aviso de mudança conferem em {latency_ms} ms; {}",
        restored_note(had_text)
    )
}
pub fn self_test_experimental(adapter: &str, had_text: bool, latency_ms: u128) -> String {
    format!(
        "escrita, leitura e aviso de mudança conferem em {latency_ms} ms, mas o adaptador {adapter} \
         ainda não foi validado nesta plataforma (só texto, mudanças detectadas por polling); {}",
        restored_note(had_text)
    )
}
pub fn self_test_no_event(waited_ms: u128, had_text: bool) -> String {
    format!(
        "escrita e leitura conferem, mas o aviso de mudança não chegou em {waited_ms} ms: cópias \
         feitas fora do ClipSync podem não ser detectadas; {}",
        restored_note(had_text)
    )
}
pub const SELF_TEST_RESTORE_FAILED: &str = "escrita, leitura e aviso de mudança conferem, mas não foi possível devolver o conteúdo anterior ao clipboard";
