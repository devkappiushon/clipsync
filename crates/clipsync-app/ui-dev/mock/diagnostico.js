// Mock do domínio diagnóstico: itens por área (sistema, adaptadores, rede, conflitos), rede com pares,
// últimos bloqueios (sem conteúdo) e pacote de suporte com prévia redigida e exportação para Downloads.

import { caminhoLog, EU } from "./dados.js";
import { atraso, cenario, erro, estado, plataforma, registrar } from "./nucleo.js";

const AGORA = Date.now();
const MIN = 60_000;

function itensLinux() {
  return [
    { area: "sistema", name: "Sistema operacional", status: "ok", detail: "Ubuntu 26.04.1 LTS · GNOME 50.1 · Wayland" },
    { area: "sistema", name: "XWayland", status: "ok", detail: "DISPLAY=:0, servidor X11 respondendo" },
    { area: "sistema", name: "Portais", status: "ok", detail: "InputCapture v1 · RemoteDesktop v2 (teclado, ponteiro, touchscreen)" },
    { area: "sistema", name: "Monitores", status: "ok", detail: "eDP-1 1920×1080 @ 0,0 (escala 1,5)" },
    { area: "sistema", name: "Layout do teclado", status: "ok", detail: "xkb: br(abnt2)" },
    { area: "adaptadores", name: "Clipboard", status: estado.modulos.clipboard ? "ok" : "missing", detail: estado.modulos.clipboard ? "x11-xfixes · data-control ausente no compositor: observação pela ponte XWayland · plataforma validada" : "Módulo desligado nesta máquina." },
    { area: "adaptadores", name: "Captura de entrada", status: estado.modulos.input ? "ok" : "missing", detail: estado.modulos.input ? "portal-inputcapture · 1 diálogo por sessão · experimental" : "Módulo desligado nesta máquina." },
    { area: "adaptadores", name: "Emulação de entrada", status: estado.modulos.input ? "ok" : "missing", detail: estado.modulos.input ? "portal-remotedesktop · token salvo, sem novo diálogo" : "Módulo desligado nesta máquina." },
    { area: "conflitos", name: "Input Leap", status: "warn", detail: "Em execução (Flatpak, cliente de 192.168.115.30): os dois podem disputar teclado e mouse. Pause um deles." },
    { area: "conflitos", name: "Barrier", status: "ok", detail: "Não detectado." },
    { area: "conflitos", name: "Gerenciador de clipboard", status: "warn", detail: "GPaste ativo: ele pode reescrever o clipboard logo depois de uma aplicação. Se um item sumir, desligue o GPaste." },
  ];
}

function itensWindows() {
  return [
    { area: "sistema", name: "Sistema operacional", status: "ok", detail: "Windows 11 Pro 24H2 (build 26100)" },
    { area: "sistema", name: "Sessão", status: "ok", detail: "Área de trabalho interativa, sem RDP" },
    { area: "sistema", name: "Monitores", status: "ok", detail: "DELL U2723QE 3840×2160 @ 0,0 (escala 1,5) · LG 24GL600F 1920×1080 @ 3840,360" },
    { area: "sistema", name: "Layout do teclado", status: "ok", detail: "00010416 · Português (Brasil ABNT2)" },
    { area: "adaptadores", name: "Clipboard", status: "ok", detail: "win32 · plataforma experimental" },
    { area: "adaptadores", name: "Captura de entrada", status: estado.modulos.input ? "warn" : "missing", detail: estado.modulos.input ? "win-hooks · experimental, ainda sem prova nesta máquina" : "Módulo desligado nesta máquina." },
    { area: "adaptadores", name: "Emulação de entrada", status: estado.modulos.input ? "warn" : "missing", detail: estado.modulos.input ? "win-sendinput · experimental, ainda sem prova nesta máquina" : "Módulo desligado nesta máquina." },
    { area: "adaptadores", name: "Atalhos ativos", status: estado.modulos.input ? "ok" : "missing", detail: estado.modulos.input ? "Win+Alt+1 a 9, Win+Alt+L e Win+Alt+P · Win+Alt+C e Win+Alt+V indisponíveis nesta versão" : "Módulo de teclado e mouse desligado." },
    { area: "conflitos", name: "Mouse Without Borders", status: "ok", detail: "Não detectado." },
  ];
}

function rede() {
  const pares = estado.dispositivos.map((d) => ({ peer_id: d.peer_id, name: d.name, state: d.state, addr: d.addr }));
  return {
    listen: "0.0.0.0:47800",
    addresses: ["192.168.115.12:47800", "10.8.0.3:47800"],
    peers: pares,
  };
}

function bloqueios() {
  if (cenario === "vazio") return [];
  if (cenario === "b01-grupos") {
    // Dados fictícios para conferir grupos, legado, saturação JSON e reflow na tabela B01.
    return [
      { at_unix_ms: AGORA - 45 * MIN, peer_id: "a".repeat(64), peer_name: "par-legado-dev", module: "input", reason: "Entrada legada fictícia sem agregação." },
      { at_unix_ms: AGORA - 35 * MIN, peer_id: "b".repeat(64), peer_name: "par-grupos-dev", module: "clipboard", reason: "Oferta fictícia recusada: malha pausada.", occurrences: 7, last_unix_ms: AGORA - 2 * MIN },
      { at_unix_ms: AGORA - 25 * MIN, peer_id: "b".repeat(64), peer_name: "par-grupos-dev", module: "clipboard", reason: "Oferta fictícia recusada: permissão ausente.", occurrences: 3, last_unix_ms: AGORA - 5 * MIN },
      { at_unix_ms: AGORA - 20 * MIN, peer_id: null, peer_name: null, module: null, reason: "Conexão fictícia recusada: chave desconhecida.", occurrences: 4, last_unix_ms: AGORA - 6 * MIN },
      { at_unix_ms: AGORA - 60 * MIN, peer_id: "c".repeat(64), peer_name: "par-saturado-dev", module: "input", reason: "Recusa fictícia com contador saturado.", occurrences: JSON.parse("18446744073709551615"), last_unix_ms: AGORA - MIN },
      { at_unix_ms: AGORA - 50 * MIN, peer_id: "d".repeat(64), peer_name: "estação-fictícia-de-validação-do-refluxo-de-texto-com-nome-muito-longo-para-a-tabela-de-diagnóstico", module: "input", reason: "Este bloqueio fictício tem um motivo deliberadamente longo para verificar a quebra de linhas na tabela: o destino recusou a emulação de teclado e mouse enquanto a sessão estava pausada, e o controle permaneceu nesta máquina até uma nova tentativa autorizada.", occurrences: 2, last_unix_ms: AGORA - 8 * MIN },
    ];
  }
  const casa = estado.dispositivos.find((d) => d.name === "pc-de-casa");
  const antigo = estado.dispositivos.find((d) => d.name === "notebook-antigo");
  // S7 (contrato §13.6 item 9): quando o destino recusa emular, a origem registra o motivo como bloqueio.
  const pessoal = estado.dispositivos.find((d) => d.name === "pc-pessoal");
  const recusas =
    plataforma === "windows"
      ? [
          { at_unix_ms: AGORA - 4 * MIN, peer_id: pessoal?.peer_id ?? null, peer_name: "pc-pessoal", module: "input", reason: "pc-pessoal está com a tela bloqueada: o controle voltou para esta máquina" },
          { at_unix_ms: AGORA - 11 * MIN, peer_id: casa?.peer_id ?? null, peer_name: "pc-de-casa", module: "input", reason: "pc-de-casa está pausado: o controle voltou para esta máquina" },
          { at_unix_ms: AGORA - 19 * MIN, peer_id: antigo?.peer_id ?? null, peer_name: "notebook-antigo", module: "input", reason: "notebook-antigo não deixou emular teclado e mouse" },
        ]
      : [];
  // I-S7-2 (§13.2 regra 2g): o Windows não liberou Win+Alt+P para retomar. O texto é o do contrato.
  const retomada = estado.bloqueioRecusa
    ? [{ at_unix_ms: AGORA - 2 * MIN, peer_id: null, peer_name: null, module: "input", reason: "o Windows não liberou Win+Alt+P para retomar: use a bandeja ou o botão Pausar tudo" }]
    : [];
  return [
    ...retomada,
    ...recusas,
    { at_unix_ms: AGORA - 28 * MIN, peer_id: casa?.peer_id ?? null, peer_name: "pc-de-casa", module: "clipboard", reason: "Oferta recusada: malha pausada." },
    { at_unix_ms: AGORA - 3 * 60 * MIN, peer_id: antigo?.peer_id ?? null, peer_name: "notebook-antigo", module: "input", reason: "Par sem permissão de controle tentou entrar." },
    { at_unix_ms: AGORA - 26 * 60 * MIN, peer_id: null, peer_name: null, module: null, reason: "Conexão de 192.168.115.90 recusada: chave desconhecida." },
  ];
}

const IP = /\b\d{1,3}(?:\.\d{1,3}){3}\b/g;

/** Pacote de suporte redigido como o crate faz: ids, nomes e IPs saem; o texto nunca leva conteúdo. */
function pacoteSuporte() {
  const contagem = { peer_id: 0, peer_name: 0, ip: 0, device_name: 1 };
  const semIp = (texto) =>
    texto.replace(IP, () => {
      contagem.ip += 1;
      return "[ip removido]";
    });
  const pares = estado.dispositivos.map((d, i) => {
    contagem.peer_id += 1;
    contagem.peer_name += 1;
    return semIp(`par ${i + 1}: [nome removido] · [id removido] · ${d.state} · ${d.addr}`);
  });
  const blocos = bloqueios().map((b) => {
    if (b.peer_id) contagem.peer_id += 1;
    if (b.peer_name) contagem.peer_name += 1;
    return semIp(`${new Date(b.at_unix_ms).toISOString()} · ${b.peer_name ? "[nome removido]" : "sem par"} · ${b.module ?? "geral"} · ${b.reason}`);
  });
  const linhas = [
    "ClipSync 0.2.0 · pacote de suporte",
    `plataforma: ${plataforma}`,
    "dispositivo: [nome removido] · id: [removido]",
    `agente: ${estado.agente} · pausado: ${estado.pausado ? "sim" : "não"}`,
    `módulos: clipboard=${estado.modulos.clipboard ? "ligado" : "desligado"} input=${estado.modulos.input ? "ligado" : "desligado"}`,
    "",
    "[sistema]",
    ...(plataforma === "linux" ? itensLinux() : itensWindows()).map((i) => semIp(`${i.area}/${i.name}: ${i.status} · ${i.detail}`)),
    "",
    "[rede]",
    semIp(`escuta: ${rede().listen} · endereços: ${rede().addresses.join(", ")}`),
    ...pares,
    "",
    "[últimos bloqueios]",
    ...blocos,
    "",
    "Não entram: chave privada, peers.toml, portal-tokens.toml, conteúdo de clipboard.",
  ];
  const redactions = Object.entries(contagem)
    .filter(([, count]) => count > 0)
    .map(([kind, count]) => ({ kind, count }));
  return { text: linhas.join("\n"), redactions };
}

registrar({
  diagnostics_get: () => ({
    log_path: caminhoLog(),
    items: plataforma === "linux" ? itensLinux() : itensWindows(),
    network: rede(),
    blocks: bloqueios(),
  }),

  support_bundle_preview: () => pacoteSuporte(),

  support_bundle_export: async () => {
    await atraso(700);
    if (cenario === "estados") throw erro("io", "Não foi possível gravar em Downloads: pasta sem permissão de escrita.");
    const texto = pacoteSuporte().text;
    const carimbo = new Date().toISOString().replace(/[-:]/g, "").replace("T", "-").slice(0, 15);
    const pasta = plataforma === "windows" ? "C:\\Users\\joao\\Downloads\\" : "/home/joao/Downloads/";
    return { path: `${pasta}clipsync-suporte-${carimbo}.txt`, bytes: new TextEncoder().encode(texto).length };
  },
});

// EU fica importado para o nome não vazar no texto: o pacote nunca o inclui.
void EU;
