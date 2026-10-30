import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import vm from 'node:vm';

// Fronteiras de renderização e API, sem árvore DOM, layout, foco ou teclado simulados.
// Os handlers e o estado do consumidor são os de produção; área descarta a pintura antiga.
const fonte = (await readFile(new URL('../../ui/js/telas/preferencias.js', import.meta.url), 'utf8'))
  .replace(/^import .*;\n/gm, '').replace('export default function montar', 'function montar').replaceAll('export function ', 'function ');
function consumidor() {
  let corrente = { input: true, clipboard: false };
  let regiao; let resolverAntiga; let consultas = 0;
  const botoes = []; const switches = []; const pedidos = [];
  const no = () => ({ querySelector: () => ({ disabled: false }), querySelectorAll: () => [], setAttribute() {}, removeAttribute() {} });
  const app = () => ({ modules: { ...corrente }, agent: 'in_app', paused: false, platform: 'windows', input_supported: true, features: {} });
  const prefs = { shortcuts: { go_to_device: [] }, notifications: {}, appearance: {}, version: '1', license: 'local' };
  const contexto = {
    api: { prefsGet: async () => prefs, appState: async () => {
      consultas++; if (consultas === 2) return new Promise((r) => { resolverAntiga = r; }); return app();
    }, modulesSet: async (pedido) => {
      pedidos.push(structuredClone(pedido)); corrente = { ...pedido.modules };
      return { changed: true, effects: { config_saved: true }, observed: { agent: 'in_app', paused: true } };
    } },
    area: (opcoes) => {
      regiao = { ...opcoes, no: no(), proteger: (fn) => fn(), recarregar: async () => opcoes.pintar(await opcoes.buscar()), desmontar() {} };
      return regiao;
    },
    botao: (opcoes) => { const b = { ...opcoes, ...no() }; botoes.push(b); return b; },
    interruptor: (opcoes) => { switches.push(opcoes); return no(); },
    segmentado: () => no(), h: no, trocar() {}, cartao: (c) => c, cabecalho() {}, indisponivel() {}, tabela() {}, tecla() {},
    criarAnunciador: () => ({ no: no(), erro() {}, status() {}, limpar() {} }),
    atualizarApp: async () => app(), obterApp: app, observarApp: () => () => {}, recurso: () => false,
    pausaObservada: () => false, guardarFoco: () => null, restaurarFoco() {}, ocupar: () => () => {},
    mensagemDe: (e) => e.message, INDISPONIVEL: 'indisponível',
  };
  vm.createContext(contexto); vm.runInContext(`${fonte}\nthis.montar = montar;`, contexto);
  contexto.montar({ ir() {}, assinar: () => () => {} });
  return { regiao: () => regiao, pintar: async () => regiao.pintar(await regiao.buscar()), pedidos,
    switch: (chave) => switches.findLast((s) => s.chaveFoco === `preferencias-modulo-${chave}`),
    aplicar: () => botoes.findLast((b) => b.rotulo === 'Aplicar módulos').aoClicar(),
    resolverAntiga: () => resolverAntiga({ ...app(), modules: { input: true, clipboard: false } }),
  };
}

test('consulta antiga descartada não muda expected da próxima aplicação', async () => {
  const tela = consumidor(); await tela.pintar();
  tela.switch('clipboard').aoMudar(true);
  const antiga = tela.regiao().buscar();
  await tela.aplicar();
  tela.resolverAntiga(); await antiga;
  tela.switch('input').aoMudar(false);
  await tela.aplicar();
  assert.deepEqual(JSON.parse(JSON.stringify(tela.pedidos)), [
    { modules: { input: true, clipboard: true }, expected: { input: true, clipboard: false } },
    { modules: { input: false, clipboard: true }, expected: { input: true, clipboard: true } },
  ]);
});
