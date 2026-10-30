import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import vm from "node:vm";
import { test } from "node:test";
import { posicionarDestino, posicaoRelativa, proporLigacao, trechosLivres as trechosDisponiveis } from "../../ui/js/travessia.js";

// Executa os handlers de produção com fronteiras pequenas de eventos/renderização.
// Não cria DOM, layout, foco ou navegador fictício; as asserções são sobre seleção e comandos.
const layoutFonte = await readFile(new URL("../../ui/js/telas/layout.js", import.meta.url), "utf8");
const pareamentoFonte = await readFile(new URL("../../ui/js/telas/pareamento.js", import.meta.url), "utf8");
const trecho = (fonte, inicio, fim) => fonte.slice(fonte.indexOf(inicio), fonte.indexOf(fim));
const executar = (fonte, contexto) => { vm.createContext(contexto); vm.runInContext(fonte, contexto); return contexto; };
const simples = (valor) => JSON.parse(JSON.stringify(valor));
const monitor = { ref: "m", name: "m", index: 0, x: 0, y: 0, width: 1000, height: 1000, scale: 1 };
const mesa = () => ({ self_id: "local", rev: 7, snap_px: 16, options: { dead_corner_px: 0 }, links: [], locked_edges: [], devices: [
  { peer_id: "local", name: "Local", is_self: true, state: "connected", x: 0, y: 0, monitors: [monitor] },
  { peer_id: "remoto", name: "Remoto", is_self: false, state: "connected", x: 9000, y: 9000, monitors: [monitor] },
] });

function editor(layout) {
  let campos = []; let botoes = []; let editada = null;
  const contexto = {
    layout, origemFormulario: { peer_id: "local", monitor: "m", edge: "right" },
    nomeDoMonitor: (m) => m.name, pct: (f) => `${f * 100}%`, BORDAS: { right: "direita" }, trechosDisponiveis,
    mesmaBorda: (a, b) => a.peer_id === b.peer_id && a.monitor === b.monitor && a.edge === b.edge,
    campo: (opcoes) => { campos.push(opcoes); return opcoes; }, botao: (opcoes) => { botoes.push(opcoes); return opcoes; },
    h() {}, cartao: (obj) => obj, guardarFoco: () => null, no: {},
    selecionarLigacao: (l) => { editada = l; }, mostrarAvisoTopo() {}, destinoPadrao: () => ({ peer_id: "remoto", monitor: "m", edge: "left" }),
    pintarCanvas() {}, pintarPainel() {}, pintarLigacoes() {}, selecao: null,
  };
  executar(trecho(layoutFonte, "  function formularioOrigem(", "  function posicaoFina(") + "\nthis.renderizar = formularioOrigem;", contexto);
  const renderizar = () => { campos = []; botoes = []; contexto.renderizar(); };
  contexto.pintarRodape = renderizar;
  renderizar();
  return { campos: () => campos, renderizar, contexto, editada: () => editada,
    abrir: () => botoes.find((b) => b.dataset?.focusKey === "origem-editar").aoClicar() };
}

test("formulário cria em cada lacuna da borda parcialmente ligada e também edita a ligação", () => {
  const layout = mesa();
  const a = { peer_id: "local", monitor: "m", edge: "right", from: 0.2, to: 0.4 };
  layout.links = [{ id: "existente", a, b: { ...a, peer_id: "remoto", edge: "left" } }];
  layout.locked_edges = [{ ...a, from: 0.6, to: 0.8 }];
  const tela = editor(layout);
  const intervalos = tela.campos().find((c) => c.rotulo === "Trecho da borda");
  assert.ok(intervalos, "o formulário precisa permitir escolher qualquer lacuna, além de ligações existentes");
  assert.equal(intervalos.opcoes.length, 4);
  for (const [i, esperado] of [[0, [0, 0.2]], [1, [0.4, 0.6]], [2, [0.8, 1]]]) {
    tela.campos().find((c) => c.rotulo === "Trecho da borda").aoMudar(intervalos.opcoes[i].valor);
    tela.abrir();
    assert.equal(tela.editada(), null);
    assert.equal(tela.contexto.selecao.linkId, null);
    assert.deepEqual(simples([tela.contexto.selecao.ponta.from, tela.contexto.selecao.ponta.to]), esperado);
  }
  tela.campos().find((c) => c.rotulo === "Trecho da borda").aoMudar(intervalos.opcoes[3].valor);
  tela.abrir();
  assert.equal(tela.editada().id, "existente");
});

function grupo() {
  const listeners = new Map(); const atributos = new Map();
  return { listeners, atributos, append() {}, setPointerCapture() {}, classList: { add() {}, remove() {} },
    addEventListener: (nome, fn) => listeners.set(nome, fn), removeEventListener: (nome) => listeners.delete(nome),
    setAttribute: (nome, valor) => atributos.set(nome, valor), removeAttribute: (nome) => atributos.delete(nome) };
}
function canvas() {
  const layout = mesa(); const destino = layout.devices[1]; const gravacoes = [];
  const contexto = { layout, avancado: false, escolha: { peer_id: "remoto", edge: "right" }, proporLigacao, posicionarDestino,
    escala: 1, encaixar: (n) => Math.round(n / 16) * 16, MALHA_PX: 16,
    moverDispositivo: (_d, x, y) => { gravacoes.push({ x, y }); }, selecionarDispositivo() {},
    selecao: null, chave: () => "grupo", ESTADOS: { connected: "conectado" }, tela: (x, y) => ({ x, y }),
    caber: (texto) => texto, nomeDoMonitor: (m) => m.name, medidasDoMonitor: () => "1000x1000",
    svg: () => grupo(), envoltorio: { clientWidth: 3000 }, BORDAS: {},
  };
  executar(trecho(layoutFonte, "  function retangulos(", "  function calcularEscala(") +
    trecho(layoutFonte, "  function grupoDispositivo(", "  function zonaBorda(") +
    "\nthis.retangulosReais = retangulos; this.construir = grupoDispositivo;", contexto);
  const rects = contexto.retangulosReais(); const g = contexto.construir(destino, rects);
  return { contexto, destino, g, gravacoes, exibida: { x: rects[1].x, y: rects[1].y } };
}
const ponteiro = (g, fim) => {
  g.listeners.get("pointerdown")({ button: 0, target: { closest: () => null }, preventDefault() {}, clientX: 0, clientY: 0, pointerId: 1 });
  g.listeners.get("pointermove")({ clientX: 16, clientY: 0 });
  g.listeners.get(fim)({ type: fim });
};
test("arrasto usa a posição realmente desenhada antes de Ligar", () => {
  const tela = canvas();
  assert.deepEqual(tela.exibida, { x: 1168, y: 0 });
  ponteiro(tela.g, "pointerup");
  assert.deepEqual(tela.gravacoes, [{ x: 1184, y: 0 }]);
});
test("seta usa a mesma posição projetada do canvas", () => {
  const tela = canvas();
  tela.g.listeners.get("keydown")({ target: tela.g, key: "ArrowRight", preventDefault() {} });
  assert.deepEqual(tela.gravacoes, [{ x: 1184, y: 0 }]);
});
test("pointercancel remove deslocamento visual sem gravar", () => {
  const tela = canvas();
  ponteiro(tela.g, "pointercancel");
  assert.deepEqual(tela.gravacoes, []);
  assert.equal(tela.g.atributos.has("transform"), false);
});

for (const recargaImediata of [false, true]) {
  test(`resposta normalizada mantém prévia e dois Ligar sem regravar posição (recarga imediata: ${recargaImediata})`, async () => {
    const tela = canvas(); const gravacoes = []; const ligacoes = []; const erros = [];
    let persistido = structuredClone(tela.contexto.layout);
    const carregar = async () => { tela.contexto.layout = structuredClone(persistido); };
    Object.assign(tela.contexto, {
      api: {
        layoutSetDevice: async (args) => {
          gravacoes.push(args);
          const device = persistido.devices.find((d) => d.peer_id === args.peer_id);
          Object.assign(device, { x: Math.round(args.x / 16) * 16, y: Math.round(args.y / 16) * 16 });
          return { device: structuredClone(device), layout_rev: ++persistido.rev };
        },
        layoutSetLink: async (args) => {
          ligacoes.push(args);
          const link = { id: args.id ?? "nova", a: args.a, b: args.b };
          persistido.links = [structuredClone(link)];
          return { link: structuredClone(link), warnings: [], layout_rev: ++persistido.rev };
        },
      },
      executar: async (fn) => { const r = await fn(); if (recargaImediata) await carregar(); return r; },
      saiu: false, posicaoRelativa, pintarSimples() {},
      salvando: false, testando: false, guardarFoco: () => null, restaurarFoco() {},
      no: {}, avisoTopo: {}, trocar() {}, anuncios: { status() {} }, carregar, encontrar() {},
      mostrarAvisoTopo: (_tom, texto) => erros.push(texto), mensagemDe: (erro) => erro.message, falhaAoLigar: (_c, texto) => texto,
    });
    vm.runInContext(trecho(layoutFonte, "  async function moverDispositivo(", "  let larguraPintada") +
      trecho(layoutFonte, "  async function salvarPosicao(", "  async function testarEntrada(") +
      "\nthis.moverReal = moverDispositivo; this.ligarReal = salvarPosicao;", tela.contexto);
    await tela.contexto.moverReal(tela.destino, 1185, 3);
    assert.deepEqual(simples(tela.contexto.escolha.posicao), { x: 1184, y: 0 });
    const recebida = tela.contexto.retangulosReais()[1];
    assert.deepEqual({ x: recebida.x, y: recebida.y }, { x: 1184, y: 0 });
    await carregar();
    const recarregada = tela.contexto.retangulosReais()[1];
    assert.deepEqual({ x: recarregada.x, y: recarregada.y }, { x: 1184, y: 0 });
    await tela.contexto.ligarReal(proporLigacao(tela.contexto.layout, tela.contexto.escolha));
    await tela.contexto.ligarReal(proporLigacao(tela.contexto.layout, tela.contexto.escolha));
    assert.equal(gravacoes.length, 1, "Ligar deve preservar a posição normalizada já salva pelo movimento");
    assert.equal(ligacoes.length, 2);
    assert.equal(ligacoes[0].base_rev, 8);
    assert.equal(ligacoes[1].base_rev, 9);
    assert.deepEqual(erros, []);
  });
}

// O módulo importado fornece a política real de iniciar/cancelar; o sprite é a única fronteira DOM.
globalThis.document = { getElementById: () => ({}) };
const { iniciarTentativa } = await import("../../ui/js/telas/pareamento.js");
function pareamento(inicial = { step: "idle", attempt_id: null }) {
  let responderConsulta; let responderConexao; let consultas = 0;
  let remoto = { step: "idle", attempt_id: null };
  let reterConsulta = true;
  const contexto = { api: {
    pairState: () => { consultas++; return reterConsulta ? new Promise((r) => { responderConsulta = r; }) : Promise.resolve(remoto); },
    pairJoinStart: () => new Promise((r) => { responderConexao = () => { remoto = { step: "connecting", attempt_id: "nova" }; r({ attempt_id: "nova" }); }; }),
    pairCancel: async () => {},
  }, inicial, iniciarTentativa, criarAvisosPrazo: () => () => null,
    guardarFoco: () => null, interacaoAtual: () => false, pintar() {}, anuncios: { erro() {}, limpar() {} },
    mensagemDe: (e) => e.message, no: {}, corpo: {}, disponiveisDoEstado: () => ({}),
    botao: (b) => { contexto.botoes.push(b); return b; }, botoes: [],
    campo: () => ({ controle: { addEventListener() {} } }), h() {}, cartao() {}, abandonar() {},
  };
  const funcoes = trecho(pareamentoFonte, "  async function chamar(", "  function disponiveisDoEstado(") +
    trecho(pareamentoFonte, "  function cartaoOcioso(", "  /** Endereço desta máquina");
  executar(`let estado = inicial; let geracaoCarga = 0; let consultaPendente = false; let saiu = false; let ocupado = false;
    let expirou = false; let erroGeral = null; let erroEndereco = null; let disponiveis = {};
    let grants = {}; let confiancaTotal = false; let avisosPrazo; let resultado = null; let tentativaDescartada = null;
    let modo = "conectar"; let enderecoDigitado = "10.0.0.1:5555";
    ${funcoes}
    this.consultar = carregar; this.recomecar = recomecar;
    this.conectar = () => { cartaoOcioso(); return botoes.find((b) => b.rotulo === "Conectar").aoClicar(); };
    this.ler = () => ({ step: estado.step, attempt_id: estado.attempt_id });`, contexto);
  return { contexto, consultas: () => consultas,
    liberarConsulta: () => { reterConsulta = false; responderConsulta({ step: "idle", attempt_id: null }); },
    liberarConexao: () => { reterConsulta = false; responderConexao(); },
  };
}
for (const acao of ["conectar", "recomecar"]) {
  test(`consulta antiga não desfaz a tentativa nova após ${acao}`, async () => {
    const tela = pareamento(acao === "recomecar" ? { step: "code", attempt_id: "antiga" } : undefined);
    const consulta = tela.contexto.consultar();
    const mutacao = tela.contexto[acao]();
    await Promise.resolve(); await Promise.resolve();
    tela.liberarConexao(); await mutacao;
    tela.liberarConsulta(); await consulta;
    assert.deepEqual(simples(tela.contexto.ler()), { step: "connecting", attempt_id: "nova" });
  });
}
for (const acao of ["conectar", "recomecar"]) {
  test(`consultas durante ${acao} são agrupadas após a mutação encerrar`, async () => {
    const tela = pareamento(acao === "recomecar" ? { step: "code", attempt_id: "antiga" } : undefined);
    const mutacao = tela.contexto[acao]();
    const primeira = tela.contexto.consultar();
    const segunda = tela.contexto.consultar();
    await Promise.resolve(); await Promise.resolve();
    assert.equal(tela.consultas(), 0, "pair_state não deve consultar durante a mutação");
    tela.liberarConexao(); await mutacao; await primeira; await segunda;
    assert.equal(tela.consultas(), 1);
    assert.deepEqual(simples(tela.contexto.ler()), { step: "connecting", attempt_id: "nova" });
  });
}
