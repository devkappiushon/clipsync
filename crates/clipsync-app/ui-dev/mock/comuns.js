// Comandos comuns (barra lateral e bandeja): app_state, agent_start, agent_stop, pause_set.

import { EU, features, malha } from "./dados.js";
import { cenario, emitir, erro, estado, plataforma, registrar } from "./nucleo.js";

import { travessiaAtual } from "./travessia.js";

registrar({
  app_state: () => ({
    initialized: estado.inicializado,
    profile_dir: plataforma === "windows" ? "C:\\Users\\joao\\AppData\\Local\\clipsync" : "/home/joao/.config/clipsync",
    device: estado.inicializado ? EU : null,
    agent: estado.agente,
    agent_error: estado.erroAgente,
    paused: estado.pausado,
    screen_locked: estado.telaTravada,
    mesh: malha(),
    modules: { ...estado.modulos },
    input_supported: true,
    version: "0.3.1",
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

  pause_set: ({ paused }) => {
    estado.pausado = paused === true;
    // O adaptador só tenta registrar o Win+Alt+P ao pausar. Se o Windows recusar, o campo liga e o bloqueio
    // fica registrado; ao retomar, o campo volta a false e o bloqueio continua no Diagnóstico.
    estado.retomadaRecusada = estado.pausado && cenario === "windows-recusa";
    if (estado.retomadaRecusada) estado.bloqueioRecusa = true;
    emitir("pause_changed", { paused: estado.pausado });
    return { paused: estado.pausado };
  },
});
