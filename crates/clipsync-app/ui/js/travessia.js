// Cálculos puros do fluxo simples: posição do computador, monitores reais e trechos livres.
// A proposta não grava nada. O botão Ligar passa o resultado ao comando validado do agente.
const OPOSTA = { left: "right", right: "left", top: "bottom", bottom: "top" };
const vertical = (edge) => edge === "left" || edge === "right";
const mesmaBorda = (a, b) => a.peer_id === b.peer_id && a.monitor === b.monitor && a.edge === b.edge;
const monitorReal = (m) => typeof m.ref === "string" && m.ref.length > 0 && Number.isFinite(m.x) && Number.isFinite(m.y) && m.width > 0 && m.height > 0;

export function ligacoesLocais(layout) {
  const conhecidos = new Set(layout.devices.map((d) => d.peer_id));
  return layout.links.filter((l) => (l.a.peer_id === layout.self_id && conhecidos.has(l.b.peer_id)) || (l.b.peer_id === layout.self_id && conhecidos.has(l.a.peer_id)));
}

export function ligacoesDoPar(layout, peerId) {
  return ligacoesLocais(layout).filter((l) => l.a.peer_id === peerId || l.b.peer_id === peerId);
}

export function trechosLivres(layout, ponta, ignorarId = null) {
  const monitor = layout.devices.find((d) => d.peer_id === ponta.peer_id)?.monitors.find((m) => m.ref === ponta.monitor);
  if (!monitor || !monitorReal(monitor)) return [];
  const comprimento = vertical(ponta.edge) ? monitor.height : monitor.width;
  const canto = (layout.options?.dead_corner_px ?? 0) / comprimento;
  const fim = 1 - canto;
  if (!(canto < fim)) return [];
  const ocupados = [...layout.locked_edges, ...layout.links.filter((l) => l.id !== ignorarId).flatMap((l) => [l.a, l.b])]
    .filter((p) => mesmaBorda(p, ponta))
    .sort((a, b) => a.from - b.from);
  const livres = [];
  let inicio = canto;
  for (const p of ocupados) {
    const de = Math.min(fim, Math.max(canto, p.from));
    const ate = Math.min(fim, p.to);
    // O agente usa f32. Uma lacuna só de arredondamento não pode habilitar Ligar.
    if (Math.fround(de) > Math.fround(inicio)) livres.push({ from: inicio, to: de });
    inicio = Math.max(inicio, ate);
  }
  if (Math.fround(inicio) < Math.fround(fim)) livres.push({ from: inicio, to: fim });
  return livres;
}

function caixa(d) {
  const ms = d?.monitors.filter(monitorReal) ?? [];
  if (ms.length === 0) return null;
  const x = Math.min(...ms.map((m) => m.x));
  const y = Math.min(...ms.map((m) => m.y));
  return { x: d.x + x, y: d.y + y, width: Math.max(...ms.map((m) => m.x + m.width)) - x, height: Math.max(...ms.map((m) => m.y + m.height)) - y };
}

function planejarPosicao(layout, { peer_id, edge, posicao }) {
  const origem = layout.devices.find((d) => d.peer_id === layout.self_id && d.is_self);
  const salvo = layout.devices.find((d) => d.peer_id === peer_id && !d.is_self);
  const ajustada = Number.isFinite(posicao?.x) && Number.isFinite(posicao?.y);
  const destino = salvo && ajustada ? { ...salvo, x: posicao.x, y: posicao.y } : salvo;
  const a = caixa(origem);
  const b = caixa(destino);
  if (!a || !b || !Object.hasOwn(OPOSTA, edge)) return null;
  const existentes = ligacoesDoPar(layout, peer_id);
  if (existentes.length > 1) return null;
  const id = existentes[0]?.id ?? null;
  const candidatos = (d, lado) => d.monitors.filter(monitorReal).flatMap((m) => {
    const ponta = { peer_id: d.peer_id, monitor: m.ref, edge: lado };
    const trecho = trechosLivres(layout, ponta, id).reduce((maior, t) => !maior || t.to - t.from > maior.to - maior.from ? t : maior, null);
    return trecho ? [{ m, ponta: { ...ponta, ...trecho } }] : [];
  });
  const eixo = vertical(edge) ? "y" : "x";
  const tamanho = vertical(edge) ? "height" : "width";
  const externo = (m) => ({ left: -m.x, right: m.x + m.width, top: -m.y, bottom: m.y + m.height })[edge];
  const pares = candidatos(origem, edge).flatMap((de) => candidatos(destino, OPOSTA[edge]).map((para) => {
    const inicioA = origem[eixo] + de.m[eixo];
    const inicioB = destino[eixo] + para.m[eixo];
    const centroA = inicioA + de.m[tamanho] / 2;
    const centroB = inicioB + para.m[tamanho] / 2;
    const alinhado = inicioA < inicioB + para.m[tamanho] && inicioB < inicioA + de.m[tamanho];
    return { de, para, deslocamento: alinhado ? 0 : centroA - centroB, distancia: Math.abs(centroA - centroB) };
  })).sort((p, q) => Math.abs(p.deslocamento) - Math.abs(q.deslocamento) || p.distancia - q.distancia || externo(q.de.m) - externo(p.de.m) || p.de.m.ref.localeCompare(q.de.m.ref));
  const par = pares[0];
  if (!par) return null;
  const passo = layout.snap_px || 16;
  const encaixar = (n) => Math.round(n / passo) * passo;
  const pos = { x: destino.x, y: destino.y };
  if (ajustada) return { pos, id, a: par.de.ponta, b: par.para.ponta };
  if (edge === "left") pos.x += a.x - b.width - 160 - b.x;
  if (edge === "right") pos.x += a.x + a.width + 160 - b.x;
  if (edge === "top") pos.y += a.y - b.height - 160 - b.y;
  if (edge === "bottom") pos.y += a.y + a.height + 160 - b.y;
  pos[eixo] += par.deslocamento;
  return { pos: { x: encaixar(pos.x), y: encaixar(pos.y) }, id, a: par.de.ponta, b: par.para.ponta };
}

export function posicionarDestino(layout, escolha) {
  return planejarPosicao(layout, escolha)?.pos ?? null;
}

export function posicaoRelativa(layout, peerId) {
  const a = caixa(layout.devices.find((d) => d.peer_id === layout.self_id));
  const b = caixa(layout.devices.find((d) => d.peer_id === peerId));
  if (!a || !b) return null;
  const dx = b.x + b.width / 2 - a.x - a.width / 2;
  const dy = b.y + b.height / 2 - a.y - a.height / 2;
  if (dx === 0 && dy === 0) return null;
  return Math.abs(dx) / (a.width + b.width) >= Math.abs(dy) / (a.height + b.height) ? (dx >= 0 ? "right" : "left") : (dy >= 0 ? "bottom" : "top");
}

export function proporLigacao(layout, escolha) {
  const destino = layout.devices.find((d) => d.peer_id === escolha.peer_id && !d.is_self && d.state === "connected");
  if (!destino) return null;
  const plano = planejarPosicao(layout, escolha);
  return plano ? { id: plano.id, a: plano.a, b: plano.b } : null;
}

// Intervalo permitido para cada alça, incluindo cantos e trechos ocupados.
export function limitesAlca(layout, ponta, lado, ignorarId = null) {
  const livre = trechosLivres(layout, ponta, ignorarId).find((t) => t.from <= ponta.from && t.to >= ponta.to);
  if (!livre) return null;
  return lado === "from"
    ? { min: livre.from, max: Math.min(livre.to, ponta.to - 1 / 16) }
    : { min: Math.max(livre.from, ponta.from + 1 / 16), max: livre.to };
}

export function resultadoDoTeste(stats) {
  if (!(stats?.received > 0)) return { tom: "aviso", titulo: "Teste inconclusivo", texto: "Nenhum evento foi contabilizado. Confira a ligação e as permissões no destino; tente novamente ou abra o Diagnóstico." };
  const perdas = stats.lost > 0;
  const ressalvas = perdas || stats.out_of_order > 0;
  return {
    tom: ressalvas ? "aviso" : "info",
    titulo: ressalvas ? "Eventos recebidos com ressalvas" : "Eventos recebidos no teste",
    texto: `${stats.received} evento(s) recebido(s), ${stats.lost} perdido(s), ${stats.out_of_order} fora de ordem. ${perdas ? "Houve perdas durante o teste. " : stats.out_of_order > 0 ? "Eventos fora de ordem não comprovam perda. " : ""}Confira a passagem e a volta usando o mouse: este teste não verifica o retorno pela borda.`,
  };
}

const RECUSAS_ANTES_DE_GRAVAR = new Set(["stale_rev", "same_device", "internal_edge_locked", "empty_segment", "segment_out_of_range", "segment_overlap", "link_not_found", "unknown_peer", "bad_request"]);

export function falhaAoLigar(code, detalhe, posicaoSalva) {
  if (!posicaoSalva) return detalhe;
  const prefixo = RECUSAS_ANTES_DE_GRAVAR.has(code)
    ? "A posição foi salva, mas a ligação não foi alterada. "
    : "A posição foi salva. Não foi possível confirmar a atualização da ligação. ";
  return prefixo + detalhe;
}
