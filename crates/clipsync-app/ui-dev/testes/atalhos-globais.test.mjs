import assert from "node:assert/strict";
import { test } from "node:test";

// Atalhos Win+Alt só são anunciados onde o sistema os registra (features.global_shortcuts).
// Teste puro sobre o estado: não cria DOM nem prova o navegador.
globalThis.document = { getElementById: () => ({}) };
const { api } = await import("../../ui/js/api.js");
const { atalhoGlobal, atualizarApp } = await import("../../ui/js/estado.js");

const estado = (features) => ({ initialized: true, device: { name: "a" }, agent: "in_app", paused: false, mesh: { connected: 0, total: 0 }, features });

test("sem estado carregado, nenhum atalho é anunciado", () => {
  assert.equal(atalhoGlobal(2), null);
  assert.equal(atalhoGlobal("P"), null);
});

test("sem atalho global confirmado (Linux, agente parado), devolve null mesmo com tecla configurada", async () => {
  api.appState = async () => estado({ global_shortcuts: false, screen_lock: true, switch_device: true });
  await atualizarApp();
  assert.equal(atalhoGlobal(2), null);
  assert.equal(atalhoGlobal("L"), null);
  assert.equal(atalhoGlobal("P"), null);
});

test("com atalho global confirmado (Windows com entrada ativa), devolve o texto da tecla", async () => {
  api.appState = async () => estado({ global_shortcuts: true });
  await atualizarApp();
  assert.equal(atalhoGlobal(2), "Win+Alt+2");
  assert.equal(atalhoGlobal("L"), "Win+Alt+L");
  assert.equal(atalhoGlobal("P"), "Win+Alt+P");
});

test("tecla vazia ou ausente nunca vira atalho, mesmo com o recurso ligado", async () => {
  api.appState = async () => estado({ global_shortcuts: true });
  await atualizarApp();
  assert.equal(atalhoGlobal(null), null);
  assert.equal(atalhoGlobal(undefined), null);
  assert.equal(atalhoGlobal(""), null);
});
