import assert from "node:assert/strict";
import { test } from "node:test";
import { api } from "../../ui/js/api.js";
import { atualizarApp } from "../../ui/js/estado.js";
globalThis.document = { getElementById: () => ({}) };
const { default: montar } = await import("../../ui/js/telas/layout.js");
const { ofertaMonitores } = await import("../../ui/js/monitores.js");

// Fronteira DOM/foco, sem motor gráfico ou leitor: monta a tela completa,
// componentes, controlador, carregamento e assinaturas reais. Só a API é substituída.
function dom() {
  const focusListeners = [];
  const doc = { defaultView: { location: { href: "#/layout" } },
    addEventListener(type, fn) { if (type === "focusin") focusListeners.push(fn); },
    getElementById: () => ({}), createTextNode: (s) => String(s) };
  const walk = (n) => typeof n === "object" ? [n, ...n.children.flatMap(walk)] : [];
  const node = (tag) => {
    const attrs = new Map();
    const n = { tagName: tag.toUpperCase(), ownerDocument: doc, children: [], parentNode: null,
      dataset: {}, style: {}, listeners: {}, disabled: false, hidden: false, className: "", id: "",
      scrollTop: 0, scrollLeft: 0, clientWidth: 960, value: "", tabIndex: 0,
      get isConnected() { return this === doc.body || !!this.parentNode?.isConnected; },
      get textContent() { return this.children.map(c => typeof c === "object" ? c.textContent : c).join(""); },
      set textContent(s) { this.replaceChildren(String(s)); },
      append(...items) { for (const item of items) {
        if (typeof item === "object") { item.parentNode?.removeChild(item); item.parentNode = this; }
        this.children.push(item);
      } },
      removeChild(child) { this.children.splice(this.children.indexOf(child), 1); child.parentNode = null; },
      replaceChildren(...items) {
        if (this.contains(doc.activeElement)) doc.activeElement = doc.body;
        for (const c of this.children) if (typeof c === "object") c.parentNode = null;
        this.children = []; this.append(...items);
      },
      setAttribute(k, v) { attrs.set(k, String(v));
        if (k.startsWith("data-")) this.dataset[k.slice(5).replace(/-([a-z])/g, (_, c) => c.toUpperCase())] = String(v);
        if (k === "class") this.className = String(v);
        if (k === "id") this.id = String(v);
        if (k === "hidden") this.hidden = true;
      },
      getAttribute(k) { return attrs.get(k) ?? null; },
      removeAttribute(k) { attrs.delete(k); if (k === "hidden") this.hidden = false; },
      toggleAttribute(k, v) { if (v) this.setAttribute(k, ""); else this.removeAttribute(k); },
      addEventListener(k, fn) { this.listeners[k] = fn; },
      contains(other) { return walk(this).includes(other); },
      matches(selector) { return selector.split(",").some(part => {
        part = part.trim();
        if (part.includes(" ")) { const bits = part.split(/\s+/); return this.matches(bits.pop()) && !!this.parentNode?.closest(bits.join(" ")); }
        const attr = part.match(/\[([^=\]]+)(?:=["']?([^"'\]]+)["']?)?\]/);
        if (attr) {
          const prefix = part.slice(0, part.indexOf("["));
          if (prefix && !this.matches(prefix)) return false;
          const k = attr[1], val = k.startsWith("data-") ? this.dataset[k.slice(5).replace(/-([a-z])/g, (_, c) => c.toUpperCase())] : k === "id" ? this.id || null : k === "hidden" ? (this.hidden ? "" : null) : this.getAttribute(k);
          return attr[2] === undefined ? val != null : val === attr[2];
        }
        if (part.startsWith(".")) return this.className.split(/\s+/).includes(part.slice(1));
        return this.tagName.toLowerCase() === part.toLowerCase();
      }); },
      closest(selector) { return this.matches(selector) ? this : this.parentNode?.closest(selector) ?? null; },
      querySelectorAll(selector) { return walk(this).slice(1).filter(n => n.matches(selector)); },
      querySelector(selector) { return this.querySelectorAll(selector)[0] ?? null; },
      focus() { if (!this.isConnected || this.disabled) return; if (doc.activeElement === this) return;
        doc.activeElement = this; for (const fn of focusListeners) fn({ target: this }); },
      setPointerCapture() {}, releasePointerCapture() {},
    };
    return n;
  };
  doc.createElement = node; doc.createElementNS = (_, tag) => node(tag);
  doc.body = node("body"); doc.documentElement = node("html"); doc.activeElement = doc.body;
  doc.querySelector = s => doc.body.querySelector(s);
  return doc;
}
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

const settle = () => new Promise(resolve => setImmediate(resolve));
async function tela({ oferta = false, resposta, aplicar, nomeLongo = false } = {}) {
  const doc = dom(); globalThis.document = doc;
  globalThis.ResizeObserver = class { observe() {} disconnect() {} };
  const l = snapshot();
  l.options = { dead_corner_px: 0 }; l.snap_px = 16;
  for (const d of l.devices) {
    Object.assign(d, { x: 0, y: 0, state: "connected", monitors: [] });
    if (nomeLongo) d.name = "Computador completo com nome extenso " + "abcdefghij".repeat(20);
  }
  if (nomeLongo) l.inventories[0].inventory.monitors[0].label = "Monitor completo " + "ABCDEFGHIJ".repeat(20);
  if (oferta) l.pending_monitor_offer = { from_peer_id: peer(2), from_name: "notebook", connection: 8, rev: 3, positions: [], links: [], legacy_links: [] };
  const calls = [];
  api.appState = async () => ({ features: { input: true }, modules: { input: true }, input_supported: true });
  api.layoutGet = async () => structuredClone(l);
  api.layoutMonitorOfferAnswer = async args => {
    calls.push(["oferta", args]);
    if (resposta) await resposta(args);
    l.pending_monitor_offer = null;
    return { activation: "confirmed" };
  };
  api.layoutPreviewOrganization = async args => {
    calls.push(["previa", structuredClone(args)]);
    return { ...args, effective_links: [], preserved_link_ids: [], replaced_link_ids: [], unresolved_link_ids: [] };
  };
  api.layoutApplyOrganization = async args => {
    calls.push(["aplicar", structuredClone(args)]);
    return aplicar ? aplicar(args) : { layout_rev: 8, activation: "confirmed", durability: "confirmed" };
  };
  await atualizarApp();
  const events = new Map();
  const mounted = montar({ ir() {}, assinar(names, fn) { for (const n of [names].flat()) events.set(n, fn); return () => {}; } });
  doc.body.append(mounted.no); await settle();
  const find = k => mounted.no.querySelectorAll("[data-focus-key]").find(n => n.dataset.focusKey === k);
  assert.ok(find("monitores-prever"), "a tela completa precisa montar sem erro");
  const click = async k => { const n = find(k); assert.ok(n, k); assert.equal(n.disabled, false, k); return n.listeners.click({}); };
  return { ...mounted, doc, l, calls, events, find, click };
}
for (const caso of ["aceitar", "recusar", "stale_offer", "erro"]) {
  test(`Layout completo: oferta ${caso} termina livre e permite a próxima prévia`, async t => {
    const h = await tela({ oferta: true, resposta: async () => { if (["erro", "stale_offer"].includes(caso)) throw { code: caso, message: "Resposta falhou" }; } });
    t.after(h.desmontar);
    await h.click(caso === "recusar" ? "monitores-oferta-recusar" : "monitores-oferta-aceitar");
    assert.equal(h.no.querySelectorAll('[aria-busy="true"]').length, 0);
    assert.equal(h.find("monitores-prever").getAttribute("aria-disabled"), "false");
    await h.click("monitores-prever");
    assert.deepEqual(h.calls.map(c => c[0]), ["oferta", "previa"]);
    assert.equal(h.find("monitores-aplicar").disabled, false);
  });
}
for (const resultado of ["confirmed", "unconfirmed", "erro"]) {
  test(`Layout completo: Aplicar ${resultado} transfere foco para Conferir prévia`, async t => {
    const d = deferred(); const h = await tela({ aplicar: () => d.promise }); t.after(h.desmontar);
    await h.click("monitores-prever"); h.find("monitores-aplicar").focus();
    const pending = h.click("monitores-aplicar");
    if (resultado === "erro") d.reject({ code: "stale_inventory" });
    else d.resolve({ layout_rev: 8, activation: resultado, durability: "confirmed" });
    await pending;
    assert.equal(h.doc.activeElement.dataset?.focusKey, "monitores-prever");
    assert.equal(h.find("monitores-aplicar").disabled, true);
    await h.click("monitores-prever");
    assert.deepEqual(h.calls.map(c => c[0]), ["previa", "aplicar", "previa"]);
  });
}
for (const acao of ["oferta", "aplicar"]) for (const escolha of ["externo", "interno", "rota", "desmontar"]) {
  test(`resposta tardia de ${acao} respeita ${escolha}`, async t => {
    const d = deferred(); const h = await tela({ oferta: acao === "oferta", resposta: () => d.promise, aplicar: () => d.promise }); t.after(h.desmontar);
    if (acao === "aplicar") await h.click("monitores-prever");
    const key = acao === "oferta" ? "monitores-oferta-aceitar" : "monitores-aplicar";
    h.find(key).focus(); const pending = h.click(key);
    let chosen;
    if (escolha === "externo") { chosen = h.doc.createElement("button"); h.doc.body.append(chosen); chosen.focus(); }
    if (escolha === "interno") { chosen = h.find("monitores-atualizar"); chosen.focus(); }
    if (escolha === "rota") h.doc.defaultView.location.href = "#/outra";
    if (escolha === "desmontar") { h.desmontar(); h.doc.body.replaceChildren(); }
    d.resolve({ layout_rev: 8, activation: "confirmed", durability: "confirmed" }); await pending;
    if (escolha === "externo") assert.equal(h.doc.activeElement, chosen);
    if (escolha === "interno") assert.equal(h.doc.activeElement.dataset.focusKey, "monitores-atualizar");
    if (escolha === "rota") assert.notEqual(h.doc.activeElement.dataset?.focusKey, "monitores-prever");
    if (escolha === "desmontar") assert.equal(h.doc.activeElement, h.doc.body);
  });
}
test("cartões: Enter/Espaço selecionam coordenada sem gravar nem rolar", async t => {
  const h = await tela(); t.after(h.desmontar);
  for (const key of ["Enter", " "]) {
    const card = h.no.querySelector(".monitores__cartao"); card.focus(); let prevented = false;
    card.listeners.keydown({ key, preventDefault() { prevented = true; } });
    assert.equal(prevented, true);
    assert.equal(h.doc.activeElement.dataset.focusKey, card.dataset.focusKey + ":x");
    assert.deepEqual(h.calls, []);
  }
});
test("nomes completos ficam visíveis associados aos números, sem mudar retângulos", async t => {
  const h = await tela({ nomeLongo: true }); t.after(h.desmontar);
  const cards = h.no.querySelectorAll(".monitores__cartao");
  const legends = h.no.querySelectorAll("legend");
  for (const [i, card] of cards.entries()) {
    assert.match(legends[i].textContent, new RegExp(`Tela ${i + 1}`));
    assert.ok(legends[i].textContent.includes(h.l.devices.find(d => card.getAttribute("aria-label").includes(d.name)).name));
    assert.equal(card.querySelector("text").textContent, String(i + 1));
    assert.equal(card.querySelector("rect").getAttribute("width"), "1000");
    assert.equal(card.querySelector("rect").getAttribute("height"), "1000");
  }
  assert.ok(legends[0].textContent.includes(h.l.inventories[0].inventory.monitors[0].label));
});
test("regiões persistentes anunciam prévia/resultado e entradas inválidas sem perder chave", async t => {
  const h = await tela(); t.after(h.desmontar);
  const live = h.no.querySelector(".anunciador");
  const status = live.querySelector('[role="status"]'), alert = live.querySelector('[role="alert"]');
  await h.click("monitores-prever");
  assert.match(status.textContent, /[Pp]révia/);
  await h.click("monitores-aplicar");
  assert.match(status.textContent, /Organização salva/);
  for (const value of ["", "NaN", "1000001"]) {
    const x = h.no.querySelectorAll("[data-focus-key]").find(n => n.dataset.focusKey.endsWith(":x"));
    x.value = value; const mover = x.dataset.focusKey.slice(0, -1) + "mover";
    h.find(mover).focus(); await h.click(mover);
    assert.match(alert.textContent, /posições finitas/);
    assert.equal(h.doc.activeElement.dataset.focusKey, mover);
    assert.equal(h.no.querySelector(".anunciador"), live);
  }
  assert.equal(h.no.querySelector(".monitores").querySelectorAll('[role="alert"], [role="status"], [aria-live]').length, 0);
});
for (const legadas of [0, 1]) test(`oferta conta geridas uma vez com ${legadas} legadas`, () => {
  globalThis.document = dom();
  const l = snapshot(); l.links = [{ id: "gerida" }, ...Array.from({ length: legadas }, () => ({ id: "legada" }))];
  l.organization = { links: [{ id: "gerida" }] };
  l.pending_monitor_offer = { positions: [], links: [], legacy_links: [] };
  const no = ofertaMonitores({ layout: l, aoResponder() {} });
  assert.ok(no.textContent.includes(`${1 + legadas} ligações atuais serão substituídas.`));
});

for (const value of ["", "NaN", "1000001"]) test(`coordenada ${value || "vazia"} recebe feedback persistente`, async t => {
  const h = await tela(); t.after(h.desmontar);
  const x = h.no.querySelectorAll("[data-focus-key]").find(n => n.dataset.focusKey.endsWith(":x"));
  x.value = value; const key = x.dataset.focusKey.slice(0, -1) + "mover";
  h.find(key).focus(); await h.click(key);
  assert.match(h.no.querySelector(".anunciador").querySelector('[role="alert"]').textContent, /posições finitas/);
  assert.equal(h.doc.activeElement.dataset.focusKey, key);
  assert.deepEqual(h.calls, []);
});
test("Espaço no cartão impede rolagem e seleciona X", async t => {
  const h = await tela(); t.after(h.desmontar);
  const card = h.no.querySelector(".monitores__cartao"); let prevented = false; card.focus();
  card.listeners.keydown({ key: " ", preventDefault() { prevented = true; } });
  assert.equal(prevented, true);
  assert.equal(h.doc.activeElement.dataset.focusKey, card.dataset.focusKey + ":x");
  assert.deepEqual(h.calls, []);
});

for (const acao of ["aceitar", "recusar"]) {
  test(`Layout completo: prévia antiga preserva alerta de ${acao} e nova prévia pode anunciar`, async t => {
    const h = await tela({ oferta: true, resposta: async () => {
      throw { code: "erro", message: "Resposta falhou" };
    } });
    t.after(h.desmontar);
    const live = h.no.querySelector(".anunciador");
    const status = live.querySelector('[role="status"]');
    const alert = live.querySelector('[role="alert"]');
    await h.click("monitores-prever");
    assert.match(status.textContent, /Prévia disponível/);
    const key = `monitores-oferta-${acao}`;
    h.find(key).focus();
    await h.click(key);
    assert.equal(alert.textContent, "Resposta falhou", "finally não pode apagar a falha recente com a prévia antiga");
    assert.equal(status.textContent, "");
    assert.equal(h.doc.activeElement.dataset.focusKey, key);
    assert.equal(h.find("monitores-aplicar").disabled, false);
    for (let i = 0; i < 2; i++) {
      await h.events.get("layout_changed")();
      assert.equal(alert.textContent, "Resposta falhou", "recarga sem transição mantém o alerta");
      assert.equal(status.textContent, "");
      assert.equal(h.no.querySelector(".anunciador"), live);
    }
    await h.click("monitores-prever");
    assert.equal(alert.textContent, "", "nova ação válida pode substituir o erro");
    assert.match(status.textContent, /Prévia disponível/);
    // Repintar a mesma prévia não substitui sequer o conteúdo da região viva.
    const texto = status.children;
    await h.events.get("layout_changed")();
    assert.equal(status.children, texto);
    assert.deepEqual(h.calls.map(c => c[0]), ["previa", "oferta", "previa"]);
  });
}
