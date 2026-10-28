// Mock do domínio pareamento: uma tentativa por vez, com temporizadores que imitam o par.
// pair_host_start -> listening -> (par chega) code -> permissions -> waiting_peer -> pair_result.
// ?cenario=pareamento começa já no passo "code", para capturar a tela sem esperar.

import { peerId } from "./dados.js";
import { cenario, emitir, erro, estado, registrar } from "./nucleo.js";

const JANELA_PADRAO_S = 120;
const CHEGADA_MS = 2500;
const RESPOSTA_PAR_MS = 1800;

const PAR_NOVO = { name: "pc-escritorio", short: "c04e-9d12", modules: { clipboard: true, input: true } };
const ENDERECOS = ["192.168.115.12:53122", "10.8.0.3:53122"];

let temporizador = null;
let proximaTentativa = 1;

function ocioso() {
  return { step: "idle", attempt_id: null, port: null, addresses: null, expires_unix_ms: null, code: null, peer: null };
}

estado.pareamento = ocioso();

function limparTemporizador() {
  clearTimeout(temporizador);
  temporizador = null;
}

function agendar(ms, fn) {
  limparTemporizador();
  temporizador = setTimeout(fn, ms);
}

function encerrar(outcome, { device = null, reason = null } = {}) {
  const attemptId = estado.pareamento.attempt_id;
  limparTemporizador();
  estado.pareamento = ocioso();
  emitir("pair_result", { attempt_id: attemptId, outcome, device, reason });
}

function expirar() {
  encerrar("expired", { reason: "Ninguém confirmou em 120 segundos." });
}

function chegadaDoPar() {
  const p = estado.pareamento;
  p.step = "code";
  p.code = "241 015";
  p.peer = { ...PAR_NOVO };
  emitir("pair_code_ready", { attempt_id: p.attempt_id, code: p.code, peer: { ...PAR_NOVO } });
  agendar(p.expires_unix_ms - Date.now(), expirar);
}

function exigirTentativa(attemptId, passos) {
  const p = estado.pareamento;
  if (p.step === "idle" || p.attempt_id !== attemptId) throw erro("no_attempt", "Não há pareamento em andamento com esse id.");
  if (!passos.includes(p.step)) throw erro("wrong_step", `Esse passo não vale agora: o pareamento está em "${p.step}".`);
  return p;
}

function iniciar(step, extra) {
  if (estado.pareamento.step !== "idle") throw erro("pair_busy", "Já existe um pareamento em andamento. Cancele antes de começar outro.");
  const attemptId = proximaTentativa++;
  estado.pareamento = {
    ...ocioso(),
    step,
    attempt_id: attemptId,
    expires_unix_ms: Date.now() + JANELA_PADRAO_S * 1000,
    ...extra,
  };
  return attemptId;
}

if (cenario === "pareamento") {
  iniciar("listening", { port: 53122, addresses: [...ENDERECOS] });
  chegadaDoPar();
}

registrar({
  pair_state: () => ({ ...estado.pareamento, peer: estado.pareamento.peer ? { ...estado.pareamento.peer } : null }),

  pair_host_start: ({ port = 0, window_secs = JANELA_PADRAO_S } = {}) => {
    if (port !== 0 && port === 47800) throw erro("bind", "A porta 47800 já está em uso pelo agente. Deixe 0 para escolher uma livre.");
    const attemptId = iniciar("listening", { port: port || 53122, addresses: [...ENDERECOS] });
    estado.pareamento.expires_unix_ms = Date.now() + window_secs * 1000;
    agendar(CHEGADA_MS, chegadaDoPar);
    return {
      attempt_id: attemptId,
      port: estado.pareamento.port,
      addresses: [...ENDERECOS],
      expires_unix_ms: estado.pareamento.expires_unix_ms,
    };
  },

  pair_join_start: ({ addr } = {}) => {
    if (typeof addr !== "string" || !/^[^\s:]+(?::\d{1,5})$/.test(addr.trim()) || Number(addr.split(":").pop()) > 65535 || Number(addr.split(":").pop()) < 1) {
      throw erro("bad_addr", "Use o formato endereço:porta, com a porta entre 1 e 65535.", { field: "addr" });
    }
    const attemptId = iniciar("connecting", {});
    agendar(RESPOSTA_PAR_MS, () => {
      if (addr.trim().startsWith("10.0.0.99")) {
        encerrar("failed", { reason: "Ninguém respondeu nesse endereço. Confira se a outra máquina está em Aceitar." });
        return;
      }
      chegadaDoPar();
    });
    return { attempt_id: attemptId };
  },

  pair_code_matches: ({ attempt_id } = {}) => {
    const p = exigirTentativa(attempt_id, ["code"]);
    p.step = "permissions";
    return {
      step: "permissions",
      available: { clipboard: estado.modulos.clipboard && p.peer.modules.clipboard, input: estado.modulos.input && p.peer.modules.input },
      defaults: { clipboard_in: false, clipboard_out: false, input_in: false },
    };
  },

  pair_finish: ({ attempt_id, grants, acknowledge_full_trust = false } = {}) => {
    const p = exigirTentativa(attempt_id, ["permissions"]);
    if (!grants || typeof grants !== "object") throw erro("bad_request", "Pedido malformado: faltam as permissões iniciais.");
    if (grants.input_in === true && acknowledge_full_trust !== true) {
      throw erro("needs_confirmation", "Conceder controle é confiança total. Confirme para continuar.", { field: "input_in" });
    }
    p.step = "waiting_peer";
    const concedidas = { clipboard_in: grants.clipboard_in === true, clipboard_out: grants.clipboard_out === true, input_in: grants.input_in === true };
    agendar(RESPOSTA_PAR_MS, () => {
      const device = {
        peer_id: peerId(p.peer.short),
        short: p.peer.short,
        name: p.peer.name,
        state: "connected",
        blocked_reason: null,
        addr: "192.168.115.77:47800",
        perms: concedidas,
        remote_modules: { ...p.peer.modules },
        remote_version: 3,
        paired_at: Date.now(),
      };
      estado.dispositivos = [...estado.dispositivos.filter((d) => d.peer_id !== device.peer_id), device];
      if (estado.layout && !estado.layout.devices.some((d) => d.peer_id === device.peer_id)) {
        estado.layout.devices.push({ peer_id: device.peer_id, name: device.name, is_self: false, state: "connected", x: 6800, y: 360, hotkey: null, monitors_source: "live", monitors: [{ ref: "eDP-1", name: "eDP-1", index: 0, x: 0, y: 0, width: 1920, height: 1080, scale: 1 }] });
      }
      encerrar("paired", { device });
      emitir("peer_connected", { peer_id: device.peer_id, short: device.short, name: device.name, addr: device.addr });
    });
    return { step: "waiting_peer" };
  },

  pair_cancel: ({ attempt_id } = {}) => {
    exigirTentativa(attempt_id, ["listening", "connecting", "code", "permissions", "waiting_peer"]);
    encerrar("cancelled", { reason: "Cancelado nesta máquina." });
    return { cancelled: true };
  },
});
