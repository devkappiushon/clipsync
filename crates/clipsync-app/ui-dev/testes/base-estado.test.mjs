import assert from 'node:assert/strict';
import { test } from 'node:test';
const respostas = [];
globalThis.window = { __TAURI__: { core: { invoke: () => new Promise((resolve) => respostas.push(resolve)) } } };
const { atualizarApp, obterApp, observarApp } = await import('../../ui/js/estado.js');

test('consulta antiga não desfaz o estado publicado por consulta mais recente', async () => {
  const vistos = [];
  const cancelar = observarApp((app) => vistos.push(app.agent));
  const antiga = atualizarApp(); const nova = atualizarApp();
  respostas[1]({ agent: 'in_app', initialized: true }); await nova;
  respostas[0]({ agent: 'stopped', initialized: true }); await antiga;
  assert.equal(obterApp().agent, 'in_app');
  assert.deepEqual(vistos, ['in_app']);
  cancelar();
});
