// Núcleo do mock: despacho de comandos, erros no formato do agente, eventos e cenários.
// Cada domínio (mock/*.js) registra seus comandos com registrar() e guarda dados em `estado`.

const ATRASO_MS = 160;
// Controle DEV: só o despacho muda; os handlers continuam somando seus próprios atrasos.
const latenciaPedida = new URLSearchParams(location.search).get("latencia");
const LATENCIA_DESPACHO = ["160", "500", "1400"].includes(latenciaPedida) ? Number(latenciaPedida) : ATRASO_MS;

export const cenario = new URLSearchParams(location.search).get("cenario") ?? "padrao";
// ?cenario=windows (ou ?plataforma=windows) simula o PC Windows da S7: dois monitores, um com escala 1,5, e o input ligado.
// ?cenario=windows-recusa é o mesmo, com o Windows tendo recusado o Win+Alt+P de retomar (I-S7-2): abre pausado.
export const CENARIOS_MONITORES = new Set(["monitores-independentes", "monitor-ausente", "monitor-ambiguo", "geometria-estimada", "inventario-alterado", "protocolo-v3", "organizacao-nao-confirmada", "oferta-monitores"]);
export const plataforma = (CENARIOS_MONITORES.has(cenario) || cenario.startsWith("windows")) || new URLSearchParams(location.search).get("plataforma") === "windows" ? "windows" : "linux";

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
  "modules_set",
  "clipboard_set_mode",
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
  await atraso(LATENCIA_DESPACHO);
  const mutacao = ["agent_start", "agent_stop", "onboarding_create", "clipboard_set_mode", "pair_host_start", "pair_join_start", "pair_code_matches", "pair_finish", "pair_cancel", "lock_to_screen"].includes(req.cmd)
    || (req.cmd === "pause_set" && req.args?.paused !== true);
  if (estado.modulosOcupados && mutacao) throw erro("profile_busy", "Reconfiguração do perfil em andamento.");
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
