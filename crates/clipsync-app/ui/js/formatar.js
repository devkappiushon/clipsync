// Formatação de valores e rótulos em português. Hora sempre no fuso local.

import { selo } from "./componentes.js";

export const INDISPONIVEL = "indisponível nesta versão";

const hora = new Intl.DateTimeFormat("pt-BR", { timeStyle: "medium" });
const diaHora = new Intl.DateTimeFormat("pt-BR", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
const decimal = new Intl.NumberFormat("pt-BR", { maximumFractionDigits: 1 });

export function formatarHora(unixMs) {
  return hora.format(new Date(unixMs));
}

/** "15:40:37" se for hoje, senão "27 out, 15:40". */
export function formatarMomento(unixMs) {
  const data = new Date(unixMs);
  return data.toDateString() === new Date().toDateString() ? hora.format(data) : diaHora.format(data);
}

export function formatarBytes(bytes) {
  if (bytes < 1024) return `${bytes} ${bytes === 1 ? "byte" : "bytes"}`;
  if (bytes < 1024 * 1024) return `${decimal.format(bytes / 1024)} KiB`;
  return `${decimal.format(bytes / (1024 * 1024))} MiB`;
}

export function formatarTipo(mime) {
  return mime.startsWith("text/") ? "texto" : mime;
}

/** Mensagem pronta para a tela, venha o erro do agente ({ code, message }) ou de JS. */
export function mensagemDe(erro) {
  if (erro && typeof erro.message === "string" && erro.message) return erro.message;
  return "Algo deu errado. Tente de novo.";
}

const ESTADOS_DISPOSITIVO = {
  connected: ["ok", "conectado"],
  connecting: ["acento", "conectando"],
  offline: ["aviso", "offline"],
  blocked: ["erro", "bloqueado"],
};

export function seloDispositivo(estado) {
  const [tom, texto] = ESTADOS_DISPOSITIVO[estado] ?? ["neutro", estado];
  return selo(tom, texto);
}

const ESTADOS_TESTE = {
  validated: ["ok", "validado"],
  experimental: ["aviso", "experimental"],
  unavailable: ["erro", "indisponível"],
  not_tested: ["neutro", "não testado"],
};

export function seloTeste(status) {
  const [tom, texto] = ESTADOS_TESTE[status] ?? ["neutro", status];
  return selo(tom, texto);
}

const ESTADOS_DIAGNOSTICO = {
  ok: ["ok", "ok"],
  warn: ["aviso", "atenção"],
  missing: ["neutro", "ausente"],
  error: ["erro", "erro"],
};

export function seloDiagnostico(status) {
  const [tom, texto] = ESTADOS_DIAGNOSTICO[status] ?? ["neutro", status];
  return selo(tom, texto);
}

const ESTADOS_ITEM = {
  received: ["acento", "recebido"],
  applied: ["ok", "aplicado"],
  in_inbox: ["acento", "na caixa"],
  rejected: ["aviso", "recusado"],
  expired: ["neutro", "expirado"],
  unreachable: ["erro", "não entregue"],
};

export function seloItemCaixa(state) {
  const [tom, texto] = ESTADOS_ITEM[state] ?? ["neutro", state];
  return selo(tom, texto);
}

const CLASSIFICACOES = {
  validated: ["ok", "plataforma validada"],
  experimental: ["aviso", "plataforma experimental"],
  unavailable: ["erro", "plataforma indisponível"],
  test: ["neutro", "adaptador de teste"],
};

export function seloClassificacao(classification) {
  const [tom, texto] = CLASSIFICACOES[classification] ?? ["neutro", classification];
  return selo(tom, texto);
}

export function formatarMinutos(ms) {
  const minutos = Math.round(ms / 60_000);
  return `${minutos} ${minutos === 1 ? "minuto" : "minutos"}`;
}
