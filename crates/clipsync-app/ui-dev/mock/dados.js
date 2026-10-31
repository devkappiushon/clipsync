// Dados compartilhados entre os domínios do mock: esta máquina e os três pares.
// Os ids seguem o contrato (peer_id = 64 hex; short = 8 hex em dois grupos de 4).

import { agenteDePe, cenario, estado, plataforma, CENARIOS_MONITORES } from "./nucleo.js";

const AGORA = Date.now();
const MIN = 60_000;
const DIA = 24 * 60 * MIN;

/** 64 hex começando pelo `short` (sem o hífen), completado de forma determinística. */
export function peerId(short) {
  const base = short.replace("-", "");
  let id = base;
  while (id.length < 64) id += ((id.charCodeAt(id.length - 3) * 31 + id.length * 7) % 16).toString(16);
  return id;
}

export const EU = { peer_id: peerId("f3d7-ea9a"), short: "f3d7-ea9a", name: plataforma === "windows" ? "desktop-win" : "notebook-a" };

function par({ short, name, addr, state, blocked_reason = null, perms, remote_modules, dias }) {
  return {
    peer_id: peerId(short),
    short,
    name,
    state,
    blocked_reason,
    addr,
    perms,
    remote_modules,
    remote_version: CENARIOS_MONITORES.has(cenario) && cenario !== "protocolo-v3" ? 4 : 3,
    paired_at: AGORA - dias * DIA,
  };
}

function dispositivosPadrao() {
  const pessoal = par({
    short: "aa31-537a",
    name: "pc-pessoal",
    addr: "192.168.115.30:47800",
    state: "connected",
    perms: { clipboard_in: true, clipboard_out: true, input_in: true },
    remote_modules: { clipboard: true, input: true },
    dias: 12,
  });
  const casa = par({
    short: "1bc6-7a55",
    name: "pc-de-casa",
    addr: "192.168.115.41:47800",
    state: "connected",
    perms: { clipboard_in: true, clipboard_out: false, input_in: false },
    remote_modules: { clipboard: true, input: true },
    dias: 9,
  });
  const antigo = par({
    short: "817a-ab9c",
    name: "notebook-antigo",
    addr: "192.168.115.55:47800",
    state: "connected",
    perms: { clipboard_in: false, clipboard_out: false, input_in: false },
    remote_modules: { clipboard: true, input: false },
    dias: 40,
  });
  if (cenario === "protocolo-antigo") pessoal.remote_version = 2;
  if (cenario === "estados") {
    casa.state = "offline";
    antigo.state = "blocked";
    antigo.blocked_reason = "A chave do par mudou desde o pareamento. Pareie de novo para confiar nela.";
  }
  if (cenario === "vazio" || cenario === "sem-perfil") return [];
  return [pessoal, casa, antigo];
}

estado.eu = EU;
estado.inicializado = cenario !== "sem-perfil";
estado.agente = cenario === "erro" ? "failed" : cenario === "sem-perfil" ? "stopped" : "in_app";
estado.erroAgente = cenario === "erro" ? "Não foi possível abrir a porta 47800: já está em uso." : null;
// Windows que recusou o Win+Alt+P de retomar (I-S7-2): o app abre pausado, que é quando o aviso importa.
// `retomadaRecusada` é o features.resume_hotkey_refused (liga ao pausar, desliga ao retomar); `bloqueioRecusa`
// é o bloqueio de input com o texto, que continua no Diagnóstico.
estado.retomadaRecusada = cenario === "windows-recusa";
estado.bloqueioRecusa = estado.retomadaRecusada;
estado.pausado = estado.retomadaRecusada;
estado.telaTravada = false;
estado.modulos = { clipboard: true, input: cenario !== "input-desligado" };
estado.dispositivos = dispositivosPadrao();

// S7: o input existe no Linux e no Windows (clipsync_input::SUPPORTED).
const CAPACIDADES_INPUT = true;

export function features() {
  const inputDisponivel = CAPACIDADES_INPUT && estado.modulos.input && agenteDePe();
  return {
    input: inputDisponivel,
    peer_monitors: inputDisponivel,
    layout_links: inputDisponivel,
    dead_corners: inputDisponivel,
    screen_lock: inputDisponivel,
    // A troca por atalho ou botão precisa do can_begin() do adaptador: o portal do Linux não deixa (false), o Windows deixa.
    switch_device: inputDisponivel && plataforma === "windows",
    layout_copy: inputDisponivel,
    os_notifications: true,
    // Windows com o módulo de input ligado: Win+Alt+N, L e P. No Linux não há tecla global sem o portal.
    global_shortcuts: plataforma === "windows" && inputDisponivel,
    peer_pause: false,
    resume_hotkey_refused: inputDisponivel && estado.retomadaRecusada === true,
    peer_controls_column: false,
    peer_rtt: false,
    peer_last_activity: false,
    peer_addr_history: false,
    clipboard_confirm_mode: false,
    clipboard_pause_capture: false,
    clipboard_private_mode: false,
    pair_peer_declined_notice: false,
  };
}

export function malha() {
  const conectados = agenteDePe() ? estado.dispositivos.filter((d) => d.state === "connected").length : 0;
  return { connected: conectados, total: estado.dispositivos.length };
}
