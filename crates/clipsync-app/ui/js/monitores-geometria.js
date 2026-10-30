// Modelo puro do rascunho. Não prepara inventário, cria links ou autoriza aplicação.
const GAP_SEMENTE = 32;
const id = (key) => `${key.peer.toLowerCase()}:${key.uid.toLowerCase()}`;
const copiarPosicao = ({ key, x, y }) => ({ key: { ...key }, x, y });
const peerValido = (peer) => typeof peer === "string" && /^[a-f0-9]{64}$/i.test(peer);
const comparar = (a, b) => a < b ? -1 : a > b ? 1 : 0;

function recusar(code) {
  // Código fixo; nenhum rótulo, UID ou outro texto do par vai para o erro.
  throw Object.assign(new Error(code), { code });
}

function validarChave(key) {
  if (!key || !peerValido(key.peer)
      || typeof key.uid !== "string" || !/^[a-f0-9]{32}$/i.test(key.uid) || /^0{32}$/.test(key.uid)) recusar("invalid_key");
}

function validarRevisao(valor) {
  if (!Number.isSafeInteger(valor) || valor < 0) recusar("invalid_revision");
}

function registrarUnico(vistos, chave) {
  if (vistos.has(chave)) recusar("identity_ambiguous");
  vistos.add(chave);
}

function dimensoesVirtuais(monitor, inventory) {
  const r = monitor.rect;
  const inteiro = (v, min, max) => Number.isInteger(v) && v >= min && v <= max;
  if (!r || !inteiro(r.x, -2_147_483_648, 2_147_483_647) || !inteiro(r.y, -2_147_483_648, 2_147_483_647)
      || !inteiro(r.width, 1, 4_294_967_295) || !inteiro(r.height, 1, 4_294_967_295)
      || r.x + r.width > 2_147_483_647 || r.y + r.height > 2_147_483_647
      || !Number.isFinite(monitor.scale) || monitor.scale <= 0
      || !["Physical", "Logical"].includes(inventory.space)
      || !["Observed", "Estimated"].includes(inventory.source)) recusar("invalid_geometry");
  const divisor = inventory.space === "Physical" ? monitor.scale : 1;
  const width = r.width / divisor;
  const height = r.height / divisor;
  if (!Number.isFinite(width) || width <= 0 || !Number.isFinite(height) || height <= 0) recusar("invalid_geometry");
  return { width, height };
}

function validarPosicao(posicao) {
  if (!Number.isFinite(posicao?.x) || !Number.isFinite(posicao?.y)
      || Math.abs(posicao.x) > 1_000_000 || Math.abs(posicao.y) > 1_000_000) recusar("invalid_position");
}

function buscarPosicao(rascunho, key) {
  validarChave(key);
  const encontradas = rascunho.positions.filter((p) => id(p.key) === id(key));
  if (encontradas.length > 1) recusar("identity_ambiguous");
  if (!encontradas.length) recusar("monitor_missing");
  return encontradas[0];
}

function comMonitores(request, monitores) {
  // Só os quatro campos enumeráveis seguem no request. JSON/spread perdem metadata:
  // o controlador deve conservar este draft e usar as operações deste módulo.
  // Recriar exige o snapshot apropriado, sem renovar stamps de uma prévia antiga.
  const itens = monitores.map((m) => Object.freeze({ ...m, key: Object.freeze({ ...m.key }) }));
  Object.defineProperty(request, "monitores", { value: Object.freeze(itens) });
  return request;
}

export function criarRascunho(layout) {
  validarRevisao(layout.rev);
  const peers = new Set();
  const chaves = new Set();
  const inventories = [];
  const nativos = [];
  for (const { stamp, inventory } of layout.inventories ?? []) {
    if (!peerValido(stamp.peer)) recusar("invalid_key");
    registrarUnico(peers, stamp.peer.toLowerCase());
    validarRevisao(stamp.connection);
    validarRevisao(stamp.rev);
    validarRevisao(inventory.rev);
    inventories.push({ peer: stamp.peer, connection: stamp.connection, rev: stamp.rev });
    for (const m of inventory.monitors) {
      const key = { peer: stamp.peer, uid: m.uid };
      validarChave(key);
      registrarUnico(chaves, id(key));
      nativos.push({
        key, label: m.label, ...dimensoesVirtuais(m, inventory),
        space: inventory.space, source: inventory.source, x: m.rect.x, y: m.rect.y,
      });
    }
  }
  nativos.sort((a, b) => comparar(a.key.peer.toLowerCase(), b.key.peer.toLowerCase()) || a.x - b.x || a.y - b.y || comparar(a.key.uid.toLowerCase(), b.key.uid.toLowerCase()));
  const positions = (layout.organization?.positions ?? []).map(copiarPosicao);
  const salvos = new Set();
  for (const p of positions) {
    validarChave(p.key);
    validarPosicao(p);
    registrarUnico(salvos, id(p.key));
  }
  let x = positions.length ? Math.max(0, ...positions.map((p) => p.x + (nativos.find((m) => id(m.key) === id(p.key))?.width ?? 0))) + GAP_SEMENTE : 0;
  for (const m of nativos) {
    if (salvos.has(id(m.key))) continue;
    // As origens nativas só ordenam/alinham a semente; nunca são divididas pelo DPI.
    const minimoY = Math.min(...nativos.filter((n) => n.key.peer.toLowerCase() === m.key.peer.toLowerCase()).map((n) => n.y));
    const posicao = { key: { ...m.key }, x, y: m.y - minimoY };
    validarPosicao(posicao);
    positions.push(posicao);
    x += m.width + GAP_SEMENTE;
  }
  const replace_link_ids = (layout.organization?.links ?? []).map((link) => link.id);
  if (replace_link_ids.some((linkId) => typeof linkId !== "string" || !linkId.length)
      || new Set(replace_link_ids).size !== replace_link_ids.length) recusar("invalid_links");
  return comMonitores({
    base_rev: layout.rev, inventories, positions,
    replace_link_ids,
  }, nativos.map(({ x: _x, y: _y, ...m }) => m));
}

export function moverMonitor(rascunho, key, coordenadas) {
  const posicao = buscarPosicao(rascunho, key);
  validarPosicao(coordenadas);
  return comMonitores({
    base_rev: rascunho.base_rev,
    inventories: rascunho.inventories.map((stamp) => ({ ...stamp })),
    positions: rascunho.positions.map((p) => copiarPosicao(p === posicao ? { ...p, x: coordenadas.x, y: coordenadas.y } : p)),
    replace_link_ids: [...rascunho.replace_link_ids],
  }, rascunho.monitores ?? []);
}

export function encaixarMonitor(rascunho, key, alvo, lado) {
  buscarPosicao(rascunho, key);
  const destino = buscarPosicao(rascunho, alvo);
  if (id(key) === id(alvo)) recusar("identity_ambiguous");
  if (!["esquerda", "direita", "acima", "abaixo"].includes(lado)) recusar("invalid_side");
  const tela = rascunho.monitores?.find((m) => id(m.key) === id(key));
  const outra = rascunho.monitores?.find((m) => id(m.key) === id(alvo));
  if (!tela || !outra) recusar("invalid_geometry");
  let { x, y } = destino;
  if (lado === "esquerda") x -= tela.width;
  if (lado === "direita") x += outra.width;
  if (lado === "acima") y -= tela.height;
  if (lado === "abaixo") y += outra.height;
  return moverMonitor(rascunho, key, { x, y });
}
