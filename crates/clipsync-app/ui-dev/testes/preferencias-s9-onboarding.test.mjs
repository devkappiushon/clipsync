import assert from 'node:assert/strict';
import { test } from 'node:test';
globalThis.document = { getElementById: () => ({}) };
const pedidos = [];
globalThis.window = { __TAURI__: { core: { invoke: async (_cmd, { req }) => {
  pedidos.push(req);
  if (req.cmd === 'modules_set') return { changed: true, modules: req.args.modules };
  if (req.cmd === 'onboarding_create') return { initialized: true };
  return { initialized: true, modules: { input: true, clipboard: true } };
} } } };
const { salvarEscolhaModulos } = await import('../../ui/js/telas/onboarding.js');
const { criarEdicaoModulos } = await import('../../ui/js/telas/preferencias.js');
const { api } = await import('../../ui/js/api.js');

test('revisar perfil existente aplica módulos sem criar ou refazer identidade', async () => {
  pedidos.length = 0;
  const edicao = criarEdicaoModulos({ modules: { input: true, clipboard: false }, aplicar: api.modulesSet,
    reconsultar: async () => ({ modules: { input: true, clipboard: true } }) });
  edicao.editar('clipboard', true);
  await salvarEscolhaModulos({ perfilExiste: true, edicao, nome: 'ignorado', modulos: edicao.rascunho });
  assert.deepEqual(pedidos, [{ cmd: 'modules_set', args: { modules: { input: true, clipboard: true }, expected: { input: true, clipboard: false } } }]);
});

test('primeiro uso cria uma vez e recusa confirmação sem tentar recreate', async () => {
  pedidos.length = 0;
  await salvarEscolhaModulos({ perfilExiste: false, nome: 'Mesa', modulos: { input: true, clipboard: false } });
  assert.deepEqual(pedidos, [{ cmd: 'onboarding_create', args: { name: 'Mesa', modules: { input: true, clipboard: false }, port: 47800, recreate: false } }]);
  window.__TAURI__.core.invoke = async (_cmd, { req }) => { pedidos.push(req); throw { code: 'needs_confirmation' }; };
  await assert.rejects(salvarEscolhaModulos({ perfilExiste: false, nome: 'Mesa', modulos: { input: true, clipboard: false } }), (e) => e.code === 'needs_confirmation');
  assert.equal(pedidos.length, 2);
  assert.equal(pedidos[1].args.recreate, false);
});

test('revisão mostra a identidade atual sem preparar outra identidade pelo onboarding_start', async () => {
  pedidos.length = 0;
  window.__TAURI__.core.invoke = async (_cmd, { req }) => { pedidos.push(req); return { default_name: 'Candidato', fingerprint_short: '11:22:33:44', defaults: { input: true, clipboard: false } }; };
  const { carregarInicio } = await import('../../ui/js/telas/onboarding.js');
  const inicio = await carregarInicio({ initialized: true, device: { name: 'Mesa existente', short: 'AA:BB:CC:DD' }, modules: { input: true, clipboard: false }, input_supported: true });
  assert.equal(inicio.fingerprint_short, 'AA:BB:CC:DD');
  assert.equal(inicio.default_name, 'Mesa existente');
  assert.deepEqual(inicio.defaults, { input: true, clipboard: false });
  assert.deepEqual(pedidos, []);
});
