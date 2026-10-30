import assert from 'node:assert/strict';
import { test } from 'node:test';
globalThis.document = { getElementById: () => ({}) };
const preferencias = await import('../../ui/js/telas/preferencias.js');
const clipboard = await import('../../ui/js/telas/clipboard.js');
const { criarEdicaoModulos, condicaoDoAtalho } = preferencias;

test('rascunho só aplica explicitamente, preserva expected e não envia no-op', async () => {
  const pedidos = [];
  let corrente = { input: true, clipboard: false };
  const edicao = criarEdicaoModulos({ modules: corrente,
    aplicar: async (p) => { pedidos.push(p); corrente = p.modules; return { changed: true }; },
    reconsultar: async () => ({ modules: corrente }) });
  await edicao.aplicar();
  assert.deepEqual(pedidos, []);
  edicao.editar('clipboard', true);
  assert.deepEqual(pedidos, []);
  await edicao.aplicar();
  assert.deepEqual(pedidos, [{ modules: { input: true, clipboard: true }, expected: { input: true, clipboard: false } }]);
  assert.equal(edicao.alterado, false);
});

test('ação pendente recusa alterações e segunda aplicação sem duplicar envio', async () => {
  let concluir; let chamadas = 0;
  const edicao = criarEdicaoModulos({ modules: { input: true, clipboard: false },
    aplicar: () => { chamadas++; return new Promise((r) => { concluir = r; }); },
    reconsultar: async () => ({ modules: { input: true, clipboard: true } }) });
  edicao.editar('clipboard', true);
  const primeira = edicao.aplicar();
  assert.equal(edicao.ocupado, true);
  assert.equal(edicao.editar('input', false), false);
  await edicao.aplicar();
  assert.equal(chamadas, 1);
  concluir({ changed: true }); await primeira;
  assert.deepEqual(edicao.rascunho, { input: true, clipboard: true });
});

test('conflito com pausa parcial reconsulta e exige nova escolha sem repetição automática', async () => {
  let consultas = 0; let chamadas = 0;
  const falha = { code: 'stale_config', details: { effects: { config_saved: false, pause: 'confirmed' }, observed: { paused: true } } };
  const edicao = criarEdicaoModulos({ modules: { input: true, clipboard: false },
    aplicar: async () => { chamadas++; throw falha; },
    reconsultar: async () => { consultas++; return { modules: { input: false, clipboard: true }, paused: true }; } });
  edicao.editar('clipboard', true);
  await assert.rejects(edicao.aplicar(), (e) => e === falha);
  assert.equal(consultas, 1); assert.equal(chamadas, 1);
  assert.deepEqual(edicao.rascunho, { input: false, clipboard: true });
  assert.equal(edicao.alterado, false);
  await edicao.aplicar(); assert.equal(chamadas, 1);
});

test('reconsulta falha bloqueia reaplicação de expected sem confirmação', async () => {
  let chamadas = 0;
  const edicao = criarEdicaoModulos({ modules: { input: true, clipboard: false },
    aplicar: async () => { chamadas++; throw { code: 'io' }; },
    reconsultar: async () => { throw Error('sem resposta'); } });
  edicao.editar('clipboard', true);
  await assert.rejects(edicao.aplicar());
  assert.equal(edicao.confirmado, false);
  await assert.rejects(edicao.aplicar()); assert.equal(chamadas, 1);
  edicao.sincronizar({ input: true, clipboard: true });
  assert.equal(edicao.confirmado, true);
});

test('ambos desligados são impedidos antes da API', async () => {
  let chamadas = 0;
  const edicao = criarEdicaoModulos({ modules: { input: true, clipboard: false }, aplicar: () => { chamadas++; }, reconsultar() {} });
  edicao.editar('input', false);
  await assert.rejects(edicao.aplicar(), (e) => e.code === 'no_usable_module');
  assert.equal(chamadas, 0);
});

test('atalho descreve condições reais sem converter capacidade em atividade', () => {
  const app = { platform: 'windows', agent: 'in_app', modules: { input: true }, paused: false, traversal: { code: 'no_local_link', local_links: 0 }, features: { global_shortcuts: true } };
  assert.equal(condicaoDoAtalho('open', app).disponivel, false);
  assert.equal(condicaoDoAtalho('send_once', app).disponivel, false);
  assert.equal(condicaoDoAtalho('lock_screen', { ...app, modules: { input: false } }).motivo, 'Módulo de teclado e mouse desligado.');
  assert.equal(condicaoDoAtalho('go_to_device', app).disponivel, false);
  assert.equal(condicaoDoAtalho('pause_all', { ...app, paused: true, features: { global_shortcuts: true, resume_hotkey_refused: true } }).disponivel, false);
  assert.equal(condicaoDoAtalho('lock_screen', { ...app, paused: true }).disponivel, false);
  assert.equal(condicaoDoAtalho('pause_all', { ...app, platform: 'linux' }).disponivel, false);
});

test('modo diferencia configuração salva, reinício pausado e processo externo', () => {
  const { feedbackModo } = clipboard;
  assert.match(feedbackModo({ mode: 'manual', restart_needed: false, effects: { config_saved: true, start: 'owned_ready' }, observed: { agent: 'in_app', paused: true } }), /pausad/);
  assert.match(feedbackModo({ mode: 'manual', restart_needed: true, effects: { config_saved: true, start: 'not_requested' }, observed: { agent: 'external', paused: null } }), /externo/);
  assert.match(feedbackModo({ mode: 'manual', restart_needed: false, effects: { config_saved: true, start: 'not_requested' }, observed: { agent: 'stopped', paused: null } }), /parado/);
});


test('sem ligação local, atalhos de captura não prometem atividade', () => {
  const app = { platform: 'windows', agent: 'in_app', modules: { input: true }, paused: false, traversal: { local_links: 0 }, features: { global_shortcuts: true } };
  assert.equal(condicaoDoAtalho('lock_screen', app).disponivel, false);
  assert.equal(condicaoDoAtalho('pause_all', app).disponivel, false);
});

test('modo no-op não sugere gravação ou reinício próprio', () => {
  const texto = clipboard.feedbackModo({ mode: 'manual', restart_needed: false, effects: { config_saved: false, pause: 'not_requested', stop: 'not_requested', start: 'not_requested' }, observed: { agent: 'in_app', paused: false } });
  assert.match(texto, /já estava/);
  assert.doesNotMatch(texto, /não salvou|Reinicie/);
});
