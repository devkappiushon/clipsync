import assert from "node:assert/strict";
import { test } from "node:test";
import * as travessia from "../../ui/js/travessia.js";
import { posicionarDestino, proporLigacao } from "../../ui/js/travessia.js";

const monitor = (ref, x, y, width, height) => ({ ref, name: ref, index: 0, x, y, width, height, scale: 1 });
const mesa = (x = 9000, y = 9000) => ({
  self_id: "local", rev: 4, snap_px: 16,
  options: { dead_corner_px: 8 }, links: [], locked_edges: [],
  devices: [
    { peer_id: "local", is_self: true, state: "connected", x: 0, y: 0,
      monitors: [monitor("DELL", 0, 0, 3840, 2160), monitor("LG", 3840, 368, 1920, 1080)] },
    { peer_id: "notebook", is_self: false, state: "connected", x, y,
      monitors: [monitor("eDP", 0, 0, 1920, 1080)] },
  ],
});

// Regressão protegida: mover só o eixo pedido deixava o notebook diagonal,
// completamente fora do intervalo perpendicular do monitor de saída.
for (const [edge, eixo, limite] of [
  ["left", "y", 2160], ["right", "y", 2160],
  ["top", "x", 5760], ["bottom", "x", 5760],
]) {
  test(`posição ${edge} projeta notebook fora de eixo para um monitor real`, () => {
    const layout = mesa();
    const pos = posicionarDestino(layout, { peer_id: "notebook", edge });
    assert.ok(pos[eixo] < limite, `posição perpendicular ${pos[eixo]} ainda está além de ${limite}`);
    const alturaOuLargura = eixo === "x" ? 1920 : 1080;
    assert.ok(pos[eixo] + alturaOuLargura > 0);
    const final = structuredClone(layout);
    Object.assign(final.devices[1], pos);
    const proposta = proporLigacao(final, { peer_id: "notebook", edge });
    const monitorSaida = final.devices[0].monitors.find((m) => m.ref === proposta.a.monitor);
    const inicio = monitorSaida[eixo];
    const tamanho = eixo === "x" ? monitorSaida.width : monitorSaida.height;
    assert.ok(pos[eixo] < inicio + tamanho && pos[eixo] + alturaOuLargura > inicio,
      "notebook precisa alinhar com o monitor da borda proposta, além da caixa global");
  });
}

test("abaixo do LG mantém alinhamento perpendicular válido, sem centralizar na mesa", () => {
  const layout = mesa(4000, 3000);
  const pos = posicionarDestino(layout, { peer_id: "notebook", edge: "bottom" });
  assert.equal(pos.x, 4000);
  const final = structuredClone(layout);
  Object.assign(final.devices[1], pos);
  const proposta = proporLigacao(final, { peer_id: "notebook", edge: "bottom" });
  assert.equal(proposta.a.monitor, "LG");
  assert.equal(proposta.a.edge, "bottom");
  assert.equal(proposta.b.edge, "top");
});

test("posição e proposta preservam origem com coordenadas negativas e não mutam a entrada", () => {
  const layout = mesa(-1600, 3000);
  layout.devices[0].monitors = [monitor("esquerdo", -1920, 0, 1920, 1080), monitor("principal", 0, 0, 1920, 1080)];
  const antes = structuredClone(layout);
  const pos = posicionarDestino(layout, { peer_id: "notebook", edge: "bottom" });
  assert.equal(pos.x, -1600);
  const final = structuredClone(layout);
  Object.assign(final.devices[1], pos);
  assert.equal(proporLigacao(final, { peer_id: "notebook", edge: "bottom" }).a.monitor, "esquerdo");
  assert.deepEqual(layout, antes);
});

test("intervalo vazio entre monitores não conta como alinhamento perpendicular", () => {
  const layout = mesa(5000, 2200);
  layout.devices[0].monitors = [monitor("alto", 0, 0, 1920, 1080), monitor("baixo", 0, 5000, 1920, 1080)];
  const pos = posicionarDestino(layout, { peer_id: "notebook", edge: "right" });
  assert.ok(pos.y < 1080 || pos.y + 1080 > 5000,
    "a caixa global contém um vazio e não pode substituir o alinhamento com monitor real");
});

test("extremos da alça respeitam cantos mortos, outra alça e trecho bloqueado", () => {
  assert.equal(typeof travessia.limitesAlca, "function");
  const layout = mesa(4000, 3000);
  layout.options.dead_corner_px = 192;
  const ponta = { peer_id: "local", monitor: "LG", edge: "bottom", from: 0.3, to: 0.7 };
  layout.locked_edges = [{ ...ponta, from: 0, to: 0.25 }, { ...ponta, from: 0.8, to: 1 }];
  assert.deepEqual(travessia.limitesAlca(layout, ponta, "from"), { min: 0.25, max: 0.6375 });
  assert.deepEqual(travessia.limitesAlca(layout, ponta, "to"), { min: 0.3625, max: 0.8 });
  layout.locked_edges = [];
  assert.deepEqual(travessia.limitesAlca(layout, ponta, "from"), { min: 0.1, max: 0.6375 });
  assert.deepEqual(travessia.limitesAlca(layout, ponta, "to"), { min: 0.3625, max: 0.9 });
});
