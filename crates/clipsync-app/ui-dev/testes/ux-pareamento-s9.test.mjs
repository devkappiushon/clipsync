import assert from "node:assert/strict";
import { test } from "node:test";
// Impede somente a montagem do sprite no import; não simula DOM, foco ou modal.
globalThis.document = { getElementById: () => ({}) };
const tela = await import("../../ui/js/telas/pareamento.js");

test("recomeçar cancela a tentativa anterior antes de abrir outra de 120 segundos", async () => {
  assert.equal(typeof tela.iniciarTentativa, "function");
  const chamadas = [];
  const comandos = {
    pairCancel: async (args) => { chamadas.push(["cancelar", args]); },
    pairHostStart: async (args) => { chamadas.push(["aceitar", args]); return { attempt_id: "nova", port: 5555, addresses: ["10.0.0.1:5555"], expires_unix_ms: 990000 }; },
    pairJoinStart: async () => { throw new Error("não deve conectar no modo aceitar"); },
  };
  const estado = await tela.iniciarTentativa({ comandos, modo: "aceitar", anterior: "antiga", endereco: "preservado" });
  assert.deepEqual(chamadas, [["cancelar", { attempt_id: "antiga" }], ["aceitar", { port: 0, window_secs: 120 }]]);
  assert.equal(estado.attempt_id, "nova");
  assert.equal(estado.step, "listening");
  assert.equal(estado.code, undefined);
  assert.equal(estado.grants, undefined);
});

test("recomeçar Conectar preserva endereço e não inicia se o cancelamento falha", async () => {
  assert.equal(typeof tela.iniciarTentativa, "function");
  let chamadas = 0;
  const comandos = {
    pairCancel: async () => { throw new Error("cancelamento recusado"); },
    pairJoinStart: async () => { chamadas++; },
  };
  await assert.rejects(tela.iniciarTentativa({ comandos, modo: "conectar", anterior: "antiga", endereco: " 10.0.0.1:5555 " }), /cancelamento recusado/);
  assert.equal(chamadas, 0);
  comandos.pairCancel = async () => {};
  comandos.pairJoinStart = async (args) => { chamadas++; assert.deepEqual(args, { addr: "10.0.0.1:5555" }); return { attempt_id: "nova" }; };
  const estado = await tela.iniciarTentativa({ comandos, modo: "conectar", anterior: "antiga", endereco: " 10.0.0.1:5555 " });
  assert.equal(estado.step, "connecting");
  assert.equal(estado.attempt_id, "nova");
  assert.equal(chamadas, 1);
});

test("prazo anuncia 30 segundos e expiração uma vez, sem alongar o código", () => {
  assert.equal(typeof tela.criarAvisosPrazo, "function");
  const verificar = tela.criarAvisosPrazo();
  assert.equal(verificar(120000, 0), null);
  assert.equal(verificar(120000, 89000), null);
  assert.equal(verificar(120000, 90000), "30s");
  assert.equal(verificar(120000, 95000), null);
  assert.equal(verificar(120000, 120000), "expirou");
  assert.equal(verificar(120000, 121000), null);
});

// Destinos mínimos de atributos, texto e foco, no padrão de base-acessivel.
// Executa montar/componentes/helpers reais; não prova navegador, tabulação, layout ou fala assistiva.
const { api } = await import('../../ui/js/api.js');
const { atualizarApp } = await import('../../ui/js/estado.js');
function promessaPendente() {
  let resolver; let rejeitar;
  const promessa = new Promise((ok, erro) => { resolver = ok; rejeitar = erro; });
  return { promessa, resolver, rejeitar };
}
const assentar = async () => { await Promise.resolve(); await Promise.resolve(); await Promise.resolve(); };
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
async function cenaPareamento(t) {
  const anterior = globalThis.document; const intervalo = globalThis.setInterval; const limpar = globalThis.clearInterval;
  const doc = destinosDeFoco(); const eventos = new Map(); const timers = new Set(); const pedidos = [];
  let agora = 1000000; let contador = 0;
  t.mock.method(Date, 'now', () => agora);
  globalThis.document = doc;
  globalThis.setInterval = (fn) => { timers.add(fn); return fn; };
  globalThis.clearInterval = (fn) => timers.delete(fn);
  const originais = { ...api };
  Object.assign(api, {
    appState: async () => ({ features: { input: true } }), pairState: async () => ({ step: 'idle', attempt_id: null }),
    pairJoinStart: async (args) => { pedidos.push(['conectar', args]); return { attempt_id: `tentativa-${++contador}` }; },
    pairCancel: async (args) => { pedidos.push(['cancelar', args]); },
    pairCodeMatches: async () => ({ step: 'permissions', available: { clipboard: true, input: true }, defaults: { clipboard_in: false, clipboard_out: false, input_in: false } }),
    pairFinish: async (args) => { pedidos.push(['concluir', args]); return { step: 'waiting_peer' }; },
  });
  await atualizarApp();
  const montagem = tela.default({ ir() {}, assinar: (nome, fn) => { eventos.set(nome, fn); return () => eventos.delete(nome); } });
  doc.body.append(montagem.no); await assentar();
  t.after(() => { api.pairCancel = async () => {}; montagem.desmontar(); Object.assign(api, originais); globalThis.document = anterior; globalThis.setInterval = intervalo; globalThis.clearInterval = limpar; });
  const porChave = (chave) => montagem.no.querySelectorAll('[data-focus-key]').find((e) => e.dataset.focusKey === chave);
  const botao = (rotulo) => montagem.no.querySelectorAll('button').find((e) => e.textContent === rotulo);
  const clicar = async (alvo) => { assert.ok(alvo, 'a ação precisa estar visível'); alvo.focus(); await alvo.acionar('click'); };
  const endereco = async (valor) => { const entrada = porChave('pareamento-endereco'); entrada.value = valor; await entrada.acionar('input'); };
  const modoConectar = porChave('pareamento-modo:conectar'); modoConectar.checked = true; await modoConectar.acionar('change'); await assentar();
  await endereco('a10.0.0.99:53122');
  return { doc, pedidos, montagem, porChave, botao, clicar, endereco,
    emitir: (nome, data) => eventos.get(nome)?.(data),
    codigo(id) { eventos.get('pair_code_ready')({ attempt_id: id, code: '123456', peer: { name: 'Outra', short: 'AA:BB', modules: { clipboard: true, input: true } } }); },
    ticar(ms) { agora = ms; [...timers].forEach((fn) => fn()); },
    async preparar(acao) {
      if (acao === 'recomecar') { await clicar(botao('Conectar')); eventos.get('pair_result')({ attempt_id: 'tentativa-1', outcome: 'failed', reason: 'Endereço inacessível.' }); }
      const espera = promessaPendente(); api.pairJoinStart = (args) => { pedidos.push(['conectar', args]); return espera.promessa; };
      const gatilho = botao(acao === 'conectar' ? 'Conectar' : 'Recomeçar com novo código');
      const pendente = clicar(gatilho); await assentar();
      return { espera, gatilho, pendente };
    },
  };
}

for (const acao of ['conectar', 'recomecar']) {
  test(`${acao} atrasado oferece destino de espera e avança ao código enquanto conserva a interação`, async (t) => {
    const c = await cenaPareamento(t); const p = await c.preparar(acao);
    assert.equal(c.doc.activeElement, p.gatilho);
    p.espera.resolver({ attempt_id: 'nova' }); await p.pendente;
    assert.equal(c.doc.activeElement === c.doc.body, false, 'espera precisa de destino lógico após remover o gatilho');
    assert.match(c.doc.activeElement.textContent, /Conectando/);
    c.codigo('nova');
    assert.equal(c.doc.activeElement.textContent, 'Código de confirmação');
  });
  for (const movimento of ['outro-controle', 'controle-removido', 'rota', 'modal']) {
    test(`${acao} atrasado preserva ${movimento} posterior à ação e ao chegar o código`, async (t) => {
      const c = await cenaPareamento(t); const p = await c.preparar(acao);
      const outro = c.doc.createElement('button'); c.doc.body.append(outro);
      if (movimento === 'rota') { c.doc.defaultView.location.href = '#/dispositivos'; outro.focus(); }
      else if (movimento === 'modal') { const modal = c.doc.createElement('dialog'); modal.setAttribute('open', ''); c.doc.body.append(modal); modal.append(outro); outro.focus(); }
      else { outro.focus(); if (movimento === 'controle-removido') outro.remove(); }
      const foco = c.doc.activeElement;
      p.espera.resolver({ attempt_id: 'nova' }); await p.pendente;
      assert.equal(c.doc.activeElement, foco);
      c.codigo('nova'); assert.equal(c.doc.activeElement, foco);
    });
  }
  test(`${acao} não avança ao código depois de movimento voluntário durante a espera`, async (t) => {
    const c = await cenaPareamento(t); const p = await c.preparar(acao);
    p.espera.resolver({ attempt_id: 'nova' }); await p.pendente;
    const outro = c.doc.createElement('button'); c.doc.body.append(outro); outro.focus();
    c.codigo('nova'); assert.equal(c.doc.activeElement, outro);
  });
  test(`${acao} não ressuscita tela desmontada e cancela tentativa retornada tarde`, async (t) => {
    const c = await cenaPareamento(t); const p = await c.preparar(acao);
    c.montagem.desmontar(); c.montagem.no.remove();
    p.espera.resolver({ attempt_id: 'nova' }); await p.pendente;
    assert.ok(c.pedidos.some(([cmd, args]) => cmd === 'cancelar' && args.attempt_id === 'nova'));
    assert.equal(c.doc.body.textContent, '');
  });
}
for (const outcome of ['expired', 'failed', 'declined_peer', 'cancelled']) {
  test(`resultado ${outcome} mantém feedback visível e somente uma região assertiva com texto`, async (t) => {
    const c = await cenaPareamento(t); await c.clicar(c.botao('Conectar'));
    c.emitir('pair_result', { attempt_id: 'tentativa-1', outcome, reason: 'Prazo encerrado.' });
    const alertas = c.montagem.no.querySelectorAll('[role="alert"]').filter((e) => e.textContent.trim());
    assert.equal(alertas.length, 1, 'resultado terminal usa apenas o anunciador persistente');
    const visivel = c.montagem.no.querySelector('.aviso--erro');
    assert.ok(visivel?.textContent.trim(), 'a mensagem terminal continua visível');
    assert.equal(visivel.textContent, alertas[0].textContent);
  });
}
test('30 segundos e expiração local anunciam uma vez, inclusive após resultado do agente', async (t) => {
  const c = await cenaPareamento(t); await c.clicar(c.botao('Conectar'));
  c.ticar(1090000); c.ticar(1095000);
  const status = c.montagem.no.querySelectorAll('[role="status"]').find((e) => e.className === 'so-leitor');
  assert.equal(status.escritas.filter((v) => /30 segundos/.test(v)).length, 1);
  c.ticar(1120000); c.ticar(1130000);
  let alertas = c.montagem.no.querySelectorAll('[role="alert"]').filter((e) => e.textContent.trim());
  assert.equal(alertas.length, 1); assert.equal(alertas[0].escritas.filter((v) => /tempo de pareamento acabou/.test(v)).length, 1);
  assert.match(c.montagem.no.textContent, /Tempo de pareamento esgotado/);
  c.emitir('pair_result', { attempt_id: 'tentativa-1', outcome: 'expired', reason: 'Prazo encerrado.' });
  alertas = c.montagem.no.querySelectorAll('[role="alert"]').filter((e) => e.textContent.trim());
  assert.equal(alertas.length, 1);
});
for (const termino of ['failed', 'expired', 'prazo-local']) {
  test(`corrigir endereço após ${termino} preserva valor para editar e cria outra tentativa limpa`, async (t) => {
    const c = await cenaPareamento(t); await c.clicar(c.botao('Conectar'));
    if (termino === 'prazo-local') c.ticar(1120000);
    else c.emitir('pair_result', { attempt_id: 'tentativa-1', outcome: termino, reason: 'Endereço inacessível.' });
    await c.clicar(c.botao('Corrigir endereço'));
    assert.equal(c.porChave('pareamento-endereco').value, 'a10.0.0.99:53122');
    assert.equal(c.doc.activeElement, c.porChave('pareamento-endereco'));
    await c.endereco('10.0.0.99:53122'); await c.clicar(c.botao('Conectar'));
    assert.deepEqual(c.pedidos.filter(([cmd]) => cmd === 'conectar').map(([, args]) => args.addr), ['a10.0.0.99:53122', '10.0.0.99:53122']);
    c.codigo('tentativa-1'); assert.doesNotMatch(c.montagem.no.textContent, /123456/);
    c.codigo('tentativa-2'); await c.clicar(c.botao('São iguais'));
    assert.equal(c.porChave('pareamento-clipboard-in').checked, false);
    assert.equal(c.porChave('pareamento-clipboard-out').checked, false);
    assert.equal(c.porChave('pareamento-input-in').checked, false);
    await c.clicar(c.botao('Concluir'));
    const envio = c.pedidos.find(([cmd]) => cmd === 'concluir')[1];
    assert.equal(envio.attempt_id, 'tentativa-2');
    assert.deepEqual(envio.grants, { clipboard_in: false, clipboard_out: false, input_in: false });
    assert.equal(envio.acknowledge_full_trust, false);
  });
}

test('corrigir após expiração descarta grants e confirmação de confiança já concedidos', async (t) => {
  const c = await cenaPareamento(t); await c.clicar(c.botao('Conectar')); c.codigo('tentativa-1');
  await c.clicar(c.botao('São iguais'));
  const clipboard = c.porChave('pareamento-clipboard-in'); clipboard.checked = true; await clipboard.acionar('change');
  const entrada = c.porChave('pareamento-input-in'); entrada.checked = true; await entrada.acionar('change');
  const conceder = c.doc.body.querySelectorAll('button').find((e) => e.textContent === 'Conceder controle');
  await c.clicar(conceder); await assentar();
  assert.equal(entrada.checked, true);
  c.ticar(1120000); await c.clicar(c.botao('Corrigir endereço'));
  await c.endereco('10.0.0.99:53122'); await c.clicar(c.botao('Conectar')); c.codigo('tentativa-2');
  await c.clicar(c.botao('São iguais')); await c.clicar(c.botao('Concluir'));
  const envio = c.pedidos.find(([cmd]) => cmd === 'concluir')[1];
  assert.deepEqual(envio.grants, { clipboard_in: false, clipboard_out: false, input_in: false });
  assert.equal(envio.acknowledge_full_trust, false);
});

test('corrigir endereço espera cancelamento, ignora eventos antigos e preserva foco escolhido durante await', async (t) => {
  const c = await cenaPareamento(t); await c.clicar(c.botao('Conectar')); c.ticar(1120000);
  const espera = promessaPendente(); api.pairCancel = (args) => { c.pedidos.push(['cancelar', args]); return espera.promessa; };
  const corrigir = c.clicar(c.botao('Corrigir endereço')); await assentar();
  assert.equal(c.porChave('pareamento-endereco'), undefined, 'editar só abre depois de cancelar a tentativa expirada');
  const outro = c.doc.createElement('button'); c.doc.body.append(outro); outro.focus();
  c.emitir('pair_result', { attempt_id: 'tentativa-1', outcome: 'cancelled' });
  espera.resolver(); await corrigir;
  assert.equal(c.doc.activeElement, outro);
  assert.equal(c.porChave('pareamento-endereco').value, 'a10.0.0.99:53122');
  c.codigo('tentativa-1');
  c.emitir('pair_result', { attempt_id: 'tentativa-1', outcome: 'expired' });
  assert.ok(c.botao('Conectar'));
  assert.doesNotMatch(c.montagem.no.textContent, /123456/);
});

test('corrigir endereço não ressuscita tela desmontada durante cancelamento', async (t) => {
  const c = await cenaPareamento(t); await c.clicar(c.botao('Conectar')); c.ticar(1120000);
  const espera = promessaPendente(); api.pairCancel = () => espera.promessa;
  const corrigir = c.clicar(c.botao('Corrigir endereço')); await assentar();
  c.montagem.desmontar(); c.montagem.no.remove(); espera.resolver(); await corrigir;
  assert.equal(c.doc.body.textContent, '');
});

test('cancelamento recusado conserva recuperação e não permite conectar endereço novo', async (t) => {
  const c = await cenaPareamento(t); await c.clicar(c.botao('Conectar')); c.ticar(1120000);
  api.pairCancel = async () => { throw Error('Cancelamento recusado.'); };
  await c.clicar(c.botao('Corrigir endereço'));
  assert.equal(c.porChave('pareamento-endereco'), undefined);
  assert.equal(c.pedidos.filter(([cmd]) => cmd === 'conectar').length, 1);
  assert.ok(c.botao('Corrigir endereço'));
});

for (const acao of ['conectar', 'recomecar']) test(`${acao} mantém posse do foco após resync silencioso da espera`, async (t) => {
  const c = await cenaPareamento(t); const p = await c.preparar(acao);
  p.espera.resolver({ attempt_id: 'nova' }); await p.pendente;
  api.pairState = async () => ({ step: 'connecting', attempt_id: 'nova', expires_unix_ms: 1120000 });
  await c.emitir('resync');
  c.codigo('nova'); assert.equal(c.doc.activeElement.textContent, 'Código de confirmação');
});

test('cancelamento recusado preserva sucesso recebido durante await da correção', async (t) => {
  const c = await cenaPareamento(t); await c.clicar(c.botao('Conectar')); c.codigo('tentativa-1');
  await c.clicar(c.botao('São iguais')); await c.clicar(c.botao('Concluir')); c.ticar(1120000);
  const espera = promessaPendente(); api.pairCancel = () => espera.promessa;
  api.pairState = async () => ({ step: 'waiting_peer', attempt_id: 'tentativa-1', expires_unix_ms: 1120000 });
  const corrigir = c.clicar(c.botao('Corrigir endereço')); await assentar();
  await c.emitir('resync');
  c.emitir('pair_result', { attempt_id: 'tentativa-1', outcome: 'paired', device: {
    name: 'Outra', short: 'AA:BB', peer_id: 'remoto', addr: '10.0.0.99:53122',
    perms: { clipboard_in: false, clipboard_out: false, input_in: false }, remote_modules: { clipboard: true, input: true },
  } });
  espera.rejeitar(Error('A decisão já foi enviada.')); await corrigir;
  assert.match(c.montagem.no.textContent, /Outra agora faz parte da malha/);
  assert.doesNotMatch(c.montagem.no.textContent, /Tempo de pareamento esgotado/);
  assert.equal(c.botao('Corrigir endereço'), undefined);
});

for (const step of ['connecting', 'code', 'permissions']) {
  test(`correção aceita mantém edição quando resync ainda publica a tentativa descartada em ${step}`, async (t) => {
    const c = await cenaPareamento(t); await c.clicar(c.botao('Conectar'));
    if (step !== 'connecting') c.codigo('tentativa-1');
    if (step === 'permissions') await c.clicar(c.botao('São iguais'));
    c.ticar(1120000);
    // pairCancel aceito não retira current: o watcher ainda pode estar encerrando A.
    const snapshotA = { step, attempt_id: 'tentativa-1', port: null, addresses: [], expires_unix_ms: 1120000,
      code: step === 'connecting' ? null : '123456',
      peer: step === 'connecting' ? null : { name: 'Outra', short: 'AA:BB', modules: { clipboard: true, input: true } },
    };
    let consultas = 0;
    api.pairState = async () => { consultas++; return snapshotA; };
    const espera = promessaPendente();
    api.pairCancel = (args) => { c.pedidos.push(['cancelar', args]); return espera.promessa; };
    const corrigir = c.clicar(c.botao('Corrigir endereço')); await assentar();
    await c.emitir('resync'); await c.emitir('resync');
    await c.clicar(c.botao('Corrigir endereço'));
    assert.equal(consultas, 0, 'resync durante cancelamento aguarda a ação ocupada');
    assert.equal(c.pedidos.filter(([cmd]) => cmd === 'cancelar').length, 1);
    c.emitir('pair_result', { attempt_id: 'tentativa-1', outcome: 'cancelled' });
    espera.resolver({ cancelled: true }); await corrigir;
    assert.equal(consultas, 1, 'resyncs pendentes são agrupados');
    assert.ok(c.porChave('pareamento-endereco'), 'snapshot de A não pode remover a edição já aberta');
    assert.equal(c.porChave('pareamento-endereco').value, 'a10.0.0.99:53122');
    assert.ok(c.doc.activeElement === c.porChave('pareamento-endereco'), 'o campo de endereço deve manter o foco após a correção');
    assert.doesNotMatch(c.montagem.no.textContent, /123456|Código confirmado|Conectando à outra máquina/);
    await c.endereco('10.0.0.99:53122');
    await c.emitir('resync'); await c.emitir('resync');
    c.codigo('tentativa-1'); c.emitir('pair_result', { attempt_id: 'tentativa-1', outcome: 'failed' });
    assert.equal(c.porChave('pareamento-endereco').value, '10.0.0.99:53122');
    assert.equal(c.pedidos.filter(([cmd]) => cmd === 'conectar').length, 1, 'só ação explícita pode criar B');
    assert.equal(c.montagem.no.querySelectorAll('[role="alert"]').filter((e) => e.textContent.trim()).length, 0);
    await c.clicar(c.botao('Conectar'));
    assert.deepEqual(c.pedidos.filter(([cmd]) => cmd === 'conectar').map(([, args]) => args.addr), ['a10.0.0.99:53122', '10.0.0.99:53122']);
    // Mesmo após criar B, uma resposta contendo A não pode trocar a tentativa local.
    await c.emitir('resync'); c.codigo('tentativa-1');
    c.emitir('pair_result', { attempt_id: 'tentativa-1', outcome: 'cancelled' });
    assert.match(c.montagem.no.textContent, /Conectando à outra máquina/);
    assert.doesNotMatch(c.montagem.no.textContent, /123456/);
    c.codigo('tentativa-2');
    assert.match(c.montagem.no.textContent, /Código de confirmação/);
    await c.clicar(c.botao('São iguais')); await c.clicar(c.botao('Concluir'));
    assert.deepEqual(c.pedidos.find(([cmd]) => cmd === 'concluir')[1], {
      attempt_id: 'tentativa-2', grants: { clipboard_in: false, clipboard_out: false, input_in: false }, acknowledge_full_trust: false,
    });
  });
}

for (const movimento of ['outro-controle', 'rota', 'desmontagem']) {
  test(`resync atrasado da tentativa descartada preserva ${movimento} depois da correção aceita`, async (t) => {
    const c = await cenaPareamento(t); await c.clicar(c.botao('Conectar')); c.ticar(1120000);
    const cancelamento = promessaPendente(); const consulta = promessaPendente();
    api.pairCancel = () => cancelamento.promessa; api.pairState = () => consulta.promessa;
    const corrigir = c.clicar(c.botao('Corrigir endereço')); await assentar();
    await c.emitir('resync'); cancelamento.resolver({ cancelled: true }); await assentar();
    assert.ok(c.porChave('pareamento-endereco'));
    const outro = c.doc.createElement('button'); c.doc.body.append(outro); outro.focus();
    if (movimento === 'rota') c.doc.defaultView.location.href = '#/dispositivos';
    if (movimento === 'desmontagem') { c.montagem.desmontar(); c.montagem.no.remove(); }
    consulta.resolver({ step: 'code', attempt_id: 'tentativa-1', port: null, addresses: [],
      expires_unix_ms: 1120000, code: '123456', peer: { name: 'Outra', short: 'AA:BB', modules: { clipboard: true, input: true } },
    });
    await corrigir;
    assert.equal(c.doc.activeElement, outro);
    assert.doesNotMatch(c.doc.body.textContent, /123456/);
    if (movimento === 'desmontagem') assert.equal(c.montagem.no.isConnected, false);
    else assert.ok(c.porChave('pareamento-endereco'), 'o retorno tardio não retira o campo de correção');
  });
}
