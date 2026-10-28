//! User-facing text (pt-BR). Code, identifiers and comments stay in English.
use clipsync_proto::RejectReason;
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

pub fn layout_parse(detail: &str) -> String {
    format!("layout.toml inválido: {detail}")
}
pub fn layout_io(detail: &str) -> String {
    format!("não foi possível ler ou gravar o layout.toml: {detail}")
}
pub fn layout_unsupported_version(v: u32) -> String {
    format!("layout.toml: versão {v} não suportada (esperado 1)")
}
pub fn layout_invalid_field(detail: &str) -> String {
    format!("layout.toml: {detail}")
}
pub fn layout_unknown_device(link: &str) -> String {
    format!("ligação {link}: dispositivo que não está na lista de dispositivos do layout")
}
pub fn layout_duplicate_id(id: &str) -> String {
    format!("layout.toml: id de ligação repetido ({id})")
}
pub fn layout_same_device(link: &str) -> String {
    format!(
        "ligação {link}: os dois lados estão no mesmo dispositivo; bordas entre monitores da mesma máquina não cruzam"
    )
}
pub fn layout_empty_segment(link: &str) -> String {
    format!("ligação {link}: trecho vazio (o início deve ser menor que o fim)")
}
pub fn layout_out_of_range(link: &str) -> String {
    format!("ligação {link}: trecho fora do intervalo de 0 a 1")
}
pub fn layout_overlap(link: &str, other: &str) -> String {
    format!("ligação {link}: o trecho se sobrepõe ao da ligação {other} na mesma borda")
}
pub const LAYOUT_MANUAL_CONFLICT: &str = "o layout.toml tem ligações e o config.toml também tem [[input.barrier]]: remova um dos dois, os dois juntos são ambíguos";
pub fn layout_bad_peer(value: &str) -> String {
    format!("id de dispositivo inválido (esperado 64 hex): {value}")
}
pub fn layout_bad_name(peer: &str) -> String {
    format!("dispositivo {peer}: o nome deve ter de 1 a 64 bytes")
}
pub fn layout_dup_device(peer: &str) -> String {
    format!("dispositivo {peer} repetido")
}
pub fn layout_bad_hotkey(peer: &str) -> String {
    format!("dispositivo {peer}: o atalho deve ficar entre 1 e 9")
}
pub fn layout_dup_hotkey(n: u8) -> String {
    format!("o atalho {n} está em mais de um dispositivo")
}
pub fn layout_bad_position(peer: &str, max: i32) -> String {
    format!("dispositivo {peer}: a posição deve ficar entre -{max} e {max} px")
}
pub fn layout_bad_dead_corner(max: u32) -> String {
    format!("canto morto fora do intervalo de 0 a {max} px")
}
pub fn layout_bad_link_field(link: &str) -> String {
    format!("ligação {link}: id e monitor não podem ser vazios")
}

pub const LOCKED: &str = "travado nesta tela: a troca de dispositivo foi ignorada";
pub const NOT_LINKED: &str = "não há ligação de layout com esse dispositivo";
pub const NOT_AUTHORIZED: &str = "sem permissão para controlar esse dispositivo";
pub const REMOTE_INPUT_OFF: &str = "teclado e mouse desligados no outro dispositivo";
pub fn send_failed(detail: &str) -> String {
    format!("falha ao enviar ao par: {detail}")
}
pub fn capture_ready(n: usize) -> String {
    format!("{n} barreira(s) instalada(s) a partir do layout.toml")
}
pub fn returned_from(peer: &str) -> String {
    format!("controle de {peer} encerrado: o cursor voltou")
}
pub fn layout_monitor_missing(link: &str, monitor: &str) -> String {
    format!("ligação {link}: monitor {monitor} não encontrado neste dispositivo")
}
pub fn layout_segment_in_dead_corner(link: &str) -> String {
    format!("ligação {link}: o trecho fica inteiro dentro do canto morto")
}
pub const PEER_TOO_OLD: &str =
    "o outro dispositivo roda uma versão antiga do ClipSync e não entende o layout de telas";
/// What the source shows when the destination refuses a session (§13.6 item 9).
pub fn refused_by(peer: &str, reason: RejectReason) -> String {
    match reason {
        RejectReason::Locked => {
            format!("{peer} está com a tela bloqueada: o controle voltou para esta máquina")
        }
        RejectReason::Paused => {
            format!("{peer} está pausado: o controle voltou para esta máquina")
        }
        _ => format!("{peer} não deixou emular teclado e mouse"),
    }
}
pub const SOURCE_SCREEN_LOCKED: &str = "a tela desta máquina está bloqueada: o controle de outras máquinas fica parado até desbloquear";
pub const RESUME_HOTKEY_REFUSED: &str =
    "o sistema não deixou registrar Win+Alt+P: retome pela bandeja ou pelo botão Pausar tudo";
pub fn no_device_for_hotkey(n: u8) -> String {
    format!("nenhum dispositivo do layout tem o atalho {n}")
}
pub const LOCKED_ON: &str = "travado nesta tela";
pub const LOCKED_OFF: &str = "trava desligada: as bordas voltam a trocar de dispositivo";
pub const NO_LAYOUT_PATH: &str = "este agente não lê layout.toml (adaptadores de teste)";
pub const LAYOUT_RELOADED: &str = "layout.toml relido e barreiras reinstaladas";
pub const OFFER_BUSY: &str = "já existe um envio de layout em andamento";
pub const NO_LAYOUT: &str = "não há layout.toml para enviar";
pub fn zones_from(peer: &str) -> String {
    format!("{peer} informou seus monitores")
}
pub fn offer_from(peer: &str) -> String {
    format!("{peer} ofereceu um layout de telas")
}

// self_test (L7): what the onboarding shows for the input module.
pub const SELF_TEST_NO_PLATFORM: &str = "teclado e mouse: indisponível neste sistema nesta versão";
pub const SELF_TEST_TIMEOUT: &str = "os portais do sistema não responderam em 5 segundos";
pub const SELF_TEST_NO_DEVICES: &str =
    "o portal de emulação não oferece teclado e ponteiro juntos, os dois são necessários";
pub const SELF_TEST_NO_MONITORS: &str =
    "nenhum monitor detectado: os monitores podem aparecer só como #0, #1";
pub const SELF_TEST_NOT_GNOME_WAYLAND: &str =
    "só o GNOME no Wayland foi testado, este desktop pode se comportar de outro jeito";
pub const SELF_TEST_KEYBOARD_UNPROVEN: &str =
    "a digitação com acentos ainda não foi validada em máquina real";
pub fn self_test_no_capture(why: &str) -> String {
    format!("o portal de captura de entrada não respondeu: {why}")
}
pub fn self_test_no_remote(why: &str) -> String {
    format!("o portal de emulação de entrada não respondeu: {why}")
}
pub fn self_test_ready(capture: u32, remote: u32, monitors: usize) -> String {
    format!(
        "portais de captura (v{capture}) e de emulação (v{remote}) respondem, {monitors} monitor(es) detectado(s), o primeiro uso pede uma permissão do sistema"
    )
}
pub fn self_test_conflicts(found: &[String]) -> String {
    format!(
        "em execução: {}, podem disputar a captura",
        found.join(", ")
    )
}
