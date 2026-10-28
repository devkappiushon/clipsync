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

export function posicionarDestino(layout, { peer_id, edge }) {
  const origem = layout.devices.find((d) => d.peer_id === layout.self_id);
  const destino = layout.devices.find((d) => d.peer_id === peer_id);
  const a = caixa(origem);
  const b = caixa(destino);
  if (!a || !b || !Object.hasOwn(OPOSTA, edge)) return null;
  const passo = layout.snap_px || 16;
  const encaixar = (n) => Math.round(n / passo) * passo;
  const pos = { x: destino.x, y: destino.y };
  if (edge === "left") pos.x += a.x - b.width - 160 - b.x;
  if (edge === "right") pos.x += a.x + a.width + 160 - b.x;
  if (edge === "top") pos.y += a.y - b.height - 160 - b.y;
  if (edge === "bottom") pos.y += a.y + a.height + 160 - b.y;
  return { x: encaixar(pos.x), y: encaixar(pos.y) };
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

function pontaExterna(layout, dispositivo, outro, edge, ignorarId) {
  const ms = dispositivo.monitors.filter(monitorReal);
  const outraCaixa = caixa(outro);
  if (ms.length === 0 || !outraCaixa) return null;
  const externo = (m) => ({ left: -m.x, right: m.x + m.width, top: -m.y, bottom: m.y + m.height })[edge];
  const alvo = vertical(edge) ? outraCaixa.y + outraCaixa.height / 2 : outraCaixa.x + outraCaixa.width / 2;
  const centro = (m) => vertical(edge) ? dispositivo.y + m.y + m.height / 2 : dispositivo.x + m.x + m.width / 2;
  // Monitores de alturas diferentes têm bordas externas que não estão no extremo global.
  // O alinhamento com o destino vem primeiro; locked_edges exclui os trechos internos.
  const candidatos = [...ms].sort((a, b) => Math.abs(centro(a) - alvo) - Math.abs(centro(b) - alvo) || externo(b) - externo(a) || a.index - b.index || a.ref.localeCompare(b.ref));
  for (const m of candidatos) {
    const ponta = { peer_id: dispositivo.peer_id, monitor: m.ref, edge };
    const livres = trechosLivres(layout, ponta, ignorarId);
    const trecho = livres.reduce((maior, t) => !maior || t.to - t.from > maior.to - maior.from ? t : maior, null);
    if (trecho) return { ...ponta, ...trecho };
  }
  return null;
}

export function proporLigacao(layout, { peer_id, edge }) {
  if (!Object.hasOwn(OPOSTA, edge) || peer_id === layout.self_id) return null;
  const origem = layout.devices.find((d) => d.peer_id === layout.self_id && d.is_self);
  const destino = layout.devices.find((d) => d.peer_id === peer_id && !d.is_self && d.state === "connected");
  if (!origem || !destino) return null;
  const existentes = ligacoesDoPar(layout, peer_id);
  if (existentes.length > 1) return null;
  const id = existentes[0]?.id ?? null;
  const a = pontaExterna(layout, origem, destino, edge, id);
  const b = pontaExterna(layout, destino, origem, OPOSTA[edge], id);
  return a && b ? { id, a, b } : null;
}

export function resultadoDoTeste(stats) {
  if (!(stats?.received > 0)) return { tom: "aviso", titulo: "Teste inconclusivo", texto: "Nenhum evento foi contabilizado. Confira a ligação e as permissões no destino; tente novamente ou abra o Diagnóstico." };
  const perdas = stats.lost > 0 || stats.out_of_order > 0;
  return {
    tom: perdas ? "aviso" : "info",
    titulo: perdas ? "Eventos recebidos com ressalvas" : "Eventos recebidos no teste",
    texto: `${stats.received} evento(s) recebido(s), ${stats.lost} perdido(s), ${stats.out_of_order} fora de ordem. Confira a passagem e a volta usando o mouse: este teste não verifica o retorno pela borda.`,
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
