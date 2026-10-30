import { test } from 'node:test';
import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
const mock = pathToFileURL(resolve('crates/clipsync-app/ui-dev/mock.js')).href;
const core = pathToFileURL(resolve('crates/clipsync-app/ui-dev/mock/nucleo.js')).href;
function caso(search, body) {
  const program = `import assert from 'node:assert/strict';
    globalThis.location={search:${JSON.stringify(search)}};
    const {invoke,listen}=await import(${JSON.stringify(mock)});
    const {estado}=await import(${JSON.stringify(core)});
    const call=(cmd,args)=>invoke('clipsync',{req:{cmd,args}});
    const req=(modules)=>({modules,expected:{...estado.modulos}});
    ${body}`;
  const output=execFileSync(process.execPath,['--input-type=module','-e',program],{encoding:'utf8'});
  if(output.trim()) console.log(output.trim());
}
test('no-op não altera módulos, pausa, identidade ou grants nem emite eventos',()=>caso('',`
  const antes=structuredClone(estado);const eventos=[];await listen('',({payload})=>eventos.push(payload));
  const result=await call('modules_set',req({...estado.modulos}));
  assert.equal(result.changed,false);assert.equal(result.effects.config_saved,false);
  assert.deepEqual(estado,antes);assert.deepEqual(eventos,[]);
`));
test('reinício simulado permanece pausado e preserva identidade e grants',()=>caso('',`
  const identidade=structuredClone(estado.eu);const pares=structuredClone(estado.dispositivos);
  const result=await call('modules_set',req({input:false,clipboard:true}));
  assert.equal(result.effects.start,'owned_ready');assert.equal(result.observed.paused,true);
  assert.equal(estado.pausado,true);assert.deepEqual(estado.eu,identidade);assert.deepEqual(estado.dispositivos,pares);
`));
test('parado salva sem iniciar nem pausar',()=>caso('?cenario=modulos-parado',`
  const result=await call('modules_set',req({input:false,clipboard:true}));
  assert.equal(result.observed.agent,'stopped');assert.equal(result.effects.pause,'not_requested');
  assert.equal(result.effects.start,'not_requested');assert.equal(estado.pausado,false);
`));
for(const [scenario,code,agent] of [['externo','agent_external','external'],['desconhecido','agent_unknown','unknown'],['ocupado','profile_busy','in_app']]) {
  test(`módulos recusam ${scenario} com efeitos explícitos`,()=>caso(`?cenario=modulos-${scenario}`,`
    const before=structuredClone(estado.modulos);
    await assert.rejects(call('modules_set',req({input:false,clipboard:true})),error=>{
      assert.equal(error.code,${JSON.stringify(code)});assert.equal(error.details.effects.config_saved,false);
      assert.equal(error.details.effects.pause,'not_requested');assert.equal(error.details.observed.agent,${JSON.stringify(agent)});return true;
    });assert.deepEqual(estado.modulos,before);
  `));
}
for(const [stage,saved,stop,start] of [['pausa',false,'not_requested','not_requested'],['parada',false,'requested','not_requested'],['escrita',false,'confirmed','not_requested'],['inicio',true,'confirmed','failed'],['trava',true,'confirmed','failed']]) {
  test(`falha parcial em ${stage} conserva efeitos anteriores`,()=>caso(`?cenario=modulos-falha-${stage}`,`
    await assert.rejects(call('modules_set',req({input:true,clipboard:false})),error=>{
      assert.equal(error.details.effects.config_saved,${saved});assert.equal(error.details.effects.stop,${JSON.stringify(stop)});
      assert.equal(error.details.effects.start,${JSON.stringify(start)});assert.equal(error.details.observed.paused,true);return true;
    });assert.equal(estado.pausado,true);
  `));
}
test('schema completo e expected antigo são recusados antes dos efeitos',()=>caso('',`
  for(const modules of [{input:true},{input:true,clipboard:true,extra:true},{input:1,clipboard:true}]) {
    await assert.rejects(call('modules_set',req(modules)),error=>error.code==='bad_request');
  }
  await assert.rejects(call('modules_set',{modules:{input:false,clipboard:true},expected:{input:false,clipboard:true}}),error=>error.code==='stale_config');
  assert.equal(estado.pausado,false);
`));
for(const [query,wanted] of [['160',160],['500',500],['1400',1400],['invalida',160]]) {
  test(`latência DEV ${query} afeta despacho de app_state`,()=>caso(`?latencia=${query}`,`
    const begin=performance.now();await call('app_state');const elapsed=performance.now()-begin;
    assert.ok(elapsed>=${wanted-20},'despacho terminou antes do atraso pedido');
    console.log(JSON.stringify({latencia_pedida:${JSON.stringify(query)},base_esperada_ms:${wanted},observado_ms:Math.round(elapsed)}));
  `));
}
test('habilitar input não promete restaurar trava de módulo antes desligado',()=>caso('?cenario=input-desligado',`
  const result=await call('modules_set',req({input:true,clipboard:true}));
  assert.equal(result.effects.lock_restore,'not_needed');assert.equal(result.observed.screen_locked,false);
`));
test('latência de despacho soma os atrasos do handler sem multiplicá-los',()=>caso('?latencia=500',`
  const begin=performance.now();await call('modules_set',req({input:false,clipboard:true}));
  const elapsed=performance.now()-begin;assert.ok(elapsed>=800);
  console.log(JSON.stringify({comando:'modules_set',latencia_pedida_ms:500,atrasos_handler_ms:320,observado_ms:Math.round(elapsed)}));
`));
test('resposta do simulador coincide com a fixture tipada Rust',()=>caso('',`
  const {readFile}=await import('node:fs/promises');
  const fixture=JSON.parse(await readFile('crates/clipsync-ui/tests/fixtures/modules_set.json','utf8'));
  assert.deepEqual(await call('modules_set',req({input:false,clipboard:true})),fixture);
`));
