import assert from "node:assert/strict";
import { test } from "node:test";
import * as modelo from "../../ui/js/monitores-geometria.js";

const desktop = "a".repeat(64);
const notebook = "b".repeat(64);
const A = { peer: desktop, uid: "1".repeat(32) };
const B = { peer: desktop, uid: "2".repeat(32) };
const N = { peer: notebook, uid: "3".repeat(32) };
const ausente = { peer: desktop, uid: "4".repeat(32) };
const desconhecida = { peer: desktop, uid: "5".repeat(32) };
const monitor = (key, label, rect, scale) => ({ uid: key.uid, label, rect, scale });
const inventario = (peer, monitors, space = "Physical", source = "Observed") => ({
  stamp: { peer, connection: 7, rev: 9 },
  inventory: {
    rev: 9, space, source,
    capabilities: { internal_edges: true, positioned_return: true, route_receiver: true },
    monitors,
  },
});
const mesa = () => ({
  rev: 42,
  inventories: [
    inventario(desktop, [
      monitor(B, "B", { x: 0, y: 120, width: 1920, height: 1080 }, 1),
      monitor(A, "A", { x: -3840, y: -120, width: 3840, height: 2160 }, 2),
    ]),
    inventario(notebook, [monitor(N, "notebook", { x: -200, y: -100, width: 1200, height: 800 }, 1.5)], "Logical"),
  ],
});
const posicao = (rascunho, key) => rascunho.positions.find((p) => p.key.peer === key.peer && p.key.uid === key.uid);
const falha = (fn, code) => assert.throws(fn, (erro) => erro instanceof Error && erro.code === code);
function congelar(valor) {
  if (valor && typeof valor === "object") {
    Object.values(valor).forEach(congelar);
    Object.freeze(valor);
  }
  return valor;
}

// Quebra protegida: tratar DPI como escala da origem ou criar adjacências na semente.
test("v1 semeia A/B/notebook por ordem nativa com espaço e dimensões virtuais", () => {
  const layout = congelar(mesa());
  const draft = modelo.criarRascunho(layout);
  assert.deepEqual(draft.positions, [
    { key: A, x: 0, y: 0 },
    { key: B, x: 1952, y: 240 },
    { key: N, x: 3904, y: 0 },
  ]);
  assert.deepEqual(draft.monitores.map(({ key, label, width, height, space, source }) => ({ key, label, width, height, space, source })), [
    { key: A, label: "A", width: 1920, height: 1080, space: "Physical", source: "Observed" },
    { key: B, label: "B", width: 1920, height: 1080, space: "Physical", source: "Observed" },
    { key: N, label: "notebook", width: 1200, height: 800, space: "Logical", source: "Observed" },
  ]);
  assert.equal(draft.base_rev, 42);
  assert.deepEqual(draft.inventories, [
    { peer: desktop, connection: 7, rev: 9 },
    { peer: notebook, connection: 7, rev: 9 },
  ]);
  assert.deepEqual(draft.replace_link_ids, []);
  assert.equal(Object.isFrozen(draft.monitores[0].key), true);
});

// Quebra protegida: descartar posição ausente, reutilizar referências ou incluir metadata no request.
test("v2 mantém posições soberanas, stamps e apenas IDs geridos num payload de quatro campos", () => {
  const layout = mesa();
  layout.organization = {
    positions: [{ key: A, x: -4500, y: 17 }, { key: B, x: 3100, y: -3 }, { key: N, x: 0, y: 17 }, { key: ausente, x: -7000, y: 30 }],
    links: [{ id: "gerida-ida" }, { id: "legado-adotado" }],
  };
  layout.links = [{ id: "avancada-preservada" }];
  const antes = structuredClone(layout);
  congelar(layout);
  const draft = modelo.criarRascunho(layout);
  assert.deepEqual(draft.positions, antes.organization.positions);
  assert.notEqual(draft.positions[0].key, layout.organization.positions[0].key);
  assert.notEqual(draft.inventories[0], layout.inventories[0].stamp);
  assert.deepEqual(draft.replace_link_ids, ["gerida-ida", "legado-adotado"]);
  assert.deepEqual(Object.keys(draft), ["base_rev", "inventories", "positions", "replace_link_ids"]);
  assert.deepEqual(JSON.parse(JSON.stringify(draft)), {
    base_rev: 42,
    inventories: [{ peer: desktop, connection: 7, rev: 9 }, { peer: notebook, connection: 7, rev: 9 }],
    positions: antes.organization.positions,
    replace_link_ids: ["gerida-ida", "legado-adotado"],
  });
  assert.deepEqual(layout, antes);
});

test("v1 sem inventário não fabrica monitores a partir dos devices legados", () => {
  assert.deepEqual(modelo.criarRascunho({ rev: 0, devices: [{ peer_id: desktop, monitors: [{ index: 0 }] }] }), {
    base_rev: 0, inventories: [], positions: [], replace_link_ids: [],
  });
});

test("estimativa continua estimativa no rascunho e não adiciona autorização de aplicar", () => {
  const layout = mesa();
  layout.inventories[0].inventory.source = "Estimated";
  const draft = modelo.criarRascunho(congelar(layout));
  assert.equal(draft.monitores[0].source, "Estimated");
  assert.equal(draft.monitores[1].source, "Estimated");
  assert.equal(draft.monitores[2].source, "Observed");
  assert.deepEqual(Object.keys(draft), ["base_rev", "inventories", "positions", "replace_link_ids"]);
});

// Quebra protegida: mover o grupo do peer em vez de uma tela ou obter novos stamps.
test("mover A mantém B do mesmo peer, notebook, ausente e snapshot anterior", () => {
  const layout = mesa();
  layout.organization = {
    positions: [{ key: A, x: 0, y: 0 }, { key: N, x: 1920, y: 0 }, { key: B, x: 3120, y: 0 }, { key: ausente, x: -50, y: 40 }],
    links: [{ id: "ida" }, { id: "volta" }],
  };
  const draft = modelo.criarRascunho(layout);
  const antes = structuredClone(draft);
  congelar(draft);
  const novo = modelo.moverMonitor(draft, congelar(A), congelar({ x: -12.5, y: 19.75 }));
  assert.deepEqual(novo.positions, [
    { key: A, x: -12.5, y: 19.75 }, { key: N, x: 1920, y: 0 },
    { key: B, x: 3120, y: 0 }, { key: ausente, x: -50, y: 40 },
  ]);
  assert.deepEqual(draft, antes);
  assert.deepEqual(novo.inventories, antes.inventories);
  assert.deepEqual(novo.replace_link_ids, ["ida", "volta"]);
  assert.equal(novo.base_rev, 42);
  assert.notEqual(novo.inventories[0], draft.inventories[0]);
  assert.notEqual(novo.positions[1].key, draft.positions[1].key);
  assert.notEqual(novo.monitores, draft.monitores);
  assert.deepEqual(novo.monitores, draft.monitores);
  assert.deepEqual(Object.keys(novo), ["base_rev", "inventories", "positions", "replace_link_ids"]);
});

test("mover não faz snap implícito e admite as duas fronteiras do canvas", () => {
  let draft = modelo.criarRascunho(mesa());
  draft = modelo.moverMonitor(draft, A, { x: 1930, y: 0.125 });
  assert.deepEqual(posicao(draft, A), { key: A, x: 1930, y: 0.125 });
  draft = modelo.moverMonitor(draft, A, { x: -1_000_000, y: 1_000_000 });
  assert.deepEqual(posicao(draft, A), { key: A, x: -1_000_000, y: 1_000_000 });
});

for (const [lado, esperado] of [
  ["esquerda", { x: -2500, y: -200 }],
  ["direita", { x: 620, y: -200 }],
  ["acima", { x: -1300, y: -1000 }],
  ["abaixo", { x: -1300, y: 880 }],
]) {
  // Quebra protegida: usar tamanho físico, escala do Logical ou caixa do peer.
  test(`encaixe ${lado} toca A exatamente e alinha notebook no eixo perpendicular`, () => {
    const inicial = modelo.criarRascunho(mesa());
    const draft = modelo.moverMonitor(inicial, A, { x: -1300, y: -200 });
    const antes = structuredClone(draft);
    congelar(draft);
    const novo = modelo.encaixarMonitor(draft, N, A, lado);
    assert.deepEqual(posicao(novo, N), { key: N, ...esperado });
    assert.deepEqual(posicao(novo, A), { key: A, x: -1300, y: -200 });
    assert.deepEqual(posicao(novo, B), { key: B, x: 1952, y: 240 });
    assert.deepEqual(novo.inventories, antes.inventories);
    assert.deepEqual(novo.replace_link_ids, antes.replace_link_ids);
    assert.deepEqual(draft, antes);
  });
}

test("encaixe usa também a dimensão virtual da tela Physical que está sendo movida", () => {
  const draft = modelo.moverMonitor(modelo.criarRascunho(mesa()), N, { x: -500, y: 70 });
  assert.deepEqual(posicao(modelo.encaixarMonitor(draft, A, N, "esquerda"), A), { key: A, x: -2420, y: 70 });
  assert.deepEqual(posicao(modelo.encaixarMonitor(draft, A, N, "acima"), A), { key: A, x: -500, y: -1010 });
});

test("mover posição salva ausente é possível mas encaixar sem dimensões é recusado", () => {
  const layout = mesa();
  layout.organization = { positions: [{ key: ausente, x: -3000, y: 10 }], links: [] };
  const draft = modelo.criarRascunho(layout);
  const antes = structuredClone(draft);
  const novo = modelo.moverMonitor(draft, ausente, { x: -2999, y: 11 });
  assert.deepEqual(posicao(novo, ausente), { key: ausente, x: -2999, y: 11 });
  falha(() => modelo.encaixarMonitor(draft, ausente, A, "direita"), "invalid_geometry");
  falha(() => modelo.encaixarMonitor(draft, A, ausente, "direita"), "invalid_geometry");
  assert.deepEqual(draft, antes);
});

test("chave desconhecida ou malformada nunca escolhe a primeira tela", () => {
  const draft = modelo.criarRascunho(mesa());
  const antes = structuredClone(draft);
  falha(() => modelo.moverMonitor(draft, desconhecida, { x: 0, y: 0 }), "monitor_missing");
  falha(() => modelo.encaixarMonitor(draft, A, desconhecida, "direita"), "monitor_missing");
  for (const key of [null, { peer: desktop }, { peer: "local", uid: A.uid }, { peer: desktop, uid: "0".repeat(32) }, { peer: desktop, uid: "g".repeat(32) }, { peer: desktop, uid: "a".repeat(31) }]) {
    falha(() => modelo.moverMonitor(draft, key, { x: 0, y: 0 }), "invalid_key");
    falha(() => modelo.encaixarMonitor(draft, A, key, "direita"), "invalid_key");
  }
  falha(() => modelo.encaixarMonitor(draft, A, A, "direita"), "identity_ambiguous");
  falha(() => modelo.encaixarMonitor(draft, A, B, "diagonal"), "invalid_side");
  assert.deepEqual(draft, antes);
});

test("coordenadas inválidas não sofrem coerção, clamp ou substituição por zero", () => {
  const draft = modelo.criarRascunho(mesa());
  const antes = structuredClone(draft);
  for (const valor of [NaN, Infinity, -Infinity, 1_000_000.01, -1_000_000.01, "12", null, undefined]) {
    falha(() => modelo.moverMonitor(draft, A, { x: valor, y: 0 }), "invalid_position");
    falha(() => modelo.moverMonitor(draft, A, { x: 0, y: valor }), "invalid_position");
  }
  const limite = modelo.moverMonitor(draft, A, { x: 1_000_000, y: -1_000_000 });
  falha(() => modelo.encaixarMonitor(limite, N, A, "direita"), "invalid_position");
  falha(() => modelo.encaixarMonitor(limite, N, A, "acima"), "invalid_position");
  assert.deepEqual(draft, antes);
});

// Quebra protegida: arredondar rev/connection u64 em vez de recusar perda de precisão.
test("revisões e stamps fora dos inteiros seguros JS são recusados", () => {
  for (const campo of ["rev", "connection", "stamp_rev", "inventory_rev"]) {
    for (const valor of [Number.MAX_SAFE_INTEGER + 1, -1, 1.25, Infinity, NaN, "9", 9n]) {
      const layout = mesa();
      if (campo === "rev") layout.rev = valor;
      if (campo === "connection") layout.inventories[0].stamp.connection = valor;
      if (campo === "stamp_rev") layout.inventories[0].stamp.rev = valor;
      if (campo === "inventory_rev") layout.inventories[0].inventory.rev = valor;
      falha(() => modelo.criarRascunho(congelar(layout)), "invalid_revision");
    }
  }
  const layout = mesa();
  layout.rev = Number.MAX_SAFE_INTEGER;
  layout.inventories[0].stamp.connection = Number.MAX_SAFE_INTEGER;
  layout.inventories[0].stamp.rev = Number.MAX_SAFE_INTEGER;
  layout.inventories[0].inventory.rev = Number.MAX_SAFE_INTEGER;
  const draft = modelo.criarRascunho(layout);
  assert.equal(draft.base_rev, 9_007_199_254_740_991);
  assert.deepEqual(draft.inventories[0], { peer: desktop, connection: 9_007_199_254_740_991, rev: 9_007_199_254_740_991 });
});

test("inventário ou posição com chave inválida ou repetida não gera seleção ambígua", () => {
  for (const modificar of [
    (l) => { l.inventories[0].inventory.monitors[0].uid = "0".repeat(32); },
    (l) => { l.inventories[0].stamp.peer = "peer inválido"; },
    (l) => { l.organization = { positions: [{ key: { peer: desktop, uid: "curto" }, x: 0, y: 0 }], links: [] }; },
  ]) {
    const layout = mesa();
    modificar(layout);
    falha(() => modelo.criarRascunho(congelar(layout)), "invalid_key");
  }
  for (const modificar of [
    (l) => { l.inventories[0].inventory.monitors.push({ ...l.inventories[0].inventory.monitors[0] }); },
    (l) => { l.inventories.push(structuredClone(l.inventories[0])); },
    (l) => { l.organization = { positions: [{ key: A, x: 0, y: 0 }, { key: A, x: 10, y: 10 }], links: [] }; },
    (l) => { l.inventories.push({ ...structuredClone(l.inventories[0]), stamp: { peer: desktop.toUpperCase(), connection: 7, rev: 9 } }); },
  ]) {
    const layout = mesa();
    modificar(layout);
    falha(() => modelo.criarRascunho(congelar(layout)), "identity_ambiguous");
  }
  const draft = modelo.criarRascunho(mesa());
  draft.positions.push({ key: A, x: 0, y: 0 });
  falha(() => modelo.moverMonitor(draft, A, { x: 10, y: 0 }), "identity_ambiguous");
});

test("geometria inválida não cria posição nem dimensão suposta", () => {
  for (const alterar of [
    (m) => { m.rect.width = 0; }, (m) => { m.rect.height = -1; },
    (m) => { m.rect.width = Infinity; }, (m) => { m.rect.height = NaN; },
    (m) => { m.rect.x = NaN; }, (m) => { m.rect.y = -Infinity; },
    (m) => { m.rect.x = 0.5; }, (m) => { m.rect.width = 1.5; },
    (m) => { m.rect.x = 2_147_483_647; },
    (m) => { m.scale = 0; }, (m) => { m.scale = NaN; },
    (m) => { m.scale = Infinity; }, (m) => { m.scale = -2; },
    (m) => { m.scale = Number.MIN_VALUE; },
  ]) {
    const layout = mesa();
    alterar(layout.inventories[0].inventory.monitors[0]);
    falha(() => modelo.criarRascunho(congelar(layout)), "invalid_geometry");
  }
  for (const campo of ["space", "source"]) {
    const layout = mesa();
    layout.inventories[0].inventory[campo] = "desconhecido";
    falha(() => modelo.criarRascunho(layout), "invalid_geometry");
  }
});

test("posições salvas e semente fora do canvas são recusadas sem clamp", () => {
  for (const valor of [NaN, Infinity, -1_000_001, 1_000_001]) {
    const layout = mesa();
    layout.organization = { positions: [{ key: ausente, x: valor, y: 0 }], links: [] };
    falha(() => modelo.criarRascunho(congelar(layout)), "invalid_position");
  }
  const layout = mesa();
  layout.organization = { positions: [{ key: A, x: 1_000_000, y: 0 }], links: [] };
  falha(() => modelo.criarRascunho(congelar(layout)), "invalid_position");
});

test("novo monitor recebe semente separada sem mover posições existentes", () => {
  const layout = mesa();
  layout.organization = { positions: [{ key: A, x: -50, y: 90 }, { key: N, x: 2000, y: 80 }], links: [] };
  const draft = modelo.criarRascunho(layout);
  assert.deepEqual(draft.positions, [
    { key: A, x: -50, y: 90 }, { key: N, x: 2000, y: 80 }, { key: B, x: 3232, y: 240 },
  ]);
  const reordenado = mesa();
  reordenado.inventories.reverse();
  reordenado.inventories[1].inventory.monitors.reverse();
  assert.deepEqual(modelo.criarRascunho(reordenado).positions, modelo.criarRascunho(mesa()).positions);
});

test("perder metadata via spread/JSON não permite encaixe com dimensão de outra tela", () => {
  const draft = modelo.criarRascunho(mesa());
  for (const copia of [{ ...draft }, JSON.parse(JSON.stringify(draft))]) {
    falha(() => modelo.encaixarMonitor(copia, A, N, "direita"), "invalid_geometry");
  }
});

test("IDs de links geridos duplicados são recusados sem ampliar o conjunto a substituir", () => {
  const layout = mesa();
  layout.organization = { positions: [], links: [{ id: "duplicado" }, { id: "duplicado" }] };
  falha(() => modelo.criarRascunho(congelar(layout)), "invalid_links");
});
