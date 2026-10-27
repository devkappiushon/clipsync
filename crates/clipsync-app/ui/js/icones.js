// Ícones de traço em SVG, injetados uma vez como <symbol>. Uso: icone("pausa").
// Cada símbolo é uma lista de formas [tag, atributos], montada com createElementNS: nada de HTML em texto.

const SIMBOLOS = {
  dispositivos: [
    ["rect", { x: 3, y: 4, width: 18, height: 12, rx: 2 }],
    ["path", { d: "M8 20h8M12 16v4" }],
  ],
  layout: [
    ["rect", { x: 3, y: 4, width: 10, height: 8, rx: 1.5 }],
    ["rect", { x: 15, y: 9, width: 6, height: 11, rx: 1.5 }],
    ["rect", { x: 3, y: 14, width: 10, height: 6, rx: 1.5 }],
  ],
  clipboard: [
    ["rect", { x: 6, y: 5, width: 12, height: 16, rx: 2 }],
    ["path", { d: "M9 5V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v1M9 11h6M9 15h4" }],
  ],
  diagnostico: [["path", { d: "M3 12h4l3-8 4 16 3-8h4" }]],
  preferencias: [
    ["path", { d: "M4 7h10M18 7h2M4 17h2M10 17h10" }],
    ["circle", { cx: 16, cy: 7, r: 2 }],
    ["circle", { cx: 8, cy: 17, r: 2 }],
  ],
  pausa: [["path", { d: "M9 5v14M15 5v14" }]],
  retomar: [["path", { d: "M7 4.5v15l12-7.5z" }]],
  copiar: [
    ["rect", { x: 8, y: 8, width: 12, height: 12, rx: 2 }],
    ["path", { d: "M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2" }],
  ],
  descartar: [["path", { d: "M5 7h14M10 7V5h4v2M7 7l1 12h8l1-12" }]],
  mais: [["path", { d: "M12 5v14M5 12h14" }]],
  fechar: [["path", { d: "M6 6l12 12M18 6 6 18" }]],
  voltar: [["path", { d: "M15 5l-7 7 7 7" }]],
  avancar: [["path", { d: "M9 5l7 7-7 7" }]],
  enviar: [["path", { d: "M4 12h12M11 6l6 6-6 6M20 5v14" }]],
  baixar: [["path", { d: "M12 4v11M7 10l5 5 5-5M5 20h14" }]],
  olho: [
    ["path", { d: "M2.5 12s3.5-6 9.5-6 9.5 6 9.5 6-3.5 6-9.5 6-9.5-6-9.5-6z" }],
    ["circle", { cx: 12, cy: 12, r: 3 }],
  ],
  cadeado: [
    ["rect", { x: 5, y: 11, width: 14, height: 9, rx: 2 }],
    ["path", { d: "M8 11V8a4 4 0 0 1 8 0v3" }],
  ],
  destravado: [
    ["rect", { x: 5, y: 11, width: 14, height: 9, rx: 2 }],
    ["path", { d: "M8 11V8a4 4 0 0 1 7.5-2" }],
  ],
  atualizar: [["path", { d: "M20 12a8 8 0 1 1-2.3-5.7M20 4v5h-5" }]],
  verificar: [["path", { d: "m5 12.5 4.5 4.5L19 7.5" }]],
  monitor: [
    ["rect", { x: 3, y: 4, width: 18, height: 12, rx: 2 }],
    ["path", { d: "M8 20h8M12 16v4" }],
  ],
  teclado: [
    ["rect", { x: 3, y: 6, width: 18, height: 12, rx: 2 }],
    ["path", { d: "M7 10h.1M11 10h.1M15 10h.1M7 14h10" }],
  ],
  "estado-ok": [
    ["circle", { cx: 12, cy: 12, r: 9 }],
    ["path", { d: "m8 12.5 2.8 2.8L16 9.5" }],
  ],
  "estado-aviso": [
    ["path", { d: "M12 3.5 21 19.5H3z" }],
    ["path", { d: "M12 10v4.5M12 17.2v.1" }],
  ],
  "estado-erro": [
    ["circle", { cx: 12, cy: 12, r: 9 }],
    ["path", { d: "m9 9 6 6M15 9l-6 6" }],
  ],
  "estado-neutro": [
    ["circle", { cx: 12, cy: 12, r: 9 }],
    ["path", { d: "M8.5 12h7" }],
  ],
  "estado-acento": [
    ["circle", { cx: 12, cy: 12, r: 9 }],
    ["circle", { cx: 12, cy: 12, r: 3.5, fill: "currentColor" }],
  ],
};

const NS = "http://www.w3.org/2000/svg";

/** Cria um elemento SVG com atributos. Também serve às telas que desenham em SVG (layout). */
export function svg(tag, atributos = {}, ...filhos) {
  const no = document.createElementNS(NS, tag);
  for (const [chave, valor] of Object.entries(atributos)) {
    if (valor == null || valor === false) continue;
    no.setAttribute(chave, valor === true ? "" : String(valor));
  }
  no.append(...filhos.flat(Infinity).filter((f) => f != null && f !== false));
  return no;
}

function injetarSprite() {
  if (document.getElementById("sprite-icones")) return;
  const sprite = svg("svg", { id: "sprite-icones", "aria-hidden": "true", width: 0, height: 0, focusable: "false" });
  sprite.classList.add("sprite");
  for (const [nome, formas] of Object.entries(SIMBOLOS)) {
    sprite.append(svg("symbol", { id: `i-${nome}`, viewBox: "0 0 24 24" }, formas.map(([tag, atributos]) => svg(tag, atributos))));
  }
  document.body.prepend(sprite);
}

injetarSprite();

export function icone(nome) {
  if (!(nome in SIMBOLOS)) throw new Error(`ícone desconhecido: ${nome}`);
  return svg("svg", { class: "icone", "aria-hidden": "true" }, svg("use", { href: `#i-${nome}` }));
}
