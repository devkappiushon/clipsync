// Mock do domínio onboarding: onboarding_start, onboarding_check_name, onboarding_create e onboarding_self_test.
// A identidade nasce em memória no start e é a mesma gravada no create (mesma impressão digital).

import { EU, features } from "./dados.js";
import { atraso, emitir, erro, estado, plataforma, registrar } from "./nucleo.js";

const NOME_PADRAO = plataforma === "windows" ? "DESKTOP-J4K2" : "notebook-a";
const NOME_RE = /^[a-z0-9][a-z0-9-]{0,31}$/i;

function validarNome(name) {
  if (typeof name !== "string" || name.trim() === "") {
    throw erro("invalid_name", "Dê um nome a esta máquina.", { field: "name" });
  }
  if (name.length > 32) throw erro("invalid_name", "O nome pode ter até 32 caracteres.", { field: "name" });
  if (!NOME_RE.test(name)) {
    throw erro("invalid_name", "Use letras, números e hífen, sem espaço nem acento.", { field: "name" });
  }
}

registrar({
  onboarding_start: () => ({
    default_name: estado.inicializado ? EU.name : NOME_PADRAO,
    fingerprint_short: EU.short,
    input_supported: true,
    already_initialized: estado.inicializado,
    defaults: { input: true, clipboard: false },
  }),

  onboarding_check_name: ({ name } = {}) => {
    validarNome(name);
    return { ok: true };
  },

  onboarding_create: async ({ name, modules, port = 47800, recreate = false } = {}) => {
    validarNome(name);
    if (!modules || typeof modules !== "object") throw erro("bad_request", "Pedido malformado: faltam os módulos.");
    const input = modules.input === true;
    const clipboard = modules.clipboard === true;
    if (!input && !clipboard) throw erro("no_module", "Ligue pelo menos um módulo para esta máquina fazer algo.");
    if (estado.inicializado && recreate !== true) {
      throw erro("needs_confirmation", "Esta máquina já tem um perfil. Refazer apaga a identidade: os pares terão de parear de novo.");
    }
    await atraso(600);
    EU.name = name;
    estado.inicializado = true;
    estado.modulos = { clipboard, input };
    if (recreate) estado.dispositivos = [];
    estado.agente = "in_app";
    estado.erroAgente = null;
    emitir("agent_state", { agent: "in_app", error: null });
    return { device: { ...EU }, listen: `0.0.0.0:${port}`, modules: { clipboard, input } };
  },

  onboarding_self_test: async () => {
    await atraso(1400);
    const resultados = [];
    if (estado.modulos.clipboard) {
      resultados.push({
        module: "clipboard",
        status: plataforma === "linux" ? "validated" : "experimental",
        detail: plataforma === "linux" ? "x11-xfixes: escreveu e leu o texto sintético; conteúdo anterior restaurado." : "win32: escreveu e leu o texto sintético. Plataforma ainda sem validação ampla.",
      });
    } else {
      resultados.push({ module: "clipboard", status: "not_tested", detail: "Módulo desligado nesta máquina." });
    }
    if (!features().input) {
      resultados.push({ module: "input", status: "unavailable", detail: "Teclado e mouse ainda não funcionam nesta plataforma." });
    } else if (estado.modulos.input) {
      resultados.push({
        module: "input",
        status: "experimental",
        detail:
          plataforma === "windows"
            ? "Captura e emulação do Windows prontas, ainda sem prova nesta máquina."
            : "Portal InputCapture respondeu e o loopback moveu o cursor 3 vezes.",
      });
    } else {
      resultados.push({ module: "input", status: "not_tested", detail: "Módulo desligado nesta máquina." });
    }
    return { results: resultados };
  },
});
