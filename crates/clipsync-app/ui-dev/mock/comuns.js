// Comandos comuns (barra lateral e bandeja): app_state, agent_start, agent_stop, pause_set.

import { EU, caminhoLog, features, malha } from "./dados.js";
import { emitir, erro, estado, plataforma, registrar } from "./nucleo.js";

import { aplicarPausa } from "./pausa.js";
import { travessiaAtual } from "./travessia.js";

registrar({
  app_state: () => ({
    initialized: estado.inicializado,
    log_path: caminhoLog(),
    profile_dir: plataforma === "windows" ? "C:\\Users\\joao\\AppData\\Local\\clipsync" : "/home/joao/.config/clipsync",
    device: estado.inicializado ? EU : null,
    agent: estado.agente,
    agent_error: estado.erroAgente,
    paused: estado.pausado,
    pause_warning: estado.avisoPausa,
    screen_locked: estado.telaTravada,
    mesh: malha(),
    modules: { ...estado.modulos },
    input_supported: true,
    version: "0.5.1",
    platform: plataforma,
    features: features(),
    traversal: travessiaAtual(),
  }),

  agent_start: () => {
    if (!estado.inicializado) throw erro("not_initialized", "Esta máquina ainda não foi configurada.");
    estado.agente = "in_app";
    estado.erroAgente = null;
    emitir("agent_state", { agent: "in_app", error: null });
    return { agent: "in_app" };
  },

  agent_stop: () => {
    estado.agente = "stopped";
    emitir("agent_state", { agent: "stopped", error: null });
    return { agent: "stopped" };
  },

  pause_set: ({ paused }) => aplicarPausa(paused),
});
