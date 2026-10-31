import assert from "node:assert/strict";
import { test } from "node:test";
import { spawnSync } from "node:child_process";
function element(tag) {
  const attrs = new Map();
  return {
    tag,
    children: [],
    dataset: {},
    listeners: {},
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
    }
  };
}
globalThis.document = { getElementById: () => ({}), createElement: element, createTextNode: (t) => t };
const { ofertaMonitores } = await import("../../ui/js/monitores.js");
const peer = "1".repeat(64),
  uid = "2".repeat(32);
test("oferta apresenta posições como texto e só responde ao clique explícito com o snapshot visto", async () => {
  const calls = [];
  const layout = {
    rev: 7,
    devices: [{ peer_id: peer, name: "notebook" }],
    pending_monitor_offer: {
      from_peer_id: peer,
      from_name: "<script>nome</script>",
      connection: 42,
      rev: 9,
      positions: [{ key: { peer, uid }, x: 1000, y: -20 }],
      links: [],
      legacy_links: []
    }
  };
  const root = ofertaMonitores({
    layout,
    aoResponder: (a) => {
      calls.push(a);
    }
  });
  assert.equal(calls.length, 0);
  const all = (n) => (typeof n === "object" ? [n, ...n.children.flatMap(all)] : [n]);
  const nodes = all(root);
  assert.ok(nodes.includes("<script>nome</script> ofereceu uma organização"));
  assert.ok(nodes.some((n) => typeof n === "string" && n.includes("X 1000, Y -20")));
  const accept = nodes.find((n) => n?.dataset?.focusKey === "monitores-oferta-aceitar");
  layout.rev = 99; // A resposta continua vinculada à revisão apresentada.
  await accept.listeners.click({});
  assert.deepEqual(calls, [{ from_peer_id: peer, connection: 42, rev: 9, base_rev: 7, accept: true }]);
});
for (const cenario of ["monitores-independentes", "protocolo-v3", "organizacao-nao-confirmada"])
  test(`simulador ${cenario} mantém transação única e compatibilidade`, () => {
    const source = `globalThis.location={search:'?cenario=${cenario}',protocol:'http:',hostname:'127.0.0.1'};const {invoke}=await import('./crates/clipsync-app/ui-dev/mock.js');const call=(cmd,args)=>invoke('clipsync',{req:{cmd,args}});const l=await call('layout_get');const {criarRascunho}=await import('./crates/clipsync-app/ui/js/monitores-geometria.js');const r=criarRascunho(l);try {const p=await call('layout_preview_organization',r);const afterPreview=await call('layout_get');if(afterPreview.rev!==l.rev)throw Error('prévia gravou');const a=await call('layout_apply_organization',r);if(a.layout_rev!==l.rev+1)throw Error('revisão incorreta');if('${cenario}'==='organizacao-nao-confirmada'&&a.activation!=='unconfirmed')throw Error('ativação inventada');console.log('aplicou');}catch(e){if('${cenario}'!=='protocolo-v3'||e.code!=='protocol_required')throw e;console.log('protocolo recusado');}`;
    const r = spawnSync(process.execPath, ["--input-type=module", "-e", source], { encoding: "utf8" });
    assert.equal(r.status, 0, r.stderr);
    assert.match(r.stdout, cenario === "protocolo-v3" ? /protocolo recusado/ : /aplicou/);
  });

for (const [cenario, expected] of [
  ["monitor-ausente", "monitor_missing"],
  ["monitor-ambiguo", "identity_ambiguous"],
  ["geometria-estimada", "inventory_required"],
  ["inventario-alterado", "stale_inventory"]
]) {
  test(`simulador ${cenario} conserva a revisão e apresenta causa própria`, () => {
    const source = `globalThis.location={search:'?cenario=${cenario}',protocol:'http:',hostname:'127.0.0.1'};const {invoke}=await import('./crates/clipsync-app/ui-dev/mock.js');const call=(cmd,args)=>invoke('clipsync',{req:{cmd,args}});const l=await call('layout_get');const {criarRascunho}=await import('./crates/clipsync-app/ui/js/monitores-geometria.js');try{await call('layout_apply_organization',criarRascunho(l));throw Error('aplicação indevida');}catch(e){if(e.code!=='${expected}')throw e;}const after=await call('layout_get');if(after.rev!==l.rev)throw Error('revisão mudou');`;
    const r = spawnSync(process.execPath, ["--input-type=module", "-e", source], { encoding: "utf8" });
    assert.equal(r.status, 0, r.stderr);
  });
}
test("oferta DEV conserva mapa completo e recusa não grava", () => {
  const source = `globalThis.location={search:'?cenario=oferta-monitores',protocol:'http:',hostname:'127.0.0.1'};const {invoke}=await import('./crates/clipsync-app/ui-dev/mock.js');const call=(cmd,args)=>invoke('clipsync',{req:{cmd,args}});const l=await call('layout_get'),o=l.pending_monitor_offer;if(o.positions.length!==3||o.links.length!==2)throw Error('oferta incompleta');const r=await call('layout_monitor_offer_answer',{from_peer_id:o.from_peer_id,connection:o.connection,rev:o.rev,base_rev:l.rev,accept:false});if(r.layout_rev!==l.rev||r.activation==='confirmed')throw Error('recusa incorreta');`;
  const r = spawnSync(process.execPath, ["--input-type=module", "-e", source], { encoding: "utf8" });
  assert.equal(r.status, 0, r.stderr);
});
