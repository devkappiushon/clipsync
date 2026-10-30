// Somente simulador DEV. Não escreve perfis, identidades, concessões ou configurações reais.
import { aplicarPausa } from "./pausa.js";
import { atraso, cenario, emitir, erro, estado, registrar } from "./nucleo.js";

if (cenario === "modulos-externo") estado.agente = "external";
if (cenario === "modulos-parado") estado.agente = "stopped";
if (cenario === "modulos-desconhecido") {
  estado.agente = "failed";
  estado.erroAgente = "Não foi possível confirmar o estado do agente.";
}

function efeitosIniciais() {
  return { pause: "not_requested", pause_persisted: null, stop: "not_requested", config_saved: false, start: "not_requested", lock_restore: "not_needed" };
}
function observado() {
  const desconhecido = ["modulos-desconhecido", "b09-desconhecido"].includes(cenario);
  return {
    agent: desconhecido ? "unknown" : estado.agente,
    paused: desconhecido ? null : estado.pausado,
    screen_locked: desconhecido || !estado.modulos.input ? null : estado.telaTravada,
    modules: { ...estado.modulos },
  };
}
function conjunto(valor) {
  return valor !== null && typeof valor === "object" && !Array.isArray(valor)
    && Object.keys(valor).length === 2 && typeof valor.input === "boolean" && typeof valor.clipboard === "boolean";
}
const igual = (a, b) => a.input === b.input && a.clipboard === b.clipboard;

async function reconfigurar(args, modo = null) {
    const effects = efeitosIniciais();
    const falhar = (code, message, observacao = observado()) => {
      emitir("resync");
      throw erro(code, message, { details: { effects: { ...effects }, observed: observacao } });
    };
    if (modo === null && (!args || Object.keys(args).some((key) => !["modules", "expected"].includes(key)) || !conjunto(args.modules) || !conjunto(args.expected))) {
      falhar("bad_request", "Informe input e clipboard como booleanos em modules e expected.");
    }
    if (!estado.inicializado) falhar("not_initialized", "Esta máquina ainda não foi configurada.");
    if (estado.modulosOcupados || cenario === "modulos-ocupado" || (estado.pareamento && estado.pareamento.step !== "idle")) {
      falhar("profile_busy", "Outra alteração do perfil está em andamento.");
    }
    if (modo !== null && !estado.modulos.clipboard) falhar("module_off", "O módulo de clipboard está desligado.");
    if (modo !== null && !["auto", "manual"].includes(modo)) falhar("bad_request", "Modo inválido: use auto ou manual.");
    const proximo = modo === null ? args.modules : { ...estado.modulos };
    const resposta = (changed) => modo === null
      ? { changed, modules: { ...estado.modulos }, effects, observed: observado() }
      : { mode: estado.clipboard.mode, restart_needed: estado.agente === "external", effects, observed: observado() };
    if (modo === null && !igual(args.expected, estado.modulos)) falhar("stale_config", "A configuração mudou. Recarregue antes de aplicar.");
    if (!proximo.input && !proximo.clipboard) falhar("no_usable_module", "Mantenha pelo menos um módulo ligado.");
    if (modo === null ? igual(proximo, estado.modulos) : modo === estado.clipboard.mode) return resposta(false);
    if (estado.agente === "external" && modo !== null) {
      estado.clipboard.mode = modo;
      effects.config_saved = true;
      emitir("resync");
      return resposta(true);
    }
    if (estado.agente === "external") falhar("agent_external", "O agente externo não pode ser reiniciado por este app.");
    if (["modulos-desconhecido", "b09-desconhecido"].includes(cenario)) falhar("agent_unknown", "Não foi possível confirmar o estado do agente.");

    const proprio = estado.agente === "in_app";
    const trava = estado.modulos.input ? estado.telaTravada : false;
    const restaurarTrava = estado.modulos.input && proximo.input;
    estado.modulosOcupados = true;
    let paradaPendente = false;
    try {
      if (proprio) {
        effects.pause = "requested";
        try {
          aplicarPausa(true);
        } catch (error) {
          effects.pause_persisted = estado.falhaPersistenciaPausa ? false : null;
          falhar(error.code, error.message);
        }
        effects.pause = "confirmed";
        effects.pause_persisted = true;
        effects.stop = "requested";
        await atraso();
        if (cenario === "modulos-falha-parada") {
          paradaPendente = true;
          falhar("agent_stop_pending", "A parada continua em andamento. Os módulos não foram salvos.");
        }
        estado.agente = "stopped";
        effects.stop = "confirmed";
      }
      if (cenario === "modulos-externo-antes-save") {
        estado.agente = "external";
        falhar("agent_external", "Outro agente iniciou. A configuração não foi alterada.", { ...observado(), paused: null, screen_locked: null });
      }
      if (cenario === "modulos-conflito-tardio") falhar("stale_config", "A configuração mudou durante a parada. Recarregue antes de aplicar.");
      if (cenario === "modulos-falha-escrita") falhar("io", "O agente está parado. Não foi possível salvar os módulos.");
      if (modo === null) estado.modulos = { ...proximo };
      else estado.clipboard.mode = modo;
      effects.config_saved = true;
      if (proprio) {
        effects.start = "requested";
        if (restaurarTrava) effects.lock_restore = "pending";
        await atraso();
        if (cenario === "modulos-externo-apos-save") {
          estado.agente = "external";
          effects.start = "external";
          if (restaurarTrava) effects.lock_restore = "failed";
          falhar("agent_external", "Módulos salvos, mas outro agente iniciou. Nenhuma retomada foi enviada.", { ...observado(), paused: null, screen_locked: null });
        }
        if (["modulos-falha-inicio", "modulos-falha-trava", "b09-falha-inicio", "b09-falha-trava"].includes(cenario)) {
          estado.agente = "failed";
          estado.erroAgente = "Módulos salvos, mas a inicialização não foi confirmada.";
          effects.start = "failed";
          if (restaurarTrava) effects.lock_restore = "failed";
          falhar("agent_start_failed", estado.erroAgente, { ...observado(), screen_locked: null });
        }
        estado.agente = "in_app";
        estado.erroAgente = null;
        estado.telaTravada = proximo.input ? trava : false;
        effects.start = "owned_ready";
        if (restaurarTrava) effects.lock_restore = "confirmed";
      } else {
        estado.agente = "stopped";
      }
      emitir("resync");
      return resposta(true);
    } finally {
      estado.modulosOcupados = paradaPendente;
    }
}
registrar({
  modules_set: (args) => reconfigurar(args),
  clipboard_set_mode: ({ mode } = {}) => reconfigurar(null, mode ?? "inválido"),
});
