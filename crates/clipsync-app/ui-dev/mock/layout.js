// Mock do domínio layout (tela 4): plano virtual com os monitores de cada máquina, ligações entre bordas,
// bordas internas travadas, opções, oferta aos pares, trava de tela, troca de dispositivo e teste de entrada.
// Validações do layout_set_link na ordem do contrato §12.5.

import { EU, peerId } from "./dados.js";
import { atraso, cenario, emitir, erro, estado, plataforma, registrar } from "./nucleo.js";

const SNAP = 16;
const BORDAS = ["left", "right", "top", "bottom"];

function monitor(ref, index, x, y, width, height, scale = 1) {
  return { ref, name: ref, index, x, y, width, height, scale };
}

function dispositivoBase(peer, extra) {
  return { peer_id: peer.peer_id, name: peer.name, is_self: peer.peer_id === EU.peer_id, state: peer.state ?? "connected", hotkey: null, monitors_source: "live", ...extra };
}

const PESSOAL = peerId("aa31-537a");
const CASA = peerId("1bc6-7a55");

/** PC Windows da S7: dois monitores com o nome amigável, em px físicos; o 4K roda a 150%. No Windows o nome serve de `ref`. */
function layoutWindows() {
  const devices = [
    dispositivoBase({ peer_id: EU.peer_id, name: EU.name }, { x: 0, y: 0, hotkey: null, monitors: [monitor("DELL U2723QE", 0, 0, 0, 3840, 2160, 1.5), monitor("LG 24GL600F", 1, 3840, 360, 1920, 1080)] }),
    dispositivoBase({ peer_id: PESSOAL, name: "pc-pessoal" }, { x: 5920, y: 360, hotkey: 1, monitors: [monitor("eDP-1", 0, 0, 0, 1920, 1080, 1.5)] }),
    dispositivoBase({ peer_id: CASA, name: "pc-de-casa", state: "offline" }, { x: 0, y: 2320, hotkey: 2, monitors_source: "cached", monitors: [monitor("eDP-1", 0, 0, 0, 1920, 1080)] }),
  ];
  const links = [
    {
      id: "l1",
      a: { peer_id: EU.peer_id, monitor: "LG 24GL600F", edge: "right", from: 0, to: 1 },
      b: { peer_id: PESSOAL, monitor: "eDP-1", edge: "left", from: 0, to: 1 },
      problem: null,
    },
  ];
  return { devices, links };
}

function layoutPadrao() {
  if (plataforma === "windows") return layoutWindows();
  const devices = [
    dispositivoBase({ peer_id: PESSOAL, name: "pc-pessoal" }, { x: 0, y: 0, hotkey: 2, monitors: [monitor("DP-1", 0, 0, 0, 2560, 1440), monitor("HDMI-1", 1, 2560, 180, 1920, 1080)] }),
    dispositivoBase({ peer_id: EU.peer_id, name: EU.name }, { x: 4640, y: 360, hotkey: 1, monitors: [monitor("eDP-1", 0, 0, 0, 1920, 1080, 1.5)] }),
    dispositivoBase({ peer_id: CASA, name: "pc-de-casa", state: "offline" }, { x: 0, y: 1696, hotkey: 3, monitors_source: "cached", monitors: [monitor("eDP-1", 0, 0, 0, 1920, 1080)] }),
  ];
  const links = [
    {
      id: "l1",
      a: { peer_id: PESSOAL, monitor: "HDMI-1", edge: "right", from: 0.5, to: 1 },
      b: { peer_id: EU.peer_id, monitor: "eDP-1", edge: "left", from: 0, to: 1 },
      problem: null,
    },
  ];
  if (cenario === "vazio") return { devices: devices.filter((d) => d.is_self), links: [] };
  return { devices, links };
}

const base = layoutPadrao();
estado.layout = {
  rev: 7,
  self_id: EU.peer_id,
  snap_px: SNAP,
  options: { dead_corner_px: 8 },
  active_peer_id: null,
  devices: base.devices,
  links: base.links,
  pending_offer:
    cenario === "estados"
      ? {
          from_peer_id: PESSOAL,
          from_name: "pc-pessoal",
          rev: 9,
          links: [
            {
              id: "o1",
              a: { peer_id: PESSOAL, monitor: "DP-1", edge: "bottom", from: 0, to: 0.75 },
              b: { peer_id: CASA, monitor: "eDP-1", edge: "top", from: 0, to: 1 },
              problem: null,
            },
          ],
        }
      : null,
  proximoLink: 2,
};

/** Bordas entre monitores da mesma máquina: trechos onde dois monitores se tocam. */
function bordasInternas(device) {
  const travadas = [];
  const ms = device.monitors;
  for (const a of ms) {
    for (const b of ms) {
      if (a === b) continue;
      // a à esquerda de b
      if (a.x + a.width === b.x) {
        const ini = Math.max(a.y, b.y);
        const fim = Math.min(a.y + a.height, b.y + b.height);
        if (fim > ini) {
          travadas.push({ peer_id: device.peer_id, monitor: a.ref, edge: "right", from: (ini - a.y) / a.height, to: (fim - a.y) / a.height });
          travadas.push({ peer_id: device.peer_id, monitor: b.ref, edge: "left", from: (ini - b.y) / b.height, to: (fim - b.y) / b.height });
        }
      }
      // a acima de b
      if (a.y + a.height === b.y) {
        const ini = Math.max(a.x, b.x);
        const fim = Math.min(a.x + a.width, b.x + b.width);
        if (fim > ini) {
          travadas.push({ peer_id: device.peer_id, monitor: a.ref, edge: "bottom", from: (ini - a.x) / a.width, to: (fim - a.x) / a.width });
          travadas.push({ peer_id: device.peer_id, monitor: b.ref, edge: "top", from: (ini - b.x) / b.width, to: (fim - b.x) / b.width });
        }
      }
    }
  }
  return travadas;
}

function arredondar(n) {
  return Math.round(n * 1000) / 1000;
}

function lockedEdges() {
  return estado.layout.devices.flatMap(bordasInternas).map((e) => ({ ...e, from: arredondar(e.from), to: arredondar(e.to) }));
}

function sobrepoe(p, q) {
  return p.peer_id === q.peer_id && p.monitor === q.monitor && p.edge === q.edge && p.from < q.to && q.from < p.to;
}

function dispositivoDe(peer) {
  const d = estado.layout.devices.find((x) => x.peer_id === peer);
  if (!d) throw erro("unknown_peer", "Esse dispositivo não está no layout.");
  return d;
}

function validarEndpoint(e, lado) {
  if (!e || typeof e !== "object") throw erro("bad_request", `Falta a ponta ${lado} da ligação.`);
  const d = dispositivoDe(e.peer_id);
  const m = d.monitors.find((x) => x.ref === e.monitor);
  if (!m) throw erro("bad_request", `O monitor ${e.monitor} não existe em ${d.name}.`, { field: `${lado}.monitor` });
  if (!BORDAS.includes(e.edge)) throw erro("bad_request", "Borda inválida.", { field: `${lado}.edge` });
  if (typeof e.from !== "number" || typeof e.to !== "number") throw erro("bad_request", "from e to precisam ser números.", { field: lado });
  return { peer_id: e.peer_id, monitor: e.monitor, edge: e.edge, from: e.from, to: e.to };
}

function comprimentoPx(e) {
  const d = dispositivoDe(e.peer_id);
  const m = d.monitors.find((x) => x.ref === e.monitor);
  return e.edge === "left" || e.edge === "right" ? m.height : m.width;
}

/** Aplica o canto morto: encurta o trecho nas pontas que encostam num canto. */
function aparar(e) {
  const morto = estado.layout.options.dead_corner_px / comprimentoPx(e);
  const from = Math.max(e.from, morto);
  const to = Math.min(e.to, 1 - morto);
  return { ponta: { ...e, from: arredondar(from), to: arredondar(to) }, aparou: from > e.from || to < e.to };
}

function estadoDosPares() {
  for (const d of estado.layout.devices) {
    if (d.is_self) {
      d.name = EU.name;
      continue;
    }
    const par = estado.dispositivos.find((p) => p.peer_id === d.peer_id);
    d.state = par ? par.state : "offline";
  }
}

function layoutAtual() {
  estadoDosPares();
  const { proximoLink, ...resto } = estado.layout;
  return { ...resto, screen_locked: estado.telaTravada, locked_edges: lockedEdges() };
}

function exigirRev(base_rev) {
  if (base_rev !== estado.layout.rev) {
    throw erro("stale_rev", "O layout mudou desde que a tela foi carregada. Recarregue e tente de novo.", { details: { rev: estado.layout.rev } });
  }
}

function gravar(reason = "saved") {
  estado.layout.rev += 1;
  emitir("layout_changed", { rev: estado.layout.rev, reason });
  return estado.layout.rev;
}

function exigirInput() {
  if (!estado.modulos.input) throw erro("input_off", "O módulo de teclado e mouse está desligado nesta máquina.");
}

registrar({
  layout_get: () => {
    exigirInput();
    return layoutAtual();
  },

  layout_set_link: ({ id = null, a, b, base_rev } = {}) => {
    exigirInput();
    const pa = validarEndpoint(a, "a");
    const pb = validarEndpoint(b, "b");
    if (pa.peer_id === pb.peer_id) throw erro("same_device", "As duas pontas estão na mesma máquina. Ligue bordas de máquinas diferentes.");
    const travadas = lockedEdges();
    for (const [lado, p] of [["a", pa], ["b", pb]]) {
      if (travadas.some((t) => sobrepoe(t, p))) {
        throw erro("internal_edge_locked", "Essa borda fica entre dois monitores da mesma máquina e não cruza para outra.", { field: lado });
      }
    }
    for (const [lado, p] of [["a", pa], ["b", pb]]) {
      if (p.from >= p.to) throw erro("empty_segment", "O trecho ficou vazio: o começo precisa vir antes do fim.", { field: lado });
      if (p.from < 0 || p.to > 1) throw erro("segment_out_of_range", "O trecho sai da borda: use valores entre 0 e 1.", { field: lado });
    }
    for (const [lado, p] of [["a", pa], ["b", pb]]) {
      const outro = estado.layout.links.find((l) => l.id !== id && (sobrepoe(l.a, p) || sobrepoe(l.b, p)));
      if (outro) throw erro("segment_overlap", `Esse trecho já está ligado por ${outro.id}. Encurte um dos dois ou remova a ligação.`, { field: lado, details: { link: outro.id } });
    }
    exigirRev(base_rev);
    const warnings = [];
    const aparadoA = aparar(pa);
    const aparadoB = aparar(pb);
    if (aparadoA.aparou) warnings.push({ code: "trimmed_by_dead_corner", side: "a" });
    if (aparadoB.aparou) warnings.push({ code: "trimmed_by_dead_corner", side: "b" });
    let link;
    if (id === null) {
      link = { id: `l${estado.layout.proximoLink++}`, a: aparadoA.ponta, b: aparadoB.ponta, problem: null };
      estado.layout.links.push(link);
    } else {
      link = estado.layout.links.find((l) => l.id === id);
      if (!link) throw erro("link_not_found", "Essa ligação não existe mais.");
      link.a = aparadoA.ponta;
      link.b = aparadoB.ponta;
      link.problem = null;
    }
    return { link: structuredClone(link), layout_rev: gravar(), warnings };
  },

  layout_remove_link: ({ id, base_rev } = {}) => {
    exigirInput();
    if (!estado.layout.links.some((l) => l.id === id)) throw erro("link_not_found", "Essa ligação não existe mais.");
    exigirRev(base_rev);
    estado.layout.links = estado.layout.links.filter((l) => l.id !== id);
    return { layout_rev: gravar() };
  },

  layout_set_device: ({ peer_id, x, y, hotkey = null, base_rev } = {}) => {
    exigirInput();
    const d = dispositivoDe(peer_id);
    if (hotkey !== null && (!Number.isInteger(hotkey) || hotkey < 1 || hotkey > 9)) throw erro("bad_request", "O atalho vai de 1 a 9.", { field: "hotkey" });
    if (hotkey !== null) {
      const dono = estado.layout.devices.find((o) => o.peer_id !== peer_id && o.hotkey === hotkey);
      if (dono) throw erro("hotkey_taken", `Win+Alt+${hotkey} já é de ${dono.name}.`, { field: "hotkey", details: { peer_id: dono.peer_id } });
    }
    exigirRev(base_rev);
    d.x = Math.round(x / SNAP) * SNAP;
    d.y = Math.round(y / SNAP) * SNAP;
    d.hotkey = hotkey;
    return { device: structuredClone(d), layout_rev: gravar() };
  },

  layout_set_options: ({ dead_corner_px, base_rev } = {}) => {
    exigirInput();
    if (!Number.isInteger(dead_corner_px) || dead_corner_px < 0 || dead_corner_px > 32) {
      throw erro("out_of_range", "Cantos mortos vão de 0 a 32 px.", { field: "dead_corner_px" });
    }
    const vazias = estado.layout.links.filter((l) =>
      [l.a, l.b].some((p) => {
        const morto = dead_corner_px / comprimentoPx(p);
        return Math.max(p.from, morto) >= Math.min(p.to, 1 - morto);
      }),
    );
    if (vazias.length > 0) {
      throw erro("empty_segment", "Com esse valor, uma ligação ficaria sem trecho. Encurte os cantos mortos ou ajuste a ligação.", { details: { links: vazias.map((l) => l.id) } });
    }
    exigirRev(base_rev);
    estado.layout.options = { dead_corner_px };
    return { options: { dead_corner_px }, layout_rev: gravar() };
  },

  layout_copy_to_peers: async ({ peer_ids = null } = {}) => {
    exigirInput();
    await atraso(600);
    const alvos = estado.layout.devices.filter((d) => !d.is_self && (peer_ids === null || peer_ids.includes(d.peer_id)));
    const results = alvos.map((d) => {
      const par = estado.dispositivos.find((p) => p.peer_id === d.peer_id);
      if (!par || par.state !== "connected") return { peer_id: d.peer_id, name: d.name, status: "offline", reason: "Par offline: a oferta não foi enviada." };
      if (!par.remote_modules.input) return { peer_id: d.peer_id, name: d.name, status: "unsupported", reason: "O par não tem o módulo de teclado e mouse." };
      if (!par.perms.input_in) return { peer_id: d.peer_id, name: d.name, status: "denied", reason: "O par não concedeu controle a esta máquina." };
      return { peer_id: d.peer_id, name: d.name, status: "offered", reason: null };
    });
    return { rev: estado.layout.rev, results };
  },

  layout_offer_answer: ({ accept, from_peer_id, rev } = {}) => {
    exigirInput();
    if (typeof accept !== "boolean") throw erro("bad_request", "Diga se aceita ou recusa a oferta.", { field: "accept" });
    if (typeof from_peer_id !== "string") throw erro("bad_request", "Falta o par que fez a oferta.", { field: "from_peer_id" });
    if (!Number.isInteger(rev)) throw erro("bad_request", "Falta a revisão da oferta.", { field: "rev" });
    const oferta = estado.layout.pending_offer;
    if (!oferta) throw erro("no_offer", "Não há oferta de layout pendente.");
    if (oferta.from_peer_id !== from_peer_id || oferta.rev !== rev) {
      throw erro("stale_offer", `A oferta mudou: agora quem oferece é ${oferta.from_name} (revisão ${oferta.rev}). Confira e responda de novo.`, {
        details: { from_peer_id: oferta.from_peer_id, rev: oferta.rev },
      });
    }
    estado.layout.pending_offer = null;
    if (accept !== true) return { layout_rev: estado.layout.rev };
    for (const l of oferta.links) {
      estado.layout.links = estado.layout.links.filter((x) => !(sobrepoe(x.a, l.a) || sobrepoe(x.b, l.b)));
      estado.layout.links.push({ ...l, id: `l${estado.layout.proximoLink++}` });
    }
    return { layout_rev: gravar("offer") };
  },

  lock_to_screen: ({ locked } = {}) => {
    exigirInput();
    estado.telaTravada = locked === true;
    emitir("screen_lock_changed", { screen_locked: estado.telaTravada, active_peer_id: estado.layout.active_peer_id });
    return { screen_locked: estado.telaTravada };
  },

  switch_to_device: ({ peer_id } = {}) => {
    exigirInput();
    const d = dispositivoDe(peer_id);
    estadoDosPares();
    if (d.state !== "connected") throw erro("offline", `${d.name} está offline.`);
    if (!estado.layout.links.some((l) => l.a.peer_id === peer_id || l.b.peer_id === peer_id)) {
      throw erro("not_linked", `${d.name} não tem ligação com esta máquina. Ligue uma borda primeiro.`);
    }
    const par = estado.dispositivos.find((p) => p.peer_id === peer_id);
    if (par && !par.remote_modules.input) throw erro("denied", `${d.name} não aceita controle desta máquina.`);
    estado.layout.active_peer_id = peer_id;
    emitir("screen_lock_changed", { screen_locked: estado.telaTravada, active_peer_id: peer_id });
    return { active_peer_id: peer_id };
  },

  input_test: async ({ peer_id, seconds = 5 } = {}) => {
    exigirInput();
    if (!Number.isInteger(seconds) || seconds < 5 || seconds > 60) throw erro("bad_request", "A duração vai de 5 a 60 segundos.", { field: "seconds" });
    const d = dispositivoDe(peer_id);
    estadoDosPares();
    if (d.state !== "connected") throw erro("offline", `${d.name} está offline.`);
    const par = estado.dispositivos.find((p) => p.peer_id === peer_id);
    if (par && !par.remote_modules.input) throw erro("denied", `${d.name} não aceita controle desta máquina.`);
    await atraso(1500);
    const received = seconds * 120;
    return { stats: { received, lost: 0, out_of_order: 1, p50_us: 410, p95_us: 1180, max_us: 3920 } };
  },
});
