import assert from "node:assert/strict";
import { test } from "node:test";
import { ligacoesLocais, proporLigacao, posicaoRelativa, posicionarDestino, resultadoDoTeste, falhaAoLigar } from "../../ui/js/travessia.js";

const monitor = (ref, x = 0, y = 0) => ({ ref, name: ref, index: 0, x, y, width: 1920, height: 1080, scale: 1 });
const ponta = (peer_id, monitor, edge, from = 0, to = 1) => ({ peer_id, monitor, edge, from, to });
const layout = () => ({ self_id: "local", rev: 7, options: { dead_corner_px: 0 }, locked_edges: [], links: [], devices: [
  { peer_id: "local", is_self: true, state: "connected", x: 0, y: 0, monitors: [monitor("M1"), monitor("M2", 1920)] },
  { peer_id: "destino", is_self: false, state: "connected", x: 4000, y: 0, monitors: [monitor("eDP-1")] },
  { peer_id: "outro", is_self: false, state: "connected", x: 0, y: 2000, monitors: [monitor("HDMI-1")] },
] });

test("ligações só entre remotos não habilitam a saída local", () => {
  const l = layout();
  l.links = [{ id: "remota", a: ponta("destino", "eDP-1", "bottom"), b: ponta("outro", "HDMI-1", "top") }];
  assert.equal(ligacoesLocais(l).length, 0);
});

test("posição à direita escolhe o monitor externo real sem mudar o layout", () => {
  const l = layout();
  const antes = structuredClone(l);
  assert.deepEqual(proporLigacao(l, { peer_id: "destino", edge: "right" }), {
    id: null, a: ponta("local", "M2", "right"), b: ponta("destino", "eDP-1", "left"),
  });
  assert.deepEqual(l, antes);
});

test("reposicionar troca a única ligação desse par em vez de acumular ligações", () => {
  const l = layout();
  l.links = [{ id: "l1", a: ponta("destino", "eDP-1", "left"), b: ponta("local", "M2", "right") }];
  const nova = proporLigacao(l, { peer_id: "destino", edge: "left" });
  assert.equal(nova.id, "l1");
  assert.equal(nova.a.monitor, "M1");
  assert.equal(nova.a.edge, "left");
  assert.equal(nova.b.edge, "right");
});

test("múltiplas ligações avançadas não são substituídas pelo modo simples", () => {
  const l = layout();
  l.links = [
    { id: "l1", a: ponta("local", "M2", "right", 0, 0.5), b: ponta("destino", "eDP-1", "left", 0, 0.5) },
    { id: "l2", a: ponta("local", "M2", "right", 0.5, 1), b: ponta("destino", "eDP-1", "left", 0.5, 1) },
  ];
  assert.equal(proporLigacao(l, { peer_id: "destino", edge: "bottom" }), null);
});

test("sugestão recorta trechos internos e ocupados sem sobrepor", () => {
  const l = layout();
  l.locked_edges = [ponta("local", "M2", "right", 0, 0.25)];
  l.links = [{ id: "outro", a: ponta("local", "M2", "right", 0.5, 1), b: ponta("outro", "HDMI-1", "left") }];
  const nova = proporLigacao(l, { peer_id: "destino", edge: "right" });
  assert.deepEqual(nova.a, ponta("local", "M2", "right", 0.25, 0.5));
});

test("sem monitor real, lado válido ou trecho livre não inventa uma ligação", () => {
  const l = layout();
  assert.equal(proporLigacao(l, { peer_id: "destino", edge: "diagonal" }), null);
  assert.equal(proporLigacao(l, { peer_id: "local", edge: "right" }), null);
  l.devices[1].monitors = [];
  assert.equal(proporLigacao(l, { peer_id: "destino", edge: "right" }), null);
  l.devices[1].monitors = [monitor("eDP-1")];
  l.locked_edges = [ponta("local", "M1", "right"), ponta("local", "M2", "right")];
  assert.equal(proporLigacao(l, { peer_id: "destino", edge: "right" }), null);
});

test("arrasto resolve a posição relativa pelas caixas reais dos computadores", () => {
  const l = layout();
  assert.equal(posicaoRelativa(l, "destino"), "right");
  l.devices[1].x = 960;
  l.devices[1].y = 2000;
  assert.equal(posicaoRelativa(l, "destino"), "bottom");
  l.devices[1].y = -2000;
  assert.equal(posicaoRelativa(l, "destino"), "top");
});

test("zero amostras nunca é sucesso e perdas continuam visíveis", () => {
  assert.equal(resultadoDoTeste({ received: 0, lost: 0, out_of_order: 0 }).tom, "aviso");
  assert.equal(resultadoDoTeste({ received: 5, lost: 1, out_of_order: 0 }).tom, "aviso");
  assert.equal(resultadoDoTeste({ received: 5, lost: 0, out_of_order: 1 }).tom, "aviso");
  const recebeu = resultadoDoTeste({ received: 5, lost: 0, out_of_order: 0 });
  assert.equal(recebeu.tom, "info");
  assert.match(recebeu.texto, /volta/);
});

test("abaixo do segundo monitor preserva o alinhamento escolhido pelo arrasto", () => {
  const l = layout();
  l.devices[1].x = 1920;
  l.devices[1].y = 1200;
  assert.equal(proporLigacao(l, { peer_id: "destino", edge: "bottom" }).a.monitor, "M2");
});

test("coordenadas negativas e escala não trocam o monitor externo", () => {
  const l = layout();
  l.devices[0].monitors[0].x = -1920;
  l.devices[0].monitors[1].x = 0;
  l.devices[0].monitors[1].scale = 1.5;
  assert.equal(proporLigacao(l, { peer_id: "destino", edge: "right" }).a.monitor, "M2");
});

test("repetir a posição atual reutiliza a ligação e os mesmos trechos", () => {
  const l = layout();
  const existente = { id: "l1", a: ponta("local", "M2", "right"), b: ponta("destino", "eDP-1", "left") };
  l.links = [existente];
  assert.deepEqual(proporLigacao(l, { peer_id: "destino", edge: "right" }), existente);
});


test("posição simples move só o eixo escolhido e preserva o deslocamento do monitor 2", () => {
  const l = layout();
  l.devices[1].x = 1920;
  assert.deepEqual(posicionarDestino(l, { peer_id: "destino", edge: "bottom" }), { x: 1920, y: 1248 });
  assert.deepEqual(posicionarDestino(l, { peer_id: "destino", edge: "right" }), { x: 4000, y: 0 });
});

test("abaixo do monitor 2 mais baixo usa sua borda externa mesmo com monitor 1 mais alto", () => {
  const l = layout();
  l.devices[0].monitors = [
    { ...monitor("DELL U2723QE"), width: 3840, height: 2160, scale: 1.5 },
    { ...monitor("LG 24GL600F", 3840, 368), index: 1 },
  ];
  l.devices[1].x = 5920;
  l.devices[1].y = 2320;
  l.locked_edges = [ponta("local", "DELL U2723QE", "right", 368 / 2160, 1448 / 2160), ponta("local", "LG 24GL600F", "left")];
  const existente = { id: "l2", a: ponta("local", "LG 24GL600F", "right"), b: ponta("destino", "eDP-1", "left") };
  l.links = [existente];
  const nova = proporLigacao(l, { peer_id: "destino", edge: "bottom" });
  assert.equal(nova.a.monitor, "LG 24GL600F");
  assert.equal(nova.a.edge, "bottom");
  assert.equal(nova.b.edge, "top");
  assert.equal(nova.id, "l2");
});

test("cantos mortos ao lado de uma ligação existente não são trecho disponível", () => {
  const l = layout();
  l.options.dead_corner_px = 8;
  l.locked_edges = [ponta("local", "M1", "right")];
  l.links = [{ id: "outro", a: ponta("local", "M2", "right", 8 / 1080, 1 - 8 / 1080), b: ponta("outro", "HDMI-1", "left") }];
  assert.equal(proporLigacao(l, { peer_id: "destino", edge: "right" }), null);
});

test("se o monitor alinhado só tem cantos mortos, escolhe outro trecho externo útil", () => {
  const l = layout();
  l.options.dead_corner_px = 8;
  l.devices[1].x = 1920;
  l.devices[1].y = 1200;
  l.links = [{ id: "outro", a: ponta("local", "M2", "bottom", 8 / 1920, 1 - 8 / 1920), b: ponta("outro", "HDMI-1", "top") }];
  const nova = proporLigacao(l, { peer_id: "destino", edge: "bottom" });
  assert.equal(nova.a.monitor, "M1");
  assert.ok(nova.a.from >= 8 / 1920);
  assert.ok(nova.a.to <= 1 - 8 / 1920);
});

test("diferença de arredondamento do f32 não vira uma passagem vazia", () => {
  const l = layout();
  l.options.dead_corner_px = 8;
  l.locked_edges = [ponta("local", "M1", "right")];
  l.links = [{ id: "outro", a: ponta("local", "M2", "right", 8 / 1080 + Number.EPSILON, 1 - 8 / 1080 - Number.EPSILON), b: ponta("outro", "HDMI-1", "left") }];
  assert.equal(proporLigacao(l, { peer_id: "destino", edge: "right" }), null);
});


test("recusa anterior à gravação difere de falha com persistência indeterminada", () => {
  const stale = falhaAoLigar("stale_rev", "O layout mudou.", true);
  assert.match(stale, /ligação não foi alterada/);
  for (const code of ["config", "timeout", "io", undefined]) {
    const mensagem = falhaAoLigar(code, "Não foi possível aplicar a alteração.", true);
    assert.match(mensagem, /não foi possível confirmar/i);
    assert.doesNotMatch(mensagem, /ligação não foi alterada/);
    assert.match(mensagem, /Não foi possível aplicar a alteração/);
  }
  assert.equal(falhaAoLigar("config", "Falha na primeira gravação.", false), "Falha na primeira gravação.");
});
