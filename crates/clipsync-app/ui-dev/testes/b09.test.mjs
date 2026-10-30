import { test } from 'node:test';
import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
const mock=pathToFileURL(resolve('crates/clipsync-app/ui-dev/mock.js')).href;
const core=pathToFileURL(resolve('crates/clipsync-app/ui-dev/mock/nucleo.js')).href;
function caso(search,body) {
  execFileSync(process.execPath,['--input-type=module','-e',`import assert from 'node:assert/strict';globalThis.location={search:${JSON.stringify(search)}};const {invoke}=await import(${JSON.stringify(mock)});const {estado}=await import(${JSON.stringify(core)});const call=(cmd,args)=>invoke('clipsync',{req:{cmd,args}});${body}`],{stdio:'pipe'});
}
for(const [scenario,paused,action] of [['pausa',true,'pausa'],['retomada',false,'retomada']]) {
  test(`B09 falha de ${scenario} fica visível e poll não apaga`,()=>caso(`?cenario=b09-falha-${scenario}`,`
    await assert.rejects(call('pause_set',{paused:${paused}}),e=>e.code==='io');
    const state=await call('app_state');assert.equal(state.paused,${paused});assert.match(state.pause_warning,/${action}/);
    assert.equal((await call('app_state')).pause_warning,state.pause_warning);
    estado.falhaPersistenciaPausa=false;await call('pause_set',{paused:${paused}});assert.equal((await call('app_state')).pause_warning,null);
  `));
}
test('B09 falha de persistência impede modo e reparo reinicia pausado com trava',()=>caso('?cenario=b09-falha-pausa',`
  const mode=estado.clipboard.mode;estado.telaTravada=true;
  await assert.rejects(call('clipboard_set_mode',{mode:'manual'}),e=>{assert.equal(e.details.effects.config_saved,false);assert.equal(e.details.effects.stop,'not_requested');return true;});
  assert.equal(estado.clipboard.mode,mode);assert.equal(estado.pausado,true);
  estado.falhaPersistenciaPausa=false;const r=await call('clipboard_set_mode',{mode:'manual'});assert.equal(r.effects.start,'owned_ready');assert.equal(r.observed.paused,true);assert.equal(r.observed.screen_locked,true);assert.equal((await call('app_state')).pause_warning,null);
`));
test('B09 no-op não emite pausa nem reinicia',()=>caso('',`
  const before=structuredClone(estado);const r=await call('clipboard_set_mode',{mode:'auto'});assert.equal(r.effects.config_saved,false);assert.equal(r.effects.pause,'not_requested');assert.deepEqual(estado,before);
`));
for(const agent of ['externo','parado']) test(`B09 ${agent} só salva configuração`,()=>caso(`?cenario=b09-${agent}`,`
  const r=await call('clipboard_set_mode',{mode:'manual'});assert.equal(r.restart_needed,${agent==='externo'});assert.equal(r.effects.start,'not_requested');assert.equal(r.effects.pause,'not_requested');assert.equal(r.effects.config_saved,true);
`));
test('B09 desconhecido conserva posse mas não inventa pausa observada',()=>caso('?cenario=b09-desconhecido',`
  const s=await call('app_state');assert.equal(s.agent,'in_app');assert.ok(s.agent_error);assert.ok(s.pause_warning);
  await assert.rejects(call('clipboard_set_mode',{mode:'manual'}),e=>{assert.equal(e.details.observed.agent,'unknown');assert.equal(e.details.observed.paused,null);assert.equal(e.details.effects.config_saved,false);return true;});
`));
