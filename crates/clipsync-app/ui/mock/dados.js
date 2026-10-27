// Dados compartilhados entre os domínios do mock: esta máquina e os três pares.
// Os ids seguem o contrato (peer_id = 64 hex; short = 8 hex em dois grupos de 4).

import { cenario, estado, plataforma } from "./nucleo.js";

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

export const EU = { peer_id: peerId("f3d7-ea9a"), short: "f3d7-ea9a", name: "notebook-a" };

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
    remote_version: "0.2.0",
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
estado.pausado = false;
estado.telaTravada = false;
estado.dispositivos = dispositivosPadrao();

const CAPACIDADES_INPUT = plataforma === "linux";

export function features() {
  return {
    input: CAPACIDADES_INPUT,
    peer_monitors: CAPACIDADES_INPUT,
    layout_links: CAPACIDADES_INPUT,
    dead_corners: CAPACIDADES_INPUT,
    screen_lock: CAPACIDADES_INPUT,
    switch_device: CAPACIDADES_INPUT,
    layout_copy: CAPACIDADES_INPUT,
    os_notifications: true,
    global_shortcuts: false,
    peer_pause: false,
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
  const conectados = estado.dispositivos.filter((d) => d.state === "connected").length;
  return { connected: conectados, total: estado.dispositivos.length };
}
