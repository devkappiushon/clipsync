// Compatibilidade dos cenários com o contrato JSON usado pela interface.
import assert from "node:assert/strict";
import { test } from "node:test";
globalThis.location = { search: "?cenario=padrao", protocol: "http:", hostname: "127.0.0.1" };
const { invoke } = await import("../mock.js");
const { estado } = await import("../mock/nucleo.js");
const appState = () => invoke("clipsync", { req: { cmd: "app_state" } });

test("input desligado desabilita as capacidades de configuração e mostra a causa", async (t) => {
  const anterior = estado.modulos.input;
  t.after(() => { estado.modulos.input = anterior; });
  estado.modulos.input = false;
  const app = await appState();
  for (const recurso of ["input", "peer_monitors", "layout_links", "dead_corners", "screen_lock", "switch_device", "layout_copy"]) assert.equal(app.features[recurso], false, recurso);
  assert.equal(app.traversal.code, "input_off");
});

test("protocolo ausente ou zero não confirma a configuração nem inventa versão", async (t) => {
  const par = estado.dispositivos[0];
  const anterior = par.remote_version;
  t.after(() => { par.remote_version = anterior; });
  for (const versao of [null, 0]) {
    par.remote_version = versao;
    const app = await appState();
    const travessia = app.traversal.peers.find((p) => p.peer_id === par.peer_id);
    assert.equal(travessia.code, "checking");
    assert.equal(travessia.protocol_warning, null);
  }
});
