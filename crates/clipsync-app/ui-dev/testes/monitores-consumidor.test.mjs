import assert from "node:assert/strict";
import { test } from "node:test";
globalThis.document = { getElementById: () => ({}) };
const m = await import("../../ui/js/monitores.js").catch(() => ({}));
const peer = (n) => String(n).repeat(64),
  uid = (n) => String(n).repeat(32);
const snapshot = () => ({
  rev: 7,
  self_id: peer(1),
  devices: [
    { peer_id: peer(1), name: "desktop", is_self: true },
    { peer_id: peer(2), name: "notebook" }
  ],
  links: [],
  organization: null,
  inventories: [
    {
      stamp: { peer: peer(1), connection: 0, rev: 1 },
      inventory: {
        rev: 1,
        space: "Physical",
        source: "Observed",
        capabilities: { internal_edges: true, positioned_return: true, route_receiver: true },
        monitors: [
          { uid: uid(1), label: "A", rect: { x: 0, y: 0, width: 1000, height: 1000 }, scale: 1 },
          { uid: uid(2), label: "B", rect: { x: 1000, y: 0, width: 1000, height: 1000 }, scale: 1 }
        ]
      }
    },
    {
      stamp: { peer: peer(2), connection: 8, rev: 2 },
      inventory: {
        rev: 2,
        space: "Logical",
        source: "Observed",
        capabilities: { internal_edges: false, positioned_return: false, route_receiver: true },
        monitors: [
          { uid: uid(3), label: "notebook", rect: { x: 0, y: 0, width: 1000, height: 1000 }, scale: 1 }
        ]
      }
    }
  ]
});
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((a, b) => {
    resolve = a;
    reject = b;
  });
  return { promise, resolve, reject };
};
function harness() {
  const calls = [];
  let waiting = null;
  const api = {
    layoutPreviewOrganization: async (req) => {
      calls.push(["preview", structuredClone(req)]);
      return waiting
        ? waiting.promise
        : {
            base_rev: req.base_rev,
            inventories: req.inventories,
            positions: req.positions,
            effective_links: [{ id: "a" }, { id: "b" }],
            preserved_link_ids: [],
            replaced_link_ids: [],
            unresolved_link_ids: []
          };
    },
    layoutApplyOrganization: async (req) => {
      calls.push(["apply", structuredClone(req)]);
      return { layout_rev: 8, activation: "unconfirmed", durability: "confirmed" };
    }
  };
  assert.equal(typeof m.controladorMonitores, "function");
  const c = m.controladorMonitores({ layout: snapshot(), api, aoMudar() {} });
  return {
    c,
    calls,
    wait(v) {
      waiting = v;
    }
  };
}
test("abrir e mover são rascunho; aplicar envia exatamente a prévia uma vez", async () => {
  const { c, calls } = harness();
  assert.equal(c.estado().rascunho.positions.length, 3);
  c.mover({ peer: peer(1), uid: uid(1) }, { x: -1000, y: 0 });
  assert.equal(calls.length, 0);
  await c.prever();
  const shown = calls[0][1];
  await Promise.all([c.aplicar(), c.aplicar()]);
  assert.deepEqual(
    calls.map((x) => x[0]),
    ["preview", "apply"]
  );
  assert.deepEqual(calls[1][1], shown);
  assert.equal(c.estado().resultado.activation, "unconfirmed");
  await c.aplicar();
  assert.equal(calls.length, 2);
});
test("prévia tardia é ignorada e erro stale conserva posições até atualizar explicitamente", async () => {
  const h = harness(),
    d = deferred();
  h.wait(d);
  const original = h.c.estado().rascunho;
  const p = h.c.prever();
  h.c.mover(original.positions[0].key, { x: 200, y: 0 });
  d.resolve({ effective_links: [] });
  await p;
  assert.equal(h.c.estado().previa, null);
  const e = deferred();
  h.wait(e);
  const q = h.c.prever();
  e.reject({ code: "stale_inventory", message: "mudou" });
  await q;
  assert.equal(h.c.estado().rascunho.positions[0].x, 200);
  assert.equal(h.c.estado().erro.code, "stale_inventory");
  const newer = snapshot();
  newer.rev = 9;
  newer.inventories[0].stamp.rev = 4;
  h.c.atualizar(newer, true);
  assert.equal(h.c.estado().rascunho.positions[0].x, 200);
  assert.equal(h.c.estado().rascunho.base_rev, 9);
  assert.equal(h.calls.filter((x) => x[0] === "apply").length, 0);
});
test("vários destinos exigem escolha e request preserva somente o par selecionado", () => {
  assert.equal(typeof m.controladorMonitores, "function");
  const l = snapshot();
  const third = structuredClone(l.inventories[1]);
  third.stamp.peer = peer(3);
  l.inventories.push(third);
  l.organization = { positions: [{ key: { peer: peer(3), uid: uid(3) }, x: 7000, y: 0 }], links: [] };
  const c = m.controladorMonitores({ layout: l, api: {}, aoMudar() {} });
  assert.equal(c.estado().rascunho, null);
  c.selecionar(peer(2));
  assert.deepEqual(
    new Set(c.estado().rascunho.positions.map((p) => p.key.peer)),
    new Set([peer(1), peer(2)])
  );
  assert.equal(c.estado().layout.organization.positions[0].x, 7000);
});
test("snap de até 16 usa contato exato e cancel restaura o início sem persistir", () => {
  assert.equal(typeof m.gestosMonitor, "function");
  const { c, calls } = harness();
  const p = c.estado().rascunho.positions[0],
    events = {};
  const node = {
    addEventListener: (n, f) => (events[n] = f),
    setPointerCapture() {},
    releasePointerCapture() {}
  };
  m.gestosMonitor(node, {
    key: p.key,
    obter: () => c.estado().rascunho,
    aoMudar: (d) => c.alterar(d),
    escala: () => 1
  });
  events.pointerdown({ button: 0, pointerId: 1, clientX: 0, clientY: 0, preventDefault() {} });
  events.pointermove({ pointerId: 1, clientX: 17, clientY: 3 });
  events.pointercancel({ pointerId: 1 });
  assert.deepEqual(c.estado().rascunho.positions[0], p);
  events.keydown({ key: "ArrowRight", preventDefault() {} });
  assert.equal(c.estado().rascunho.positions[0].x, p.x + 16);
  assert.equal(calls.length, 0);
});

test("evento de layout durante apply não apaga prévia congelada nem posições de outro par", async () => {
  const l = snapshot(),
    third = { key: { peer: peer(3), uid: uid(4) }, x: 9000, y: 0 };
  l.organization = { positions: [third], links: [] };
  const waiting = deferred();
  const c = m.controladorMonitores({
    layout: l,
    api: {
      layoutPreviewOrganization: async (r) => ({
        ...r,
        effective_links: [{ id: "a" }],
        preserved_link_ids: [],
        replaced_link_ids: [],
        unresolved_link_ids: []
      }),
      layoutApplyOrganization: () => waiting.promise
    }
  });
  await c.prever();
  const applying = c.aplicar();
  c.atualizar({ ...l, rev: 8 });
  waiting.resolve({ layout_rev: 8, activation: "confirmed", durability: "confirmed" });
  await applying;
  assert.equal(c.estado().erro, null);
  assert.equal(c.estado().resultado.layout_rev, 8);
  assert.ok(c.estado().layout.organization.positions.some((p) => p.key.peer === peer(3) && p.x === 9000));
});
test("rótulo ausente é memória da sessão, sem fabricar dimensão", () => {
  const { c } = harness(),
    l = snapshot();
  l.inventories[0].inventory.monitors.pop();
  c.atualizar(l, true);
  assert.equal(c.estado().rotulos.get(`${peer(1)}:${uid(2)}`), "B");
  assert.ok(!c.estado().rascunho.monitores.some((m) => m.key.uid === uid(2)));
});

// Fronteira pequena de nós/foco, sem simular layout gráfico nem leitor de tela.
function focoDOM() {
  const listeners = [];
  const doc = {
    body: {},
    documentElement: {},
    defaultView: { location: { href: "#/layout" } },
    getElementById: () => ({}),
    createTextNode: (t) => t,
    addEventListener: (n, f) => {
      if (n === "focusin") listeners.push(f);
    },
    querySelector: () => null
  };
  const walk = (n) => (typeof n === "object" ? [n, ...(n.children ?? []).flatMap(walk)] : []);
  const node = (tag) => {
    const attrs = new Map();
    return {
      tag,
      ownerDocument: doc,
      isConnected: true,
      children: [],
      dataset: {},
      listeners: {},
      disabled: false,
      scrollTop: 0,
      scrollLeft: 0,
      append(...items) {
        this.children.push(...items);
      },
      setAttribute(k, v) {
        attrs.set(k, String(v));
      },
      getAttribute: (k) => attrs.get(k) ?? null,
      removeAttribute: (k) => attrs.delete(k),
      addEventListener(k, f) {
        this.listeners[k] = f;
      },
      contains(n) {
        return walk(this).includes(n);
      },
      querySelectorAll(selector) {
        return selector === "[data-focus-key]" ? walk(this).filter((n) => n.dataset?.focusKey) : [];
      },
      replaceChildren(...children) {
        for (const n of walk(this).slice(1)) n.isConnected = false;
        if (this.contains(doc.activeElement)) doc.activeElement = doc.body;
        this.children = children;
      },
      focus() {
        doc.activeElement = this;
        listeners.forEach((f) => f({ target: this }));
      }
    };
  };
  doc.createElement = node;
  doc.createElementNS = (_ns, tag) => node(tag);
  return { doc, node, walk };
}
test("repintura busy conserva o botão focável e resposta tardia não rouba o foco escolhido", async () => {
  const { doc, node, walk } = focoDOM();
  globalThis.document = doc;
  const { trocar } = await import("../../ui/js/dom.js");
  const root = node("main"),
    wait = deferred();
  let c;
  const paint = () =>
    trocar(
      root,
      m.editorMonitores({ ...c.estado(), aoMudar: c.alterar, aoPrever: c.prever, aoAplicar: c.aplicar })
    );
  c = m.controladorMonitores({
    layout: snapshot(),
    aoMudar: paint,
    api: { layoutPreviewOrganization: () => wait.promise }
  });
  paint();
  const button = walk(root).find((n) => n.dataset?.focusKey === "monitores-prever");
  button.focus();
  const task = button.listeners.click({});
  const busy = walk(root).find((n) => n.dataset?.focusKey === "monitores-prever");
  assert.equal(doc.activeElement, busy);
  assert.equal(busy.disabled, false);
  assert.equal(busy.getAttribute("aria-disabled"), "true");
  const other = node("button");
  other.focus();
  wait.resolve({
    effective_links: [],
    preserved_link_ids: [],
    replaced_link_ids: [],
    unresolved_link_ids: []
  });
  await task;
  assert.equal(doc.activeElement, other);
});

test("gesto fora do canvas mantém rascunho e apresenta erro sem exceção solta", () => {
  const { c, calls } = harness(),
    draft = c.estado().rascunho,
    key = draft.positions[0].key,
    events = {},
    errors = [];
  const node = {
    addEventListener: (k, f) => (events[k] = f),
    setPointerCapture() {},
    releasePointerCapture() {}
  };
  m.gestosMonitor(node, {
    key,
    obter: () => c.estado().rascunho,
    aoMudar: c.alterar,
    aoErro: (e) => errors.push(e.code)
  });
  events.pointerdown({ button: 0, pointerId: 1, clientX: 0, clientY: 0, preventDefault() {} });
  events.pointermove({ pointerId: 1, clientX: 2000000, clientY: 0 });
  events.pointerup({ pointerId: 1 });
  assert.deepEqual(errors, ["invalid_position"]);
  assert.equal(c.estado().rascunho, draft);
  assert.equal(calls.length, 0);
});
