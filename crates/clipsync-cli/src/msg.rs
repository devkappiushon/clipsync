//! All user-facing text (pt-BR). Code, identifiers and comments stay in English.

pub const ERROR_PREFIX: &str = "erro";
pub const AGENT_STOPPED: &str = "agente parado (suba com `clipsync run`)";
pub const NOT_INITIALIZED: &str = "perfil não inicializado (rode `clipsync init`)";

// ---- init ----
pub const INIT_INTRO: &str = "\
Módulos do ClipSync (nada é transmitido agora, e cada permissão é concedida depois, no pareamento):
  • input: este computador pode enviar teclado e mouse a um par, e receber os de um par que você autorizar.
  • clipboard: textos copiados aqui podem ser enviados a pares autorizados, e os deles chegam aqui.";
pub const ASK_NAME: &str = "Nome deste dispositivo";
pub const ASK_CLIPBOARD: &str = "Ligar o módulo de clipboard?";
pub const ASK_RECREATE: &str =
    "Já existe um perfil aqui. Recriar a identidade? Os pares atuais terão de parear de novo.";
pub const INIT_NOT_CONFIRMED: &str = "nada foi alterado";
pub const BAD_MODULE: &str = "módulo desconhecido em --modules (use input e/ou clipboard)";
pub const NO_MODULE: &str = "pelo menos um módulo precisa estar ligado";

pub fn init_done(
    dir: &str,
    id: &str,
    name: &str,
    listen: &str,
    input: bool,
    clipboard: bool,
    fake: bool,
) -> String {
    let on = |b| if b { "ligado" } else { "desligado" };
    format!(
        "Perfil criado em {dir}\n  dispositivo: {name}\n  id: {id}\n  escuta: {listen}\n  input: {} · clipboard: {}{}\nNada foi transmitido.\nPróximo passo: `clipsync pair host` neste computador e `clipsync pair join ENDEREÇO` no outro.",
        on(input),
        on(clipboard),
        if fake {
            "\n  adaptadores: falsos (apenas teste)"
        } else {
            ""
        }
    )
}

// ---- status / pause / resume ----
pub const PAUSED: &str = "pausado: nada é enviado nem aplicado até `clipsync resume`";
pub const RESUMED: &str = "retomado: o que foi copiado durante a pausa não será enviado";
pub const RUNNING: &str = "ativo";
pub const PAUSED_WORD: &str = "PAUSADO";
pub const NO_PEERS: &str = "nenhum par pareado";
pub const MODULES_HEADER: &str = "Módulos:";
pub const PEERS_HEADER: &str = "Pares:";

pub fn status_head(name: &str, id: &str, listen: &str, paused: bool) -> String {
    format!(
        "{name} · id {id} · escuta {listen} · {}",
        if paused { PAUSED_WORD } else { RUNNING }
    )
}
pub fn stopped_head(name: &str, listen: &str, input: bool, clipboard: bool) -> String {
    let on = |b| if b { "ligado" } else { "desligado" };
    format!(
        "{name} · escuta {listen} · input {} · clipboard {} · {AGENT_STOPPED}",
        on(input),
        on(clipboard)
    )
}
pub fn peer_line(
    name: &str,
    id: &str,
    state: &str,
    addr: &str,
    modules: &str,
    perms: &str,
) -> String {
    format!("  {name} ({id}) · {state} · {addr} · módulos do par: {modules} · permissões: {perms}")
}
pub fn module_line(
    kind: &str,
    enabled: bool,
    adapter: &str,
    last_event: &str,
    last_block: &str,
) -> String {
    format!(
        "  {kind}: {} · adaptador {adapter} · último evento: {last_event} · último bloqueio: {last_block}",
        if enabled { "ligado" } else { "desligado" }
    )
}
pub const NONE: &str = "nenhum";
pub const CONNECTED: &str = "conectado";
pub const CONNECTING: &str = "conectando";
pub const DISCONNECTED: &str = "desconectado";
pub fn blocked(reason: &str) -> String {
    format!("bloqueado ({reason})")
}

// ---- pair ----
pub const ASK_CODE: &str = "O código acima é o mesmo no outro computador?";
pub const ASK_CLIPBOARD_IN: &str =
    "Permitir que este par ENVIE textos copiados para cá (clipboard-in)?";
pub const ASK_CLIPBOARD_OUT: &str =
    "Permitir que este computador ENVIE o clipboard para o par (clipboard-out)?";
pub const ASK_INPUT_IN: &str = "Permitir que este par CONTROLE o teclado e o mouse deste computador (input-in)? É confiança total: quem controla pode digitar e clicar como você.";
pub const BAD_ALLOW: &str =
    "permissão desconhecida em --allow (use clipboard-in, clipboard-out, input-in)";
pub const BAD_ADDR: &str = "endereço inválido (use IP:PORTA, por exemplo 127.0.0.1:47801)";
pub const PAIR_DECLINED_LOCAL: &str = "pareamento não confirmado: nada foi gravado";
pub const PAIR_DECLINED_PEER: &str = "o outro lado não confirmou: nada foi gravado";
pub fn pair_waiting(addr: &str, secs: u64) -> String {
    format!("Aguardando em {addr} (expira em {})", humanize(secs))
}
fn humanize(secs: u64) -> String {
    if secs.is_multiple_of(60) {
        format!("{} min", secs / 60)
    } else {
        format!("{secs} s")
    }
}
pub fn pair_prompt(code: &str, name: &str, id: &str, modules: &str) -> String {
    format!("Código: {code}\nPar: {name} (id {id}) · módulos: {modules}")
}
pub fn paired(name: &str, id: &str, perms: &str) -> String {
    format!("Pareado com {name} (id {id}). Permissões concedidas por você: {perms}")
}
pub fn pair_failed(detail: &str) -> String {
    format!("pareamento falhou: {detail}")
}

// ---- run ----
pub fn run_started(name: &str, listen: &str, modules: &str) -> String {
    format!("Agente de {name} no ar, escutando em {listen} · módulos: {modules}. Ctrl-C encerra.")
}
pub const RUN_STOPPED: &str = "agente encerrado";
pub const ALREADY_RUNNING: &str = "já existe um agente rodando neste perfil";

// ---- send / inbox ----
pub const EMPTY_STDIN: &str = "nada para enviar: a entrada está vazia";
pub const STDIN_TOO_BIG: &str = "texto acima do limite de 1 MiB";
pub const STDIN_NOT_UTF8: &str = "a entrada não é texto UTF-8";
pub const UNKNOWN_PEER: &str = "par desconhecido (veja `clipsync status`)";
pub const INBOX_EMPTY: &str = "caixa de recebidos vazia";
pub fn delivery(peer: &str, state: &str) -> String {
    format!("{peer}: {state}")
}
pub fn inbox_line(
    id: &str,
    from: &str,
    time: &str,
    bytes: usize,
    state: &str,
    preview: &str,
) -> String {
    format!("{id} · de {from} · {time} · bytes={bytes} · {state} · {preview}")
}
pub fn unexpected_reply(what: &str) -> String {
    format!("resposta inesperada do agente: {what}")
}

// ---- diagnose / probe ----
pub const PROBE_PEER_REQUIRED: &str = "--loopback exige --to PAR";
pub fn diag_line(status: &str, area: &str, name: &str, detail: &str) -> String {
    format!("[{status}] {area} · {name}: {detail}")
}
