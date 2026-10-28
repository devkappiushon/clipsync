// Mock do domínio preferências: prefs_get e prefs_set. O ui.toml não depende do agente,
// por isso os dois respondem mesmo no cenário erro; lá, só a gravação falha com `io`.
// ?tema=claro|escuro define o tema inicial, para capturar as telas nos dois temas.

import { cenario, emitir, erro, estado, registrar } from "./nucleo.js";

const TEMAS = { claro: "light", escuro: "dark", sistema: "system" };
const temaInicial = TEMAS[new URLSearchParams(location.search).get("tema")] ?? "system";

estado.prefs = {
  shortcuts: {
    open: "Win+Alt+C",
    pause_all: "Win+Alt+P",
    send_once: "Win+Alt+V",
    lock_screen: "Win+Alt+L",
    go_to_device: ["Win+Alt+1", "Win+Alt+2", "Win+Alt+3"],
  },
  autostart: false,
  notifications: { item_received: true, device_connected: false, block: true },
  appearance: { theme: temaInicial, reduce_motion: false },
  language: "pt-BR",
  version: "0.2.0",
  license: "a definir",
};

const CAMPOS = new Set(["autostart", "notifications", "appearance"]);
const TEMAS_VALIDOS = new Set(["system", "light", "dark"]);

registrar({
  prefs_get: () => estado.prefs,

  prefs_set: (args) => {
    for (const chave of Object.keys(args)) {
      if (!CAMPOS.has(chave)) throw erro("bad_request", `O campo ${chave} não existe nas preferências.`, { field: chave });
    }
    if (args.appearance?.theme !== undefined && !TEMAS_VALIDOS.has(args.appearance.theme)) {
      throw erro("bad_request", "Tema inválido: use system, light ou dark.", { field: "appearance.theme" });
    }
    if (cenario === "erro") throw erro("io", "Não foi possível gravar o ui.toml: permissão negada.");
    if ("autostart" in args) estado.prefs.autostart = args.autostart === true;
    if (args.notifications) estado.prefs.notifications = { ...estado.prefs.notifications, ...args.notifications };
    if (args.appearance) estado.prefs.appearance = { ...estado.prefs.appearance, ...args.appearance };
    emitir("prefs_changed", { prefs: estado.prefs });
    return { prefs: estado.prefs };
  },
});
