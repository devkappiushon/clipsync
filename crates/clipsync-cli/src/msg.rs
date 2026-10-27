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
pub const ASK_RECREATE: &str = "Já existe um perfil aqui. Recriar a identidade? Os pares atuais terão de parear de novo.";
pub const INIT_NOT_CONFIRMED: &str = "nada foi alterado";
pub const BAD_MODULE: &str = "módulo desconhecido em --modules (use input e/ou clipboard)";
pub const NO_MODULE: &str = "pelo menos um módulo precisa estar ligado";

pub fn init_done(dir: &str, id: &str, name: &str, listen: &str, input: bool, clipboard: bool, fake: bool) -> String {
    let on = |b| if b { "ligado" } else { "desligado" };
    format!(
        "Perfil criado em {dir}\n  dispositivo: {name}\n  id: {id}\n  escuta: {listen}\n  input: {} · clipboard: {}{}\nNada foi transmitido.\nPróximo passo: `clipsync pair host` neste computador e `clipsync pair join ENDEREÇO` no outro.",
        on(input),
        on(clipboard),
        if fake { "\n  adaptadores: falsos (apenas teste)" } else { "" }
    )
}
