// Ajustes pedidos em 2026-10-27 para o arquivo "ClipSync · telas"
// (https://www.figma.com/design/vkXY3Q37AaGe1argTA2STv). Rode no Scripter ou no MCP (use_figma).
// 1) Atalhos com a tecla Windows: "Ctrl+Alt" vira "Win+Alt" em todas as teclas de atalho e textos.
// 2) Malha de pontos de 16 px no canvas de "04 Layout de telas" (claro e escuro, se existir).
// 3) 02: o rótulo "É controlada por esta máquina" passa a quebrar linha em vez de encostar no interruptor.
// 4) 04: rótulo do notebook encurtado para "esta máquina".
const fonts = [["Inter","Regular"],["Inter","Medium"],["Inter","Semi Bold"],["Geist","SemiBold"],["Geist","Medium"],["Geist Mono","Regular"],["Geist Mono","Medium"]];
await Promise.all(fonts.map(([family, style]) => figma.loadFontAsync({ family, style })));
const page = figma.currentPage;
const out = { teclas: 0, textos: 0, malhas: 0, rotulos: 0, erros: [] };
const KBD_PROP = "Teclas#3:11";

// 1) Teclas de atalho (instâncias do componente "Tecla de atalho") e textos soltos
for (const inst of page.findAllWithCriteria({ types: ["INSTANCE"] })) {
  const main = await inst.getMainComponentAsync();
  if (!main || main.name !== "Tecla de atalho") continue;
  const props = inst.componentProperties;
  const cur = props[KBD_PROP] && props[KBD_PROP].value;
  if (typeof cur === "string" && cur.includes("Ctrl+Alt")) { inst.setProperties({ [KBD_PROP]: cur.replace(/Ctrl\+Alt/g, "Win+Alt") }); out.teclas++; }
}
for (const t of page.findAllWithCriteria({ types: ["TEXT"] })) {
  if (t.characters.includes("Ctrl+Alt")) {
    try { const segs = t.getStyledTextSegments(["fontName"]); for (const s of segs) await figma.loadFontAsync(s.fontName); t.characters = t.characters.replace(/Ctrl\+Alt/g, "Win+Alt"); out.textos++; }
    catch (e) { out.erros.push(`texto ${t.id}: ${e.message}`); }
  }
}

// 2) Malha no canvas de 04 (linhas finas a cada 16 px, atrás dos monitores)
const colecoes = await figma.variables.getLocalVariableCollectionsAsync();
async function varPorNome(colecao, nome) { const c = colecoes.find(c => c.name === colecao); for (const id of c.variableIds) { const v = await figma.variables.getVariableByIdAsync(id); if (v.name === nome) return v; } return null; }
for (const [nomeTela, colecao] of [["04 Layout de telas", "Cor claro"], ["04 Layout de telas · Escuro", "Cor escuro"]]) {
  const tela = page.children.find(c => c.name === nomeTela);
  if (!tela) continue;
  const canvas = tela.findOne(n => n.name === "Canvas de monitores");
  if (!canvas) { out.erros.push(`${nomeTela}: canvas não encontrado`); continue; }
  for (const old of canvas.findAll(n => n.name === "Malha")) old.remove();
  const borda = await varPorNome(colecao, "borda/padrão");
  const malha = figma.createFrame(); malha.name = "Malha"; malha.fills = []; malha.locked = true;
  malha.resize(canvas.width, canvas.height); malha.clipsContent = true;
  const base = figma.variables.setBoundVariableForPaint({ type: "SOLID", color: { r: 0, g: 0, b: 0 } }, "color", borda);
  const fill = { ...base, opacity: 0.8 };
  for (let x = 16; x < canvas.width; x += 16) { const l = figma.createRectangle(); l.resize(1, canvas.height); l.x = x; l.y = 0; l.fills = [fill]; l.name = "v"; malha.appendChild(l); }
  for (let y = 16; y < canvas.height; y += 16) { const l = figma.createRectangle(); l.resize(canvas.width, 1); l.x = 0; l.y = y; l.fills = [fill]; l.name = "h"; malha.appendChild(l); }
  canvas.insertChild(0, malha); malha.x = 0; malha.y = 0;
  malha.constraints = { horizontal: "STRETCH", vertical: "STRETCH" };
  out.malhas++;
}

// 3) e 4) Rótulos
for (const t of page.findAllWithCriteria({ types: ["TEXT"] })) {
  if (t.characters === "É controlada por esta máquina" && t.parent && "layoutMode" in t.parent && t.parent.layoutMode !== "NONE") {
    t.textAutoResize = "HEIGHT"; t.layoutSizingHorizontal = "FILL"; out.rotulos++;
  }
  if (t.characters === "esta máquina · 1 monitor") { t.characters = "esta máquina"; out.rotulos++; }
}
return out;
