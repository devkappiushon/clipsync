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

// S10 substitui gravação por grupo: operações por monitor permanecem no rascunho.
test("posição de monitor só persiste depois da prévia e aplicação explícita", async () => {
  globalThis.document = { getElementById: () => ({}) };
  const { controladorMonitores } = await import("../../ui/js/monitores.js");
  const local = "1".repeat(64), remoto = "2".repeat(64);
  const inventory = (peer, uid, space) => ({ stamp: { peer, connection: peer === local ? 0 : 3, rev: 1 }, inventory: {
    rev: 1, space, source: "Observed", capabilities: { internal_edges: space === "Physical", positioned_return: space === "Physical", route_receiver: true },
    monitors: [{ uid: uid.repeat(32), label: uid, rect: { x: 0, y: 0, width: 1000, height: 1000 }, scale: 1 }],
  } });
  const commands = [];
  const c = controladorMonitores({ layout: { rev: 1, self_id: local, devices: [], inventories: [inventory(local,"1","Physical"), inventory(remoto,"2","Logical")], organization: null }, api: {
    layoutPreviewOrganization: async (args) => { commands.push("preview"); return { ...args, effective_links: [], preserved_link_ids: [], replaced_link_ids: [], unresolved_link_ids: [] }; },
    layoutApplyOrganization: async (args) => { commands.push(["apply",args]); return { layout_rev: 2, activation: "confirmed", durability: "confirmed" }; },
  } });
  c.mover({ peer: remoto, uid: "2".repeat(32) }, { x: 1000, y: 0 });
  assert.deepEqual(commands, []);
  await c.aplicar(); assert.deepEqual(commands, []);
  await c.prever(); await c.aplicar(); await c.aplicar();
  assert.equal(commands.length, 2); assert.equal(commands[1][1].positions[1].x, 1000);
});

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
