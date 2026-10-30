import assert from 'node:assert/strict';
import { test } from 'node:test';
const respostas = [];
globalThis.window = { __TAURI__: { core: { invoke: async () => {
  const resposta = respostas.shift(); if (resposta instanceof Error) throw resposta; return resposta;
} } } };
const estado = await import('../../ui/js/estado.js');

test('tarefa própria sem Status não transforma fallback persistido em pausa confirmada', () => {
  assert.equal(estado.pausaObservada({ agent: 'in_app', agent_error: 'sem Status', paused: true }), null);
  assert.equal(estado.pausaObservada({ agent: 'in_app', agent_error: null, paused: true }), true);
  assert.equal(estado.pausaObservada({ agent: 'external', paused: false }), false);
  assert.equal(estado.pausaObservada({ agent: 'stopped', paused: true }), null);
});

test('aviso persiste com polling omitido e erro; só resposta explícita limpa', async () => {
  respostas.push({ agent: 'in_app', paused: false, pause_warning: 'Não foi possível confirmar a retomada.' }); await estado.atualizarApp();
  respostas.push({ agent: 'in_app', paused: false }); await estado.atualizarApp();
  assert.equal(estado.obterApp().pause_warning, 'Não foi possível confirmar a retomada.');
  respostas.push(Error('sem resposta')); await assert.rejects(estado.atualizarApp());
  assert.equal(estado.obterApp().pause_warning, 'Não foi possível confirmar a retomada.');
  respostas.push({ agent: 'in_app', paused: true, pause_warning: null }); await estado.atualizarApp();
  assert.equal(estado.obterApp().pause_warning, null);
});

test('consulta confirmada permite encerrar incerteza local sem confundir evento com consulta', async () => {
  const consultas = [];
  const parar = estado.observarApp((_app, consulta) => consultas.push(consulta));
  respostas.push({ agent: 'in_app', paused: false, pause_warning: null });
  await estado.atualizarApp(); parar();
  assert.deepEqual(consultas, [true]);
});
