// Mock do agente para abrir a interface no navegador (nunca entra no app Tauri).
// Cenários: ?cenario=padrao | vazio | sem-perfil | erro | pareamento | estados
// Plataforma: ?plataforma=windows simula o PC com input e atalhos do Windows.
// Cada domínio mora em mock/*.js e registra seus comandos no núcleo.

import "./mock/dados.js";
import "./mock/comuns.js";
import "./mock/onboarding.js";
import "./mock/dispositivos.js";
import "./mock/pareamento.js";
import "./mock/layout.js";
import "./mock/clipboard.js";
import "./mock/diagnostico.js";
import "./mock/prefs.js";

export { invoke, listen } from "./mock/nucleo.js";

import { listen as ouvir, emitir } from "./mock/nucleo.js";
import { travessiaAtual } from "./mock/travessia.js";
const ATUALIZA_TRAVESSIA = new Set(["pause_changed", "screen_lock_changed", "agent_state", "peer_connected", "peer_disconnected", "layout_changed", "resync"]);
ouvir("clipsync://event", ({ payload }) => {
  if (ATUALIZA_TRAVESSIA.has(payload.event)) emitir("traversal_changed", { traversal: travessiaAtual() });
});
