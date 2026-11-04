import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import vm from "node:vm";

const source = await readFile(new URL("../../ui/js/telas/diagnostico.js", import.meta.url), "utf8");
// Captura os argumentos dos construtores da tela real. Layout e DOM são validados no portal.
function render(dados, app = null) {
  let paint;
  const h = (tag, attrs, ...children) => ({ tag, attrs: attrs ?? {}, children: children.flat(Infinity).filter((v) => v != null && v !== false) });
  const context = {
    h, area: (options) => { paint = options.pintar; return { no: h("div"), desmontar() {} }; },
    cartao: ({ titulo, filhos }) => h("section", null, h("h2", null, titulo), filhos),
    cabecalho: () => h("header"), botao: () => h("button"), criarAnunciador: () => ({ no: h("div") }),
    estadoVazio: () => h("p"), indisponivel: () => "indisponível", recurso: () => false,
    obterApp: () => app, api: {},
  };
  vm.createContext(context);
  vm.runInContext(source.replace(/^import .+;\n/gm, "").replace("export default function montar", "function montar") + "\nthis.montar = montar;", context);
  context.montar({ assinar: () => () => {} });
  return paint({ items: [], network: { peers: [], addresses: [] }, blocks: [], ...dados });
}
function nodes(node) {
  return typeof node === "object" && node ? [node, ...node.children.flatMap(nodes)] : [];
}
function pathNode(tree) {
  return nodes(tree).find((node) => node.tag === "dl" && node.children.some((child) => child.tag === "dt" && child.children.includes("Arquivo de log")))?.children.find((child) => child.tag === "dd");
}

test("Diagnóstico mostra o caminho do payload como texto em dd mono", () => {
  const path = "/perfil/<img src=x>/logs/clipsync.jsonl";
  const tree = render({ log_path: path }, { log_path: "/outro/log" });
  const dd = pathNode(tree);
  assert.ok(dd, "caminho ausente na tela");
  assert.match(dd.attrs.class, /\bmono\b/);
  assert.deepEqual(Array.from(dd.children), [path]);
  assert.equal(nodes(tree).some((node) => node.tag === "img" || Object.hasOwn(node.attrs, "style")), false);
});

test("payload antigo usa o caminho do app_state, inclusive com default vazio", () => {
  for (const payload of [{}, { log_path: "" }]) {
    const dd = pathNode(render(payload, { log_path: "C:\\perfil\\logs\\clipsync.jsonl" }));
    assert.ok(dd);
    assert.deepEqual(Array.from(dd.children), ["C:\\perfil\\logs\\clipsync.jsonl"]);
  }
});

test("sem caminho nas duas respostas exibe ausência sem inventar arquivo", () => {
  const dd = pathNode(render({}));
  assert.ok(dd);
  assert.deepEqual(Array.from(dd.children), ["Caminho não informado pelo agente."]);
});

for (const platform of ["linux", "windows"]) {
  test(`simulador ${platform} usa o mesmo caminho no diagnóstico e no app_state`, () => {
    const mock = new URL("../mock.js", import.meta.url).href;
    execFileSync(process.execPath, ["--max-old-space-size=512", "--input-type=module", "-e", `
      import assert from "node:assert/strict";
      globalThis.location = { search: "?cenario=padrao&plataforma=${platform}" };
      const {invoke} = await import(${JSON.stringify(mock)});
      const call = (cmd) => invoke("clipsync", {req:{cmd}});
      const app = await call("app_state");
      const diag = await call("diagnostics_get");
      const separator = ${JSON.stringify(platform === "windows" ? "\\" : "/")};
      assert.equal(diag.log_path, app.profile_dir + separator + "logs" + separator + "clipsync.jsonl");
      assert.equal(app.log_path, diag.log_path);
    `], { stdio: "pipe" });
  });
}
