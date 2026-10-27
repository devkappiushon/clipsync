// Gera a versão escura das 9 telas no arquivo do Figma "ClipSync · telas"
// (https://www.figma.com/design/vkXY3Q37AaGe1argTA2STv).
// Rode com o MCP do Figma (use_figma) ou com o plugin Scripter dentro do arquivo.
// Clona cada quadro claro para y=900, renomeia com "· Escuro" e religa cada cor
// da coleção "Cor claro" para a variável de mesmo nome em "Cor escuro".
const L2D = {"VariableID:2:4":"VariableID:2:5","VariableID:2:6":"VariableID:2:7","VariableID:2:8":"VariableID:2:9","VariableID:2:10":"VariableID:2:11","VariableID:2:12":"VariableID:2:13","VariableID:2:14":"VariableID:2:15","VariableID:2:16":"VariableID:2:17","VariableID:2:18":"VariableID:2:19","VariableID:2:20":"VariableID:2:21","VariableID:2:22":"VariableID:2:23","VariableID:2:24":"VariableID:2:25","VariableID:2:26":"VariableID:2:27","VariableID:2:28":"VariableID:2:29","VariableID:2:30":"VariableID:2:31","VariableID:2:32":"VariableID:2:33","VariableID:2:34":"VariableID:2:35"};
const dark = {};
await Promise.all(Object.values(L2D).map(async id => { dark[id] = await figma.variables.getVariableByIdAsync(id); }));
const fonts = [["Inter","Regular"],["Inter","Medium"],["Inter","Semi Bold"],["Geist","SemiBold"],["Geist","Medium"],["Geist Mono","Regular"],["Geist Mono","Medium"]];
await Promise.all(fonts.map(([family, style]) => figma.loadFontAsync({ family, style })));
const names = ["01a Onboarding · Esta máquina","01b Onboarding · O que sincronizar","01c Onboarding · Teste","02 Dispositivos","03 Pareamento","04 Layout de telas","05 Clipboard","06 Diagnóstico","07 Preferências"];
const page = figma.currentPage;
for (const c of [...page.children]) if (c.name.endsWith(" · Escuro")) c.remove();
const out = [];
for (const name of names) {
  const src = page.children.find(c => c.name === name);
  if (!src) { out.push({ name, error: "não encontrado" }); continue; }
  const clone = src.clone();
  clone.name = `${name} · Escuro`;
  clone.x = src.x; clone.y = 900;
  let rebound = 0;
  const nodes = [clone, ...clone.findAll(() => true)];
  for (const n of nodes) {
    for (const prop of ["fills", "strokes"]) {
      if (!(prop in n)) continue;
      const arr = n[prop];
      if (!Array.isArray(arr) || !arr.length) continue;
      let changed = false;
      const next = arr.map(p => { const id = p.boundVariables && p.boundVariables.color && p.boundVariables.color.id; if (id && L2D[id]) { changed = true; return figma.variables.setBoundVariableForPaint(p, "color", dark[L2D[id]]); } return p; });
      if (changed) { n[prop] = next; rebound++; }
    }
  }
  out.push({ name: clone.name, id: clone.id, nodes: nodes.length, rebound });
}
return out;
