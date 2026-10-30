import { test } from 'node:test';
import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
const modulo = (path) => JSON.stringify(pathToFileURL(resolve(path)).href);
function caso(cenario, corpo) {
  execFileSync(process.execPath, ['--input-type=module', '-e', `
    import assert from 'node:assert/strict';
    globalThis.location = { search: ${JSON.stringify(`?cenario=${cenario}`)} };
    globalThis.document = { getElementById: () => ({}) };
    const mock = await import(${modulo('crates/clipsync-app/ui-dev/mock.js')});
    const comandos = [];
    globalThis.window = { __TAURI__: { core: { invoke: (cmd, args) => { comandos.push(structuredClone(args.req)); return mock.invoke(cmd,args); } } } };
    const { api } = await import(${modulo('crates/clipsync-app/ui/js/api.js')});
    const { criarEdicaoModulos } = await import(${modulo('crates/clipsync-app/ui/js/telas/preferencias.js')});
    const { atualizarApp, obterApp } = await import(${modulo('crates/clipsync-app/ui/js/estado.js')});
    const inicio = await atualizarApp();
    const edicao = criarEdicaoModulos({ modules: inicio.modules, aplicar: api.modulesSet, reconsultar: atualizarApp });
    ${corpo}
  `], { encoding: 'utf8' });
}

test('API e simulador: aplicar reinicia pausado, reconsulta e só retoma por pedido explícito', () => caso('padrao', `
  await edicao.aplicar();
  assert.equal(comandos.filter((r)=>r.cmd==='modules_set').length,0);
  edicao.editar('input',false);
  assert.equal(comandos.filter((r)=>r.cmd==='modules_set').length,0);
  const resposta=await edicao.aplicar();
  assert.equal(resposta.effects.start,'owned_ready');
  assert.equal(obterApp().paused,true);
  assert.equal(comandos.filter((r)=>r.cmd==='pause_set').length,0);
  assert.equal(comandos.filter((r)=>r.cmd==='onboarding_create').length,0);
  assert.deepEqual(edicao.rascunho,{input:false,clipboard:true});
  await api.pauseSet({paused:false});await atualizarApp();
  assert.equal(obterApp().paused,false);
`));

for (const [cenario, code, salvo] of [
  ['modulos-conflito-tardio','stale_config',false],
  ['modulos-falha-inicio','agent_start_failed',true],
  ['modulos-falha-pausa','io',false],
  ['modulos-externo','agent_external',false],
  ['modulos-desconhecido','agent_unknown',false],
  ['modulos-externo-apos-save','agent_external',true],
]) test(`API e simulador: ${cenario} atualiza rascunho sem retry nem recriação`, () => caso(cenario, `
  edicao.editar('input',false);
  await assert.rejects(edicao.aplicar(), e => e.code === ${JSON.stringify(code)} && e.details.effects.config_saved === ${salvo});
  assert.deepEqual(edicao.rascunho,${salvo ? '{input:false,clipboard:true}' : '{input:true,clipboard:true}'});
  assert.equal(edicao.alterado,false);
  await edicao.aplicar();
  assert.equal(comandos.filter((r)=>r.cmd==='modules_set').length,1);
  assert.equal(comandos.filter((r)=>r.cmd==='onboarding_create'||r.cmd==='pause_set').length,0);
`));

for (const [cenario, trecho, pausa] of [
  ['padrao', 'pausada', true], ['b09-externo', 'externo', false], ['b09-parado', 'parado', false],
]) test(`API e feedback de modo: ${cenario} conserva o estado real`, () => caso(cenario, `
  const {feedbackModo}=await import(${modulo('crates/clipsync-app/ui/js/telas/clipboard.js')});
  const resposta=await api.clipboardSetMode({mode:'manual'});await atualizarApp();
  assert.match(feedbackModo(resposta),new RegExp(${JSON.stringify(trecho)}));
  assert.equal(obterApp().paused,${pausa});
  assert.equal(comandos.filter((r)=>r.cmd==='pause_set').length,0);
`));

for (const [cenario, salvo, trecho] of [
  ['b09-falha-pausa',false,'não foi persistida'], ['b09-falha-inicio',true,'inicialização'], ['b09-falha-trava',true,'trava'],
]) test(`API e feedback de modo parcial: ${cenario} reconsulta sem alegar aplicação completa`, () => caso(cenario, `
  const {resumoEfeitos}=await import(${modulo('crates/clipsync-app/ui/js/telas/preferencias.js')});
  await assert.rejects(api.clipboardSetMode({mode:'manual'}),e=>{
    assert.equal(e.details.effects.config_saved,${salvo});
    assert.match(resumoEfeitos(e.details),new RegExp(${JSON.stringify(trecho)}));return true;
  });
  await atualizarApp();
  assert.equal(obterApp().paused,true);
  assert.equal(comandos.filter((r)=>r.cmd==='pause_set').length,0);
`));

for (const [cenario, pedir, trecho] of [
  ['b09-falha-pausa',true,'pausa'], ['b09-falha-retomada',false,'retomada'],
]) test(`API e aviso persistente: ${cenario} nomeia a ação realmente solicitada`, () => caso(cenario, `
  await assert.rejects(api.pauseSet({paused:${pedir}}));
  await atualizarApp();
  const aviso=obterApp().pause_warning;
  assert.match(aviso,new RegExp(${JSON.stringify(trecho)}));
  assert.equal(obterApp().paused,${pedir});
  await atualizarApp();assert.equal(obterApp().pause_warning,aviso);
`));

test('API e estado desconhecido: posse própria não confirma pausa pelo fallback', () => caso('b09-desconhecido', `
  const {pausaObservada}=await import(${modulo('crates/clipsync-app/ui/js/estado.js')});
  assert.equal(obterApp().agent,'in_app');assert.equal(pausaObservada(obterApp()),null);
  await assert.rejects(api.clipboardSetMode({mode:'manual'}),e=>e.code==='agent_unknown');
  await atualizarApp();assert.equal(pausaObservada(obterApp()),null);
  assert.ok(obterApp().pause_warning);
`));
