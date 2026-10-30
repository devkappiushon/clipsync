// Mock do domínio clipboard: estado do módulo, modo, caixa de recebidos (sem preview na lista),
// prévia sob demanda, copiar, descartar e enviar agora. Texto dos itens fica fora do InboxItem.

import { agenteDePe, atraso, cenario, emitir, erro, estado, plataforma, registrar } from "./nucleo.js";

const AGORA = Date.now();
const MIN = 60_000;
const RETENCAO = { max_items: 50, ttl_ms: 10 * MIN };

/** id de 32 hex a partir de um short de 8 hex, completado de forma determinística. */
function itemId(short) {
  let id = short.replace("-", "");
  while (id.length < 32) id += ((id.charCodeAt(id.length - 2) * 17 + id.length) % 16).toString(16);
  return id;
}

const textos = new Map();

function item({ short, from, haMin, bytes, state, reason = null, texto }) {
  const id = itemId(short);
  textos.set(id, texto);
  return { id, short, from, received_unix_ms: AGORA - haMin * MIN, mime: "text/plain", bytes, state, reason };
}

function caixaPadrao() {
  if (cenario === "vazio") return [];
  return [
    item({ short: "9f1c-20aa", from: "pc-pessoal", haMin: 0.7, bytes: 64, state: "in_inbox", texto: "https://docs.rs/tauri/2.12.5/tauri/struct.Builder.html#method.setup" }),
    item({ short: "4b77-e310", from: "pc-pessoal", haMin: 2, bytes: 10, state: "applied", texto: "olá ção" }),
    item({ short: "a2d0-5c19", from: "pc-de-casa", haMin: 28, bytes: 2150, state: "rejected", reason: "Recebido com a malha pausada.", texto: "Ata da reunião de 27/10: 1) revisar o contrato §12 com o Keel; 2) fechar a release 0.2.0 com os quatro pacotes; 3) testar o AppImage no notebook e no pc de casa. Pendências: ícones da bandeja, pacote de suporte, atalhos globais no Wayland, tradução dos textos." }),
    item({ short: "e8c3-11f4", from: "notebook-antigo", haMin: 9, bytes: 31, state: "expired", texto: "" }),
  ];
}

estado.caixa = caixaPadrao();
estado.clipboard = {
  mode: "auto",
  adapter: estado.modulos.clipboard ? (plataforma === "windows" ? "win32" : "x11-xfixes") : null,
  classification: estado.modulos.clipboard ? "validated" : "unavailable",
  last_event: cenario === "vazio" ? null : "recebido de pc-pessoal há 1 minuto",
  last_block: cenario === "vazio" ? null : "oferta de pc-de-casa recusada há 28 minutos: malha pausada",
};

function achar(id) {
  const encontrado = estado.caixa.find((i) => i.id === id);
  if (!encontrado) throw erro("no_item", "Este item não está mais na caixa.");
  return encontrado;
}

registrar({
  clipboard_get: () => ({
    enabled: estado.modulos.clipboard,
    adapter: estado.clipboard.adapter,
    classification: estado.clipboard.classification,
    mode: estado.clipboard.mode,
    last_event: estado.clipboard.last_event,
    last_block: estado.clipboard.last_block,
    inbox: [...estado.caixa].sort((a, b) => b.received_unix_ms - a.received_unix_ms),
    retention: { ...RETENCAO },
  }),

  clipboard_preview: ({ id } = {}) => {
    achar(id);
    return { preview: (textos.get(id) ?? "").slice(0, 200) };
  },

  clipboard_copy_item: ({ id } = {}) => {
    const encontrado = achar(id);
    if (encontrado.state !== "in_inbox" && encontrado.state !== "rejected") {
      throw erro("wrong_state", "Só itens na caixa ou recusados podem ser copiados.");
    }
    if (!textos.get(id)) throw erro("wrong_state", "Este item não tem mais texto guardado.");
    encontrado.state = "applied";
    encontrado.reason = null;
    estado.clipboard.last_event = `aplicado de ${encontrado.from} agora`;
    return { state: "applied" };
  },

  clipboard_discard_item: ({ id } = {}) => {
    achar(id);
    estado.caixa = estado.caixa.filter((i) => i.id !== id);
    textos.delete(id);
    return { removed: true };
  },

  clipboard_send_now: async ({ peer_id = null } = {}) => {
    const alvos = estado.dispositivos.filter((d) => d.perms.clipboard_out && (peer_id === null || d.peer_id === peer_id));
    if (alvos.length === 0) throw erro("no_target", "Nenhum par pode receber o seu clipboard. Conceda 'Pode receber meu clipboard' em Dispositivos.");
    if (cenario === "vazio") throw erro("empty_clipboard", "O clipboard está vazio: copie um texto antes de enviar.");
    await atraso(500);
    const outcomes = alvos.map((d) => {
      if (d.state !== "connected") return { peer_id: d.peer_id, name: d.name, state: "unreachable", reason: "Par offline." };
      if (!d.remote_modules.clipboard) return { peer_id: d.peer_id, name: d.name, state: "rejected", reason: "O par não tem o módulo de clipboard." };
      return { peer_id: d.peer_id, name: d.name, state: "applied", reason: null };
    });
    estado.clipboard.last_event = `enviado a ${outcomes.length} ${outcomes.length === 1 ? "par" : "pares"} agora`;
    return { outcomes };
  },
});

// Simulação de chegada: só no cenário estados, 6 s depois de abrir, um item entra na caixa.
if (cenario === "estados" && agenteDePe()) {
  setTimeout(() => {
    const novo = item({ short: "77d1-0b2e", from: "pc-pessoal", haMin: 0, bytes: 23, state: "in_inbox", texto: "senha do wi-fi: casa-2026" });
    novo.received_unix_ms = Date.now();
    estado.caixa = [novo, ...estado.caixa];
    estado.clipboard.last_event = "recebido de pc-pessoal agora";
    emitir("clip_received", { id: novo.id, short: novo.short, from: novo.from, bytes: novo.bytes, state: novo.state });
    emitir("notification", { kind: "item_received", title: "Clipboard recebido", body: "pc-pessoal enviou 23 bytes." });
  }, 6000);
}
