// Núcleo do mock: despacho de comandos, erros no formato do agente, eventos e cenários.
// Cada domínio (mock/*.js) registra seus comandos com registrar() e guarda dados em `estado`.

const ATRASO_MS = 160;

export const cenario = new URLSearchParams(location.search).get("cenario") ?? "padrao";
export const plataforma = new URLSearchParams(location.search).get("plataforma") === "windows" ? "windows" : "linux";

/** Dados mutáveis do mock. Cada domínio usa sua própria chave. */
export const estado = {};

const comandos = new Map();
const ouvintes = new Set();

export function registrar(mapa) {
  for (const [nome, tratador] of Object.entries(mapa)) comandos.set(nome, tratador);
}

/** UiError do contrato §12.3. `message` já em português. */
export function erro(code, message, { field = null, details = null } = {}) {
  return { code, message, field, details };
}

export const atraso = (ms = ATRASO_MS) => new Promise((resolver) => setTimeout(resolver, ms));

export function emitir(event, data = {}) {
  for (const fn of ouvintes) fn({ payload: { event, data: structuredClone(data) } });
}

// Comandos que não passam pelo agente (perfil, peers.toml e ui.toml): respondem com o agente parado ou com falha.
const SEMPRE_RESPONDEM = new Set([
  "app_state",
  "agent_start",
  "agent_stop",
  "onboarding_start",
  "onboarding_check_name",
  "onboarding_create",
  "devices_list",
  "prefs_get",
  "prefs_set",
]);

export const agenteDePe = () => estado.agente === "in_app" || estado.agente === "external";

export async function invoke(nome, { req }) {
  if (nome !== "clipsync") throw erro("unknown_command", `Comando ${nome} desconhecido.`);
  if (!req || typeof req.cmd !== "string") throw erro("bad_request", "Pedido malformado.");
  const tratador = comandos.get(req.cmd);
  if (!tratador) throw erro("unknown_command", `Comando ${req.cmd} desconhecido.`);
  await atraso();
  if (!agenteDePe() && !SEMPRE_RESPONDEM.has(req.cmd)) {
    throw erro("agent_not_running", "O agente não está rodando. Inicie o agente e tente de novo.");
  }
  const resposta = await tratador(req.args ?? {});
  return resposta === undefined ? {} : structuredClone(resposta);
}

export async function listen(_evento, fn) {
  ouvintes.add(fn);
  return () => ouvintes.delete(fn);
}
