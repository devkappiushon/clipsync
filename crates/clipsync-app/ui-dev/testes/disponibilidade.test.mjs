import assert from "node:assert/strict";
import { test } from "node:test";

// Mesmo harness mínimo de destinos de ux-pareamento-s9: monta telas, componentes,
// descrições e handlers reais. Não comprova navegador, layout nem fala assistiva.
globalThis.document = { getElementById: () => ({}) };
const { api } = await import("../../ui/js/api.js");
const { atualizarApp } = await import("../../ui/js/estado.js");
const telas = Object.fromEntries(await Promise.all(["dispositivos", "pareamento", "clipboard", "diagnostico"].map(async (nome) =>
  [nome, (await import(`../../ui/js/telas/${nome}.js`)).default])));
const assentar = async () => { await new Promise((r) => setTimeout(r, 0)); await Promise.resolve(); };
function destinosDeFoco() {
  const doc = { defaultView: { location: { href: '#/pareamento' } }, ouvintes: [] };
  const elementos = (no) => no.filhos.flatMap((f) => typeof f === 'string' ? [] : [f, ...elementos(f)]);
  const corresponde = (no, seletor) => seletor.split(',').some((s) => {
    s = s.trim();
    if (s === '[data-focus-key]') return no.dataset.focusKey !== undefined;
    if (s.startsWith('.')) return no.className.split(' ').includes(s.slice(1));
    if (s === '[hidden]') return no.hidden;
    if (s === '[inert]') return no.atributos.has('inert');
    if (s === '[id]') return !!no.id;
    if (s === '[autofocus]') return no.atributos.has('autofocus');
    if (s.startsWith('[role=')) return no.getAttribute('role') === s.slice(7, -2);
    if (s === 'dialog[open]') return no.tagName === 'DIALOG' && no.atributos.has('open');
    if (s.endsWith(':not(:disabled)')) return no.tagName === s.split(':')[0].toUpperCase() && !no.disabled;
    return no.tagName === s.toUpperCase();
  });
  const criar = (tag) => {
    const no = { tagName: tag.toUpperCase(), ownerDocument: doc, filhos: [], pai: null, atributos: new Map(), dataset: {}, style: {},
      className: '', disabled: false, checked: false, hidden: false, tabIndex: ['input', 'button'].includes(tag) ? 0 : -1, escritas: [], eventos: new Map(),
      get isConnected() { return this === doc.body || !!this.pai?.isConnected; },
      get textContent() { return this.filhos.map((f) => typeof f === 'string' ? f : f.textContent).join(''); },
      set textContent(v) { this.escritas.push(v); this.replaceChildren(String(v)); },
      contains(alvo) { return this === alvo || elementos(this).includes(alvo); },
      append(...filhos) { for (const f of filhos) { if (typeof f === 'object') { if (f.pai) f.pai.filhos = f.pai.filhos.filter((v) => v !== f); f.pai = this; } this.filhos.push(typeof f === 'object' ? f : String(f)); } },
      replaceChildren(...filhos) {
        if (this.contains(doc.activeElement)) doc.activeElement = doc.body;
        this.filhos.forEach((f) => { if (typeof f === 'object') f.pai = null; }); this.filhos = []; this.append(...filhos);
      },
      remove() { if (this.contains(doc.activeElement)) doc.activeElement = doc.body; this.pai.filhos = this.pai.filhos.filter((f) => f !== this); this.pai = null; },
      setAttribute(k, v) { this.atributos.set(k, String(v)); if (k === 'hidden') this.hidden = true; },
      getAttribute(k) { return this.atributos.get(k) ?? null; },
      removeAttribute(k) { this.atributos.delete(k); if (k === 'hidden') this.hidden = false; },
      toggleAttribute(k, ativo) { if (ativo) this.setAttribute(k, ''); else this.removeAttribute(k); },
      addEventListener(k, fn) { const lista = this.eventos.get(k) ?? []; lista.push(fn); this.eventos.set(k, lista); },
      querySelectorAll(s) { return elementos(this).filter((f) => corresponde(f, s)); },
      querySelector(s) { return this.querySelectorAll(s)[0] ?? null; },
      matches(s) { return corresponde(this, s); },
      closest(s) { return corresponde(this, s) ? this : this.pai?.closest(s); },
      focus() { if (doc.activeElement === this || !this.isConnected || this.disabled || this.closest('[hidden], [inert]')) return; doc.activeElement = this; doc.ouvintes.forEach((fn) => fn({ target: this })); },
      showModal() { this.setAttribute('open', ''); this.querySelector('[autofocus]')?.focus(); },
      close(valor) { this.returnValue = valor; this.removeAttribute('open'); (this.eventos.get('close') ?? []).forEach((fn) => fn()); },
      async acionar(nome) { await Promise.all((this.eventos.get(nome) ?? []).map((fn) => fn({ target: this, preventDefault() {} }))); },
    };
    no.classList = { add(c) { if (!no.className.split(' ').includes(c)) no.className += ` ${c}`; },
      remove(c) { no.className = no.className.split(' ').filter((v) => v !== c).join(' '); },
      toggle(c, ativo) { if (ativo) this.add(c); else this.remove(c); } };
    return no;
  };
  doc.body = criar('body'); doc.documentElement = doc.body; doc.activeElement = doc.body;
  doc.createElement = criar; doc.createElementNS = (_ns, tag) => criar(tag);
  doc.addEventListener = (nome, fn) => { if (nome === 'focusin') doc.ouvintes.push(fn); };
  doc.querySelector = (s) => doc.body.querySelector(s);
  return doc;
}

const SUPORTE = "Teclado e mouse: indisponível nesta versão neste sistema.";
const DESLIGADO = "O módulo de teclado e mouse está desligado neste computador. Confira Módulos em Preferências.";
const DESCONHECIDO = "Não foi possível confirmar os recursos de teclado e mouse deste computador. Consulte o Diagnóstico.";
const REMOTO = (nome) => `Não foi possível confirmar os recursos de ${nome}.`;
const AGREGADO = "Não foi possível confirmar a disponibilidade de teclado e mouse para este pareamento.";
const ORIENTACAO = (nome) => `Para controlar ${nome} a partir deste computador, autorize este computador na tela Dispositivos de ${nome}. O ClipSync não informa aqui se essa autorização já foi concedida.`;
const nomeHostil = '<img src=x onerror="globalThis.invadiu=true">';
const appPadrao = () => ({ features: { input: true }, input_supported: true, modules: { clipboard: true, input: true }, agent: "in_app" });
const parPadrao = () => ({ peer_id: "remoto", name: "Outra", short: "AA:BB", state: "connected", addr: null, paired_at: 1700000000000,
  remote_modules: { clipboard: true, input: true }, perms: { clipboard_in: false, clipboard_out: false, input_in: false } });

async function cena(t, tela, { app = appPadrao(), par = parPadrao(), step = "permissions", available,
  clipboard = { enabled: true, classification: "validated" }, peers = [] } = {}) {
  const anterior = globalThis.document; const intervalo = globalThis.setInterval; const limpar = globalThis.clearInterval;
  const doc = destinosDeFoco(); doc.defaultView.location.href = `#/${tela}`;
  globalThis.document = doc; globalThis.setInterval = () => 1; globalThis.clearInterval = () => {};
  const originais = { ...api }; const pedidos = []; const eventos = new Map();
  Object.assign(api, {
    appState: async () => app,
    devicesList: async () => ({ devices: [par] }),
    deviceSetPerms: async (args) => { pedidos.push(["permissoes", args]); par.perms = { ...args.perms }; return { device: par }; },
    pairState: async () => ({ step, attempt_id: "tentativa", code: "123456", expires_unix_ms: Date.now() + 120000,
      peer: { name: par.name, short: par.short, modules: par.remote_modules } }),
    pairCodeMatches: async () => ({ step: "permissions", available: available ?? { clipboard: true, input: true },
      defaults: { clipboard_in: false, clipboard_out: false, input_in: false } }),
    pairCancel: async () => {},
    pairFinish: async (args) => { pedidos.push(["concluir", args]); return { step: "waiting_peer" }; },
    clipboardGet: async () => ({ ...clipboard, mode: "auto", inbox: [], retention: { max_items: 10, ttl_ms: 60000 } }),
    diagnosticsGet: async () => ({ items: [], network: { peers }, blocks: [] }),
  });
  await atualizarApp();
  const montagem = telas[tela]({ ir() {}, assinar: (nomes, fn) => {
    for (const nome of Array.isArray(nomes) ? nomes : [nomes]) eventos.set(nome, fn);
    return () => {};
  } });
  doc.body.append(montagem.no);
  t.after(() => { montagem.desmontar(); Object.assign(api, originais); globalThis.document = anterior;
    globalThis.setInterval = intervalo; globalThis.clearInterval = limpar; });
  await assentar();
  assert.doesNotMatch(montagem.no.textContent, /Não foi possível carregar/, 'a tela deve montar sem erro do harness');
  const porChave = (chave) => montagem.no.querySelectorAll('[data-focus-key]').find((e) => e.dataset.focusKey === chave);
  const botao = (rotulo) => doc.body.querySelectorAll('button').find((e) => e.textContent === rotulo);
  const clicar = async (alvo) => { assert.ok(alvo, 'ação presente'); await alvo.acionar('click'); await assentar(); };
  return { doc, montagem, pedidos, porChave, botao, clicar, eventos,
    entrada: () => porChave(tela === 'dispositivos' ? 'permissao:remoto:input-in' : 'pareamento-input-in'),
    descricao(entrada) { return montagem.no.querySelectorAll('[id]').find((e) => e.id === entrada.getAttribute('aria-describedby'))?.textContent; },
    async mudar(entrada, valor) { entrada.checked = valor; await entrada.acionar('change'); await assentar(); },
  };
}

for (const tela of ["dispositivos", "pareamento"]) {
  for (const [rotulo, dados, esperado] of [
    ["suporte ausente precede módulo off", { input_supported: false, modules: { input: false, clipboard: true } }, SUPORTE],
    ["módulo explicitamente off", { modules: { input: false, clipboard: true } }, DESLIGADO],
    ["estado sem campos", { input_supported: undefined, modules: undefined, agent: undefined }, DESCONHECIDO],
    ["agente com erro", { agent_error: "sem resposta" }, DESCONHECIDO],
    ["agente parado", { agent: "stopped", input_supported: undefined }, DESCONHECIDO],
  ]) test(`${tela}: ${rotulo} explica controle já bloqueado`, async (t) => {
    const c = await cena(t, tela, { app: { ...appPadrao(), ...dados, features: { input: false } } });
    assert.equal(c.entrada().disabled, true);
    assert.equal(c.descricao(c.entrada()), esperado);
    assert.deepEqual(c.pedidos, []);
  });

  test(`${tela}: ramo disponível mantém concessão local e confirmação`, async (t) => {
    const c = await cena(t, tela);
    assert.equal(c.entrada().disabled, false);
    await c.mudar(c.entrada(), true);
    assert.deepEqual(c.pedidos, [], 'mudar o switch não concede antes do diálogo');
    assert.ok(c.doc.body.querySelector('dialog[open]'));
    await c.clicar(c.botao('Conceder controle'));
    if (tela === 'pareamento') await c.clicar(c.botao('Concluir'));
    const [comando, args] = c.pedidos[0];
    assert.equal(comando, tela === 'dispositivos' ? 'permissoes' : 'concluir');
    assert.equal(args.acknowledge_full_trust, true);
    assert.deepEqual(args.perms ?? args.grants, { clipboard_in: false, clipboard_out: false, input_in: true });
    assert.equal(args.peer_id ?? args.attempt_id, tela === 'dispositivos' ? 'remoto' : 'tentativa');
  });

  test(`${tela}: explicação local não cria gate quando recurso input já era true`, async (t) => {
    const c = await cena(t, tela, { app: { ...appPadrao(), input_supported: false, modules: { input: false, clipboard: true } } });
    assert.equal(c.entrada().disabled, false, 'o helper de texto não passa a decidir habilitação');
    assert.notEqual(c.descricao(c.entrada()), SUPORTE);
    assert.notEqual(c.descricao(c.entrada()), DESLIGADO);
  });

  test(`${tela}: orientação de autorização no destino é texto seguro sem controle nem escrita`, async (t) => {
    const c = await cena(t, tela, { par: { ...parPadrao(), name: nomeHostil } });
    const orientacao = c.montagem.no.querySelectorAll('p').find((e) => e.textContent === ORIENTACAO(nomeHostil));
    assert.ok(orientacao, 'orientação estática no grupo existente');
    const grupo = orientacao.closest('.opcao');
    assert.ok(grupo);
    assert.equal(grupo.querySelectorAll('input, button').length, 0);
    assert.equal(grupo.querySelectorAll('[role="switch"]').length, 0);
    assert.equal(grupo.querySelectorAll('img').length, 0);
    assert.equal(grupo.querySelector('.opcao__rotulo').textContent, `Controlar ${nomeHostil} a partir deste computador`);
    assert.equal(c.montagem.no.querySelectorAll('[role="switch"]').length, 3);
    assert.deepEqual(c.pedidos, []);
  });
}

for (const state of ['offline', 'connected']) test(`Dispositivos: remoto false/default em ${state} conserva gates sem inventar ausência`, async (t) => {
  const c = await cena(t, 'dispositivos', { par: { ...parPadrao(), state, remote_version: '0.4.0', remote_modules: { clipboard: false, input: false } } });
  for (const chave of ['clipboard-in', 'clipboard-out', 'input-in']) {
    const entrada = c.porChave(`permissao:remoto:${chave}`);
    assert.equal(entrada.disabled, true);
    assert.equal(c.descricao(entrada), REMOTO('Outra'));
  }
  assert.deepEqual(c.pedidos, []);
});

test('Dispositivos: valor concedido permanece marcado e revogação conserva payload sem confirmação', async (t) => {
  const c = await cena(t, 'dispositivos', { par: { ...parPadrao(), perms: { clipboard_in: true, clipboard_out: false, input_in: true } } });
  assert.equal(c.entrada().checked, true);
  await c.mudar(c.entrada(), false);
  assert.equal(c.doc.body.querySelector('dialog[open]'), null);
  assert.deepEqual(c.pedidos, [['permissoes', { peer_id: 'remoto',
    perms: { clipboard_in: true, clipboard_out: false, input_in: false }, acknowledge_full_trust: false }]]);
});

test('Dispositivos: módulo clipboard local off conserva gate antes do remoto', async (t) => {
  const c = await cena(t, 'dispositivos', { app: { ...appPadrao(), modules: { input: true, clipboard: false } },
    par: { ...parPadrao(), remote_modules: { input: true, clipboard: false } } });
  for (const chave of ['clipboard-in', 'clipboard-out']) {
    const entrada = c.porChave(`permissao:remoto:${chave}`);
    assert.equal(entrada.disabled, true);
    assert.equal(c.descricao(entrada), 'O módulo de clipboard está desligado nesta máquina.');
  }
});

test('Dispositivos: bloqueio de confiança precede suporte e módulos', async (t) => {
  const c = await cena(t, 'dispositivos', { app: { ...appPadrao(), input_supported: false, features: { input: false } },
    par: { ...parPadrao(), state: 'blocked', remote_modules: { clipboard: false, input: false } } });
  for (const entrada of c.montagem.no.querySelectorAll('[role="switch"]')) {
    assert.equal(entrada.disabled, true);
    assert.equal(c.descricao(entrada), 'Computador bloqueado. Use Reparear para renovar a confiança.');
    await c.mudar(entrada, true);
  }
  assert.deepEqual(c.pedidos, []);
});

for (const step of ['code', 'permissions', 'waiting_peer']) test(`Pareamento: etapa ${step} preserva gates e progresso`, async (t) => {
  const c = await cena(t, 'pareamento', { step });
  if (step === 'code') {
    assert.equal(c.entrada(), undefined, 'as permissões só aparecem após confirmar o código');
    assert.equal(c.botao('Concluir'), undefined);
    assert.ok(c.botao('São iguais'));
    assert.deepEqual(c.pedidos, []);
    return;
  }
  assert.equal(c.entrada().disabled, step !== 'permissions');
  assert.equal(c.botao('Concluir').disabled, step !== 'permissions');
  if (step === 'waiting_peer') assert.match(c.montagem.no.textContent, /Suas escolhas foram enviadas/);
  assert.deepEqual(c.pedidos, []);
});

test('Pareamento: available agregado false não é atribuído ao computador remoto', async (t) => {
  const c = await cena(t, 'pareamento', { step: 'code', available: { clipboard: true, input: false } });
  await c.clicar(c.botao('São iguais'));
  assert.equal(c.entrada().disabled, true);
  assert.equal(c.descricao(c.entrada()), AGREGADO);
  await c.clicar(c.botao('Concluir'));
  assert.deepEqual(c.pedidos[0][1].grants, { clipboard_in: false, clipboard_out: false, input_in: false });
  assert.equal(c.pedidos[0][1].acknowledge_full_trust, false);
});

for (const [rotulo, enabled, classification, modulo, plataforma, tom] of [
  ['off', false, 'unavailable', 'desligado', 'não avaliada com o módulo desligado', 'neutro'],
  ['desconhecido', undefined, 'unavailable', 'não confirmado', 'estado da plataforma não confirmado', 'neutro'],
  ['null', null, 'validated', 'não confirmado', 'estado da plataforma não confirmado', 'neutro'],
  ['ligado validado', true, 'validated', 'ligado', 'plataforma validada', 'ok'],
  ['ligado indisponível', true, 'unavailable', 'ligado', 'plataforma indisponível', 'erro'],
  ['ligado experimental', true, 'experimental', 'ligado', 'plataforma experimental', 'aviso'],
]) test(`Clipboard: ${rotulo} apresenta estado sem diagnóstico conflitante`, async (t) => {
  const c = await cena(t, 'clipboard', { clipboard: { enabled, classification } });
  const celulas = c.montagem.no.querySelectorAll('.clipboard__celula');
  const valor = (rotulo) => celulas.find((e) => e.querySelector('dt')?.textContent === rotulo).querySelector('dd');
  assert.equal(valor('Módulo').textContent, modulo);
  assert.equal(valor('Plataforma').textContent, plataforma);
  assert.ok(valor('Plataforma').querySelector(`.selo--${tom}`));
  const aviso = c.montagem.no.querySelector('.clipboard__estado').querySelector('.aviso');
  assert.equal(!!aviso, enabled === false, 'só false observado é descrito como desligado');
});

test('Diagnóstico: vazio orienta estado e endereço sem prometer latência', async (t) => {
  const c = await cena(t, 'diagnostico');
  const texto = c.montagem.no.querySelectorAll('p').find((e) => e.textContent === 'Pareie um computador em Dispositivos para acompanhar o estado e o endereço disponível.');
  assert.ok(texto);
  assert.doesNotMatch(texto.textContent, /latência|RTT/);
});
for (const peer_rtt of [false, true]) test(`Diagnóstico: gate RTT ${peer_rtt} conserva colunas e explicação de medição ausente`, async (t) => {
  const c = await cena(t, 'diagnostico', { app: { ...appPadrao(), features: { input: true, peer_rtt } }, peers: [parPadrao()] });
  const colunas = c.montagem.no.querySelectorAll('th').map((e) => e.textContent);
  assert.equal(colunas.includes('RTT'), peer_rtt);
  assert.equal(colunas.includes('Entrada p50 / p95'), peer_rtt);
  if (!peer_rtt) assert.ok(c.montagem.no.querySelectorAll('p').some((e) => e.textContent ===
    'RTT e latência de entrada por computador: indisponível nesta versão. Essas medições ainda não foram implementadas.'));
});
