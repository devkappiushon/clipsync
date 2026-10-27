// Ícones de traço em SVG, injetados uma vez como <symbol>. Uso: icone("pausa").

const SIMBOLOS = {
  dispositivos: '<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>',
  layout: '<rect x="3" y="4" width="10" height="8" rx="1.5"/><rect x="15" y="9" width="6" height="11" rx="1.5"/><rect x="3" y="14" width="10" height="6" rx="1.5"/>',
  clipboard: '<rect x="6" y="5" width="12" height="16" rx="2"/><path d="M9 5V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v1M9 11h6M9 15h4"/>',
  diagnostico: '<path d="M3 12h4l3-8 4 16 3-8h4"/>',
  preferencias: '<path d="M4 7h10M18 7h2M4 17h2M10 17h10"/><circle cx="16" cy="7" r="2"/><circle cx="8" cy="17" r="2"/>',
  pausa: '<path d="M9 5v14M15 5v14"/>',
  retomar: '<path d="M7 4.5v15l12-7.5z"/>',
  copiar: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/>',
  descartar: '<path d="M5 7h14M10 7V5h4v2M7 7l1 12h8l1-12"/>',
  mais: '<path d="M12 5v14M5 12h14"/>',
  "estado-ok": '<circle cx="12" cy="12" r="9"/><path d="m8 12.5 2.8 2.8L16 9.5"/>',
  "estado-aviso": '<path d="M12 3.5 21 19.5H3z"/><path d="M12 10v4.5M12 17.2v.1"/>',
  "estado-erro": '<circle cx="12" cy="12" r="9"/><path d="m9 9 6 6M15 9l-6 6"/>',
  "estado-neutro": '<circle cx="12" cy="12" r="9"/><path d="M8.5 12h7"/>',
  "estado-acento": '<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="3.5" fill="currentColor"/>',
};

const NS = "http://www.w3.org/2000/svg";

function injetarSprite() {
  if (document.getElementById("sprite-icones")) return;
  const sprite = document.createElementNS(NS, "svg");
  sprite.id = "sprite-icones";
  sprite.setAttribute("aria-hidden", "true");
  sprite.setAttribute("width", "0");
  sprite.setAttribute("height", "0");
  sprite.style.position = "absolute";
  sprite.innerHTML = Object.entries(SIMBOLOS)
    .map(([nome, desenho]) => `<symbol id="i-${nome}" viewBox="0 0 24 24">${desenho}</symbol>`)
    .join("");
  document.body.prepend(sprite);
}

injetarSprite();

export function icone(nome) {
  if (!(nome in SIMBOLOS)) throw new Error(`ícone desconhecido: ${nome}`);
  const svg = document.createElementNS(NS, "svg");
  svg.setAttribute("class", "icone");
  svg.setAttribute("aria-hidden", "true");
  const uso = document.createElementNS(NS, "use");
  uso.setAttribute("href", `#i-${nome}`);
  svg.append(uso);
  return svg;
}
