import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import vm from 'node:vm';
import { pausaObservada } from '../../ui/js/estado.js';
const fonte = await readFile(new URL('../../ui/js/app.js', import.meta.url), 'utf8');
const trecho = (inicio, fim) => fonte.slice(fonte.indexOf(inicio), fonte.indexOf(fim));

for (const pausado of [false, true]) test(`falha ao ${pausado ? 'retomar' : 'pausar'} com reconsulta falha conserva aviso e estado desconhecido`, async () => {
  const pedidos = []; const erros = [];
  const app = { agent: 'in_app', agent_error: null, paused: pausado };
  const contexto = { el: { pausar: {} }, avisoOperacao: { hidden: true }, falhaConsultaPausa: null,
    comCarregamento: (_botao, fn) => fn(), pausaObservada, obterApp: () => app,
    api: { pauseSet: async (pedido) => { pedidos.push(pedido); throw { code: 'io', message: 'sem resposta' }; } },
    atualizarApp: async () => { throw Error('sem resposta'); }, pintarAvisoPausa() {}, pintarRodape() {}, erroOperacao: (e) => erros.push(e),
  };
  vm.createContext(contexto);
  vm.runInContext(trecho('function alternarPausa()', '\nasync function iniciar()') + '\nthis.alternar = alternarPausa;', contexto);
  await contexto.alternar();
  assert.deepEqual(JSON.parse(JSON.stringify(pedidos)), [{ paused: !pausado }]);
  assert.equal(app.paused, pausado);
  assert.match(contexto.falhaConsultaPausa, pausado ? /a retomada/ : /a pausa/);
  assert.equal(erros.length, 0);
});

test('aviso da pausa usa texto do backend, sem repetir anúncio no polling nem mover foco', () => {
  const publicados = [];
  const contexto = { avisoPausa: { hidden: true }, textoPausa: { textContent: '' }, falhaConsultaPausa: null, ultimoAvisoPausa: null,
    anuncios: { erro: (texto) => publicados.push(texto), limpar: () => publicados.push(null) } };
  vm.createContext(contexto);
  vm.runInContext(trecho('function pintarAvisoPausa(app)', '\nfunction alternarPausa()') + '\nthis.pintar = pintarAvisoPausa;', contexto);
  const aviso = 'A retomada foi aplicada em memória, mas não foi salva.';
  contexto.pintar({ pause_warning: aviso }); contexto.pintar({ pause_warning: aviso, paused: false });
  assert.equal(contexto.textoPausa.textContent, aviso);
  assert.equal(contexto.avisoPausa.hidden, false);
  assert.deepEqual(publicados, [aviso]);
  contexto.pintar({ pause_warning: null });
  assert.equal(contexto.avisoPausa.hidden, true);
  assert.deepEqual(publicados, [aviso, null]);
});

for (const pausado of [true, false]) test(`W-S9-PREF-01: aviso anterior de ${pausado ? 'Pausa' : 'Retomar'} conserva nova ${pausado ? 'retomada' : 'pausa'} incerta até consulta aceita`, async () => {
  const pedidos = []; const publicados = []; const erros = [];
  const antigo = pausado
    ? 'A pausa foi aplicada em memória, mas não foi salva.'
    : 'A retomada foi aplicada em memória, mas não foi salva.';
  const app = { agent: 'in_app', agent_error: null, paused: pausado, pause_warning: antigo };
  let observar;
  const contexto = {
    el: { pausar: {} }, avisoOperacao: { hidden: true }, avisoPausa: { hidden: true }, textoPausa: { textContent: '' },
    falhaConsultaPausa: null, ultimoAvisoPausa: null,
    anuncios: { erro: (texto) => publicados.push(texto), limpar: () => publicados.push(null) },
    comCarregamento: (_botao, fn) => fn(), pausaObservada, obterApp: () => app,
    api: { pauseSet: async (pedido) => { pedidos.push(pedido); throw { code: 'io', message: 'sem resposta' }; } },
    atualizarApp: async () => { throw Error('sem resposta'); }, pintarRodape() {}, erroOperacao: (erro) => erros.push(erro),
    observarApp: (fn) => { observar = fn; },
  };
  vm.createContext(contexto);
  vm.runInContext(
    trecho('function pintarAvisoPausa(app)', '\nasync function iniciar()') +
    trecho('  observarApp((app, consultado) => {', '  observarApp(pintarRodape);') +
    '\nthis.alternar = alternarPausa; this.pintar = pintarAvisoPausa;', contexto,
  );
  contexto.pintar(app);
  await contexto.alternar();
  const local = contexto.falhaConsultaPausa;
  const atual = contexto.textoPausa.textContent;
  const anunciosAposFalha = [...publicados];

  // Publicação de evento e repinturas da mesma faixa não equivalem a AppState aceito.
  observar(app, false); contexto.pintar(app); contexto.pintar(app);
  const localAposEvento = contexto.falhaConsultaPausa;
  const anunciosAposRepinturas = [...publicados];
  const backendAposEvento = app.pause_warning;

  // Só uma consulta aceita encerra a incerteza local; o warning remoto continua intacto.
  observar(app, true);
  const localAposConsulta = contexto.falhaConsultaPausa;
  const textoAposConsulta = contexto.textoPausa.textContent;
  const backendAposConsulta = app.pause_warning;
  app.pause_warning = null; observar(app, true);

  assert.deepEqual(JSON.parse(JSON.stringify(pedidos)), [{ paused: !pausado }]);
  assert.equal(app.paused, pausado, 'falha não confirma o estado solicitado');
  assert.match(local, pausado ? /a retomada/ : /a pausa/);
  assert.equal(atual, `${local} ${antigo}`, 'incerteza atual e aviso anterior precisam ser comunicados juntos');
  assert.deepEqual(anunciosAposFalha, [antigo, `${local} ${antigo}`]);
  assert.deepEqual(anunciosAposRepinturas, anunciosAposFalha, 'mesmo aviso não é reanunciado por repintura/evento');
  assert.equal(localAposEvento, local);
  assert.equal(backendAposEvento, antigo);
  assert.equal(localAposConsulta, null);
  assert.equal(textoAposConsulta, antigo);
  assert.equal(backendAposConsulta, antigo, 'frontend não limpa warning do backend para ocultar conflito');
  assert.equal(contexto.avisoPausa.hidden, true);
  assert.deepEqual(publicados, [antigo, `${local} ${antigo}`, antigo, null]);
  assert.equal(erros.length, 0, 'faixa persistente evita um segundo anúncio genérico do erro');
});
