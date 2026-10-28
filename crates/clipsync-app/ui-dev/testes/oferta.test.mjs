// Contrato do layout_offer_answer no simulador (S6-18): a resposta diz qual oferta o usuário viu.
// Roda sem navegador e sem npm: node --test crates/clipsync-app/ui-dev/testes/oferta.test.mjs
// Só prova o simulador; o comportamento real é testado em Rust (clipsync-ui).

import assert from "node:assert/strict";
import { test } from "node:test";

globalThis.location = { search: "?cenario=estados", protocol: "http:", hostname: "127.0.0.1" };

const { invoke } = await import("../mock.js");
const { estado } = await import("../mock/nucleo.js");

const chamar = (args) => invoke("clipsync", { req: { cmd: "layout_offer_answer", args } });
const rejeita = (promessa, code) => assert.rejects(promessa, (e) => e.code === code, `esperava ${code}`);
const oferta = () => estado.layout.pending_offer;

test("resposta sem par ou sem rev é bad_request e a oferta continua", async () => {
  const o = oferta();
  await rejeita(chamar({ accept: true }), "bad_request");
  await rejeita(chamar({ accept: true, from_peer_id: o.from_peer_id }), "bad_request");
  await rejeita(chamar({ accept: true, rev: o.rev }), "bad_request");
  await rejeita(chamar({ from_peer_id: o.from_peer_id, rev: o.rev }), "bad_request");
  assert.ok(oferta());
});

test("rev ou par diferentes da oferta pendente dão stale_offer, com a oferta atual nos detalhes, e nada é aplicado", async () => {
  const o = oferta();
  const ligacoes = estado.layout.links.length;
  await rejeita(chamar({ accept: true, from_peer_id: o.from_peer_id, rev: o.rev + 1 }), "stale_offer");
  await rejeita(chamar({ accept: true, from_peer_id: "0".repeat(64), rev: o.rev }), "stale_offer");
  await assert.rejects(chamar({ accept: true, from_peer_id: o.from_peer_id, rev: 1 }), (e) => e.details.rev === o.rev && e.details.from_peer_id === o.from_peer_id);
  assert.equal(estado.layout.links.length, ligacoes);
  assert.ok(oferta());
});

test("a oferta certa é aceita uma vez e a segunda resposta dá no_offer", async () => {
  const o = oferta();
  const ligacoes = estado.layout.links.length;
  const r = await chamar({ accept: true, from_peer_id: o.from_peer_id, rev: o.rev });
  assert.equal(typeof r.layout_rev, "number");
  assert.equal(oferta(), null);
  assert.equal(estado.layout.links.length, ligacoes + o.links.length);
  await rejeita(chamar({ accept: true, from_peer_id: o.from_peer_id, rev: o.rev }), "no_offer");
});
