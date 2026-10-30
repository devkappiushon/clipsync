// Doubles pequenos de controles: verificam a política JS, não o DOM ou teclado do navegador.
import assert from 'node:assert/strict';
import { test } from 'node:test';
// O sprite já existe no app; evitamos somente sua criação no import.
globalThis.document = { getElementById: () => ({}) };
const { comCarregamento } = await import('../../ui/js/componentes.js');

function controle() {
  const atributos = new Map();
  return { disabled: false, getAttribute: (k) => atributos.get(k) ?? null,
    setAttribute: (k, v) => atributos.set(k, v), removeAttribute: (k) => atributos.delete(k) };
}
function pendente() {
  let resolver;
  const promessa = new Promise((r) => { resolver = r; });
  return { promessa, resolver };
}

test('busy mantém o controle focável e só executa uma requisição', async () => {
  const no = controle();
  const espera = pendente();
  let chamadas = 0;
  const primeira = comCarregamento(no, async () => { chamadas++; await espera.promessa; return 7; });
  assert.equal(no.disabled, false);
  assert.equal(no.getAttribute('aria-busy'), 'true');
  assert.equal(no.getAttribute('aria-disabled'), 'true');
  await comCarregamento(no, () => { chamadas++; });
  espera.resolver();
  assert.equal(await primeira, 7);
  assert.equal(chamadas, 1);
  assert.equal(no.getAttribute('aria-disabled'), null);
});

test('controle já indisponível não chama a operação', async () => {
  const no = controle();
  no.disabled = true;
  let chamadas = 0;
  await comCarregamento(no, () => { chamadas++; });
  assert.equal(chamadas, 0);
  assert.equal(no.disabled, true);
});

test('finalização não habilita controle que ficou indisponível durante a espera', async () => {
  const no = controle();
  const espera = pendente();
  const tarefa = comCarregamento(no, () => espera.promessa);
  no.disabled = true;
  espera.resolver();
  await tarefa;
  assert.equal(no.disabled, true);
});

// O teste usa apenas foco, conexão e uma lista de chaves; não simula layout/modal nativo.
function cenaFoco() {
  const doc = { body: {}, documentElement: {}, defaultView: { location: { href: '#/clipboard' } } };
  const ouvintes = [];
  doc.addEventListener = (nome, fn) => { if (nome === 'focusin') ouvintes.push(fn); };
  doc.querySelector = () => doc.modal ?? null;
  const raiz = { ownerDocument: doc, isConnected: true, filhos: [],
    contains: (n) => raiz.filhos.includes(n), querySelectorAll: () => raiz.filhos };
  const novo = (chave) => ({ ...controle(), ownerDocument: doc, isConnected: true, dataset: { focusKey: chave },
    focus() { doc.activeElement = this; ouvintes.forEach((fn) => fn({ target: this })); } });
  const original = novo('item-"\\:1');
  raiz.filhos = [original]; original.focus();
  return { doc, raiz, original, novo, substituir() {
    original.isConnected = false; doc.activeElement = doc.body;
    const proximo = novo(original.dataset.focusKey); raiz.filhos = [proximo]; return proximo;
  } };
}

async function acessibilidade() {
  const modulo = await import('../../ui/js/acessibilidade.js').catch(() => ({}));
  assert.equal(typeof modulo.guardarFoco, 'function', 'base de continuidade deve estar disponível');
  return modulo;
}

test('repintura restaura a chave literal sem interpretar ID como seletor', async () => {
  const { guardarFoco, restaurarFoco } = await acessibilidade();
  const cena = cenaFoco();
  const salvo = guardarFoco(cena.raiz);
  const proximo = cena.substituir();
  assert.equal(restaurarFoco(salvo, cena.raiz), true);
  assert.equal(cena.doc.activeElement, proximo);
});

test('resposta não rouba foco escolhido durante a espera, mesmo se o novo controle depois sumir', async () => {
  const { guardarFoco, restaurarFoco } = await acessibilidade();
  const cena = cenaFoco();
  const salvo = guardarFoco(cena.raiz);
  const outro = cena.novo('outro'); outro.focus();
  cena.substituir(); outro.isConnected = false;
  assert.equal(restaurarFoco(salvo, cena.raiz), false);
});

test('troca de rota ou novo diálogo impede retorno de foco antigo', async () => {
  const { guardarFoco, restaurarFoco } = await acessibilidade();
  for (const mudar of [(c) => { c.doc.defaultView.location.href = '#/layout'; }, (c) => { c.doc.modal = {}; }]) {
    const cena = cenaFoco(); const salvo = guardarFoco(cena.raiz);
    cena.substituir(); mudar(cena);
    assert.equal(restaurarFoco(salvo, cena.raiz), false);
  }
});

test('alvo removido retorna ao fallback lógico; raiz desmontada não recebe foco', async () => {
  const { guardarFoco, restaurarFoco } = await acessibilidade();
  const cena = cenaFoco(); const salvo = guardarFoco(cena.raiz);
  cena.substituir(); const proximo = cena.novo('proxima-linha'); cena.raiz.filhos = [proximo];
  assert.equal(restaurarFoco(salvo, cena.raiz, { fallback: () => proximo }), true);
  assert.equal(cena.doc.activeElement, proximo);
  const outro = guardarFoco(cena.raiz); cena.raiz.isConnected = false;
  assert.equal(restaurarFoco(outro, cena.raiz, { fallback: proximo }), false);
});

test('escolha ocupada preserva valor confirmado e rejeita segunda alteração', async () => {
  const { criarEscolhaAssincrona } = await acessibilidade();
  const espera = pendente(); let exibido = false; let chamadas = 0;
  const escolher = criarEscolhaAssincrona({ valor: false, aplicar: (v) => { exibido = v; },
    aoMudar: async () => { chamadas++; await espera.promessa; } });
  const primeira = escolher(true);
  await escolher(false);
  assert.equal(exibido, false);
  assert.equal(chamadas, 1);
  espera.resolver(); await primeira;
  assert.equal(exibido, true);
});

test('falha de escolha mantém valor confirmado sem chamar novamente', async () => {
  const { criarEscolhaAssincrona } = await acessibilidade();
  let exibido = 'auto'; let chamadas = 0;
  const escolher = criarEscolhaAssincrona({ valor: 'auto', aplicar: (v) => { exibido = v; },
    aoMudar: async () => { chamadas++; throw new Error('recusado'); } });
  await assert.rejects(escolher('manual'), /recusado/);
  assert.equal(exibido, 'auto'); assert.equal(chamadas, 1);
});

test('retorno do modal aceita foco interno e usa próximo alvo quando gatilho foi removido', async () => {
  const { guardarFoco, restaurarFoco } = await acessibilidade();
  const cena = cenaFoco(); const salvo = guardarFoco(cena.raiz);
  const interno = cena.novo('confirmar'); interno.focus();
  const dialogo = { contains: (n) => n === interno };
  const proximo = cena.substituir(); proximo.dataset.focusKey = 'proxima-linha';
  assert.equal(restaurarFoco(salvo, cena.raiz, { aposDialogo: dialogo, fallback: proximo }), true);
  assert.equal(cena.doc.activeElement, proximo);
});

test('anunciador mantém dois canais vazios e não duplica escrita idêntica', async () => {
  const { criarAnunciador } = await acessibilidade();
  const anterior = globalThis.document;
  // Somente o destino das escritas: não existe árvore/foco/layout neste double.
  globalThis.document = { createElement: () => ({ ...controle(), filhos: [], escritas: 0, texto: '',
    append(no) { this.filhos.push(no); }, get textContent() { return this.texto; },
    set textContent(v) { this.texto = v; this.escritas++; } }) };
  try {
    const canal = criarAnunciador(); const [polido, assertivo] = canal.no.filhos;
    assert.equal(polido.textContent, ''); assert.equal(assertivo.textContent, '');
    canal.status('Carregando'); canal.status('Carregando');
    assert.equal(polido.escritas, 1);
    canal.erro('Falha ao salvar');
    assert.equal(polido.textContent, ''); assert.equal(assertivo.textContent, 'Falha ao salvar');
    canal.limpar(); assert.equal(assertivo.textContent, '');
  } finally { globalThis.document = anterior; }
});

test('resultado assíncrono só abre modal se a interação ainda pertence à ação', async () => {
  const { interacaoAtual, guardarFoco } = await acessibilidade();
  assert.equal(typeof interacaoAtual, 'function');
  const cena = cenaFoco(); const salvo = guardarFoco(cena.raiz);
  assert.equal(interacaoAtual(salvo, cena.raiz), true);
  cena.novo('outra-acao').focus();
  assert.equal(interacaoAtual(salvo, cena.raiz), false);
});

test('polling durante ação só repinta depois de concluir e preserva fallback solicitado', async () => {
  const { coordenarAtualizacao } = await import('../../ui/js/areas.js');
  assert.equal(typeof coordenarAtualizacao, 'function');
  const vistas = []; const espera = pendente();
  const barreira = coordenarAtualizacao(async (opcoes) => { vistas.push(opcoes); });
  const tarefa = barreira.proteger(() => espera.promessa);
  const fallback = () => 'proxima-linha';
  assert.equal(barreira.adiar({ silencioso: true, focoFallback: fallback }), true);
  assert.equal(barreira.adiar({ silencioso: true }), true);
  assert.deepEqual(vistas, []);
  espera.resolver(); await tarefa;
  assert.equal(vistas.length, 1);
  assert.equal(vistas[0].focoFallback, fallback);
  assert.equal(barreira.adiar({}), false);
});

test('fallback nativo mantém botão, campo e link na sequência de Tab', async () => {
  const { guardarFoco, restaurarFoco } = await acessibilidade();
  for (const tagName of ['BUTTON', 'INPUT', 'A']) {
    const cena = cenaFoco(); const salvo = guardarFoco(cena.raiz);
    const alvo = cena.substituir();
    alvo.dataset.focusKey = 'fallback'; alvo.dataset.focusFallback = '';
    alvo.tagName = tagName; alvo.tabIndex = 0;
    alvo.matches = (seletor) => seletor.includes('[data-focus-fallback]');
    assert.equal(restaurarFoco(salvo, cena.raiz, { fallback: alvo }), true);
    assert.equal(alvo.tabIndex, 0, `${tagName} deve continuar alcançável por Tab`);
  }
});

test('título de fallback continua recebendo foco programático sem entrar no Tab', async () => {
  const { guardarFoco, restaurarFoco } = await acessibilidade();
  const cena = cenaFoco(); const salvo = guardarFoco(cena.raiz);
  const titulo = cena.substituir(); titulo.dataset.focusKey = 'titulo';
  let recebeuTabIndex = false;
  Object.defineProperty(titulo, 'tabIndex', { get: () => -1, set: (v) => { recebeuTabIndex = v === -1; } });
  titulo.matches = (seletor) => seletor.includes('h2');
  const focar = titulo.focus;
  titulo.focus = function () { if (recebeuTabIndex) focar.call(this); };
  assert.equal(restaurarFoco(salvo, cena.raiz, { fallback: titulo }), true);
  assert.equal(titulo.tabIndex, -1);
});

// Destinos mínimos de árvore/atributos para executar area e anunciador reais.
// Não simula navegador, tabulação, layout ou fala assistiva.
async function cenaArea(t) {
  const { area } = await import('../../ui/js/areas.js');
  const anterior = globalThis.document;
  const doc = { body: {}, activeElement: null };
  const elemento = (tagName) => ({ ...controle(), ownerDocument: doc, tagName, style: {}, dataset: {},
    filhos: [], texto: '', escritas: [], isConnected: true, contains: () => false,
    querySelectorAll: () => [], querySelector: () => null, closest: () => null, addEventListener() {},
    append(...filhos) { this.filhos.push(...filhos); }, replaceChildren(...filhos) { this.filhos = filhos; },
    get childElementCount() { return this.filhos.length; },
    get textContent() { return this.texto; }, set textContent(v) { this.texto = v; this.escritas.push(v); } });
  doc.createElement = elemento; doc.createElementNS = (_ns, tag) => elemento(tag);
  globalThis.document = doc;
  const respostas = []; const pintados = []; const consulta = pendente();
  const regiao = area({ buscar: () => {
    consulta.resolver();
    return new Promise((resolver, rejeitar) => respostas.push({ resolver, rejeitar }));
  },
    pintar: (dados) => { pintados.push(dados); return elemento('div'); } });
  const [anunciador, conteudo] = regiao.no.filhos;
  const [status, erro] = anunciador.filhos;
  t.after(() => { regiao.desmontar(); globalThis.document = anterior; });
  return { regiao, respostas, pintados, status, erro, conteudo, inicioConsulta: consulta.promessa };
}

for (const ordem of [[0, 1], [1, 0]]) {
  test(`consulta silenciosa conclui anúncio explícito uma vez, respostas ${ordem.join('/')}`, async (t) => {
    const c = await cenaArea(t);
    const buscas = [c.regiao.recarregar(), c.regiao.recarregar({ silencioso: true })];
    assert.equal(c.status.textContent, 'Carregando.');
    c.respostas[ordem[0]].resolver(ordem[0]); await buscas[ordem[0]];
    c.respostas[ordem[1]].resolver(ordem[1]); await buscas[ordem[1]];
    assert.deepEqual(c.pintados, [1], 'resposta antiga não substitui dados novos');
    assert.equal(c.status.textContent, 'Conteúdo atualizado.');
    assert.equal(c.status.escritas.filter((v) => v === 'Conteúdo atualizado.').length, 1);
    assert.equal(c.conteudo.getAttribute('aria-busy'), null);
    const antes = [...c.status.escritas];
    const polling = c.regiao.recarregar({ silencioso: true });
    c.respostas[2].resolver(2); await polling;
    assert.deepEqual(c.status.escritas, antes, 'polling isolado não anuncia nova conclusão');
  });
}

test('falha da consulta silenciosa mais nova encerra anúncio explícito com erro', async (t) => {
  const c = await cenaArea(t);
  const explicita = c.regiao.recarregar();
  const silenciosa = c.regiao.recarregar({ silencioso: true });
  c.respostas[1].rejeitar(new Error('Sem resposta')); await silenciosa;
  c.respostas[0].resolver('antiga'); await explicita;
  assert.deepEqual(c.pintados, []);
  assert.equal(c.status.textContent, '');
  assert.equal(c.erro.textContent, 'Sem resposta');
  assert.equal(c.erro.escritas.filter((v) => v === 'Sem resposta').length, 1);
  assert.equal(c.conteudo.getAttribute('aria-busy'), null);
});

test('desmontar área impede resultado tardio e anúncio de conclusão', async (t) => {
  const c = await cenaArea(t);
  const explicita = c.regiao.recarregar();
  const silenciosa = c.regiao.recarregar({ silencioso: true });
  c.regiao.desmontar();
  const antes = [...c.status.escritas];
  c.respostas[0].resolver('antiga'); c.respostas[1].resolver('nova');
  await Promise.all([explicita, silenciosa]);
  assert.deepEqual(c.pintados, []);
  assert.deepEqual(c.status.escritas, antes);
  assert.equal(c.erro.textContent, '');
});

test('recarga explícita adiada por ação mantém intenção mesmo com polling agrupado', async (t) => {
  const c = await cenaArea(t); const espera = pendente();
  const operacao = c.regiao.proteger(() => espera.promessa);
  await c.regiao.recarregar(); await c.regiao.recarregar({ silencioso: true });
  assert.equal(c.respostas.length, 0);
  espera.resolver();
  // O finally de proteger inicia uma única consulta agrupada.
  await c.inicioConsulta;
  assert.equal(c.respostas.length, 1);
  c.respostas[0].resolver('nova'); await operacao;
  assert.equal(c.status.textContent, 'Conteúdo atualizado.');
  assert.equal(c.status.escritas.filter((v) => v === 'Conteúdo atualizado.').length, 1);
});
