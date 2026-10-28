// Estado global leve: o AppState da barra lateral e a distribuição dos eventos do agente.

import { api, ouvir } from "./api.js";
import { aplicarTema } from "./tema.js";

let app = null;
const aoMudarApp = new Set();
const assinantes = new Map();

export const obterApp = () => app;

/** Recurso ligado nesta versão? Ex.: recurso("peer_pause"). Sem estado carregado, false. */
export const recurso = (nome) => app?.features?.[nome] === true;

export function observarApp(fn) {
  aoMudarApp.add(fn);
  return () => aoMudarApp.delete(fn);
}

function definirApp(proximo) {
  app = proximo;
  for (const fn of aoMudarApp) fn(app);
}

export async function atualizarApp() {
  definirApp(await api.appState());
  return app;
}

/** Assina um ou mais eventos (nomes do §12.5). fn(data, nome). Devolve a função que cancela. */
export function assinar(eventos, fn) {
  const nomes = Array.isArray(eventos) ? eventos : [eventos];
  for (const nome of nomes) {
    if (!assinantes.has(nome)) assinantes.set(nome, new Set());
    assinantes.get(nome).add(fn);
  }
  return () => {
    for (const nome of nomes) assinantes.get(nome)?.delete(fn);
  };
}

const TEMAS = { system: "sistema", light: "claro", dark: "escuro" };

export function aplicarAparencia(aparencia) {
  aplicarTema(TEMAS[aparencia.theme] ?? "sistema");
  document.documentElement.toggleAttribute("data-reduzir-movimento", aparencia.reduce_motion === true);
}

let reconsulta = null;
function reconsultarApp() {
  clearTimeout(reconsulta);
  reconsulta = setTimeout(() => atualizarApp().catch(() => {}), 150);
}

function tratar({ event, data }) {
  switch (event) {
    case "pause_changed":
      if (app) definirApp({ ...app, paused: data.paused });
      // Pausar ou retomar muda também o que o agente informa em features (resume_hotkey_refused).
      reconsultarApp();
      break;
    case "agent_state":
      if (app) definirApp({ ...app, agent: data.agent, agent_error: data.error });
      break;
    case "screen_lock_changed":
      if (app) definirApp({ ...app, screen_locked: data.screen_locked });
      break;
    case "prefs_changed":
      aplicarAparencia(data.prefs.appearance);
      break;
    case "peer_connected":
    case "peer_disconnected":
    case "peer_blocked":
    case "resync":
      reconsultarApp();
      break;
    default:
      break;
  }
  for (const fn of assinantes.get(event) ?? []) fn(data, event);
}

export async function iniciarEventos() {
  await ouvir(tratar);
}
