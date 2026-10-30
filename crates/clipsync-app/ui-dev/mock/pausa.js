// Fixtures DEV B09; nenhuma persistência, perfil ou permissão real.
import { cenario, emitir, erro, estado } from "./nucleo.js";

estado.avisoPausa = null;
estado.falhaPersistenciaPausa = ["b09-falha-pausa", "b09-falha-retomada", "b09-pausa-volatil", "b09-retomada-volatil", "modulos-falha-pausa"].includes(cenario);
if (["b09-falha-retomada", "b09-pausa-volatil"].includes(cenario)) estado.pausado = true;
if (cenario === "b09-externo") estado.agente = "external";
if (cenario === "b09-parado") estado.agente = "stopped";
if (cenario === "b09-desconhecido") {
  estado.agente = "in_app";
  estado.erroAgente = "O agente próprio ainda existe, mas seu estado não foi confirmado.";
  estado.avisoPausa = "A pausa solicitada não foi confirmada. Confira o estado antes de tentar novamente.";
}
function aviso(paused, persistencia) {
  const acao = paused ? "pausa" : "retomada";
  return persistencia
    ? `A ${acao} foi aplicada em memória, mas não foi salva. Esse estado não sobreviverá ao encerramento.`
    : `A ${acao} solicitada não foi confirmada. Confira o estado antes de tentar novamente.`;
}
if (["b09-pausa-volatil", "b09-retomada-volatil"].includes(cenario)) estado.avisoPausa = aviso(cenario === "b09-pausa-volatil", true);

export function aplicarPausa(paused) {
  if (typeof paused !== "boolean") throw erro("bad_request", "Informe paused como booleano.");
  if (cenario === "b09-desconhecido") {
    estado.avisoPausa = aviso(paused, false);
    emitir("resync");
    throw erro("io", "Não foi possível confirmar o estado do agente.");
  }
  estado.pausado = paused;
  estado.retomadaRecusada = paused && cenario === "windows-recusa";
  if (estado.retomadaRecusada) estado.bloqueioRecusa = true;
  emitir("pause_changed", { paused });
  if (estado.falhaPersistenciaPausa) {
    estado.avisoPausa = aviso(paused, true);
    emitir("resync");
    throw erro("io", "estado aplicado em memória, mas não foi salvo");
  }
  estado.avisoPausa = null;
  emitir("resync");
  return { paused };
}
