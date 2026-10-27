// Casca da janela: rotas por hash, barra lateral e eventos globais.
// Cada tela é um módulo em js/telas/<nome>.js com `export default function montar(ctx)`,
// que devolve { no, desmontar? } (ou uma promessa disso).

import { api } from "./api.js";
import { estadoErro } from "./areas.js";
import { comCarregamento, selo } from "./componentes.js";
import { h } from "./dom.js";
import { aplicarAparencia, assinar, atualizarApp, iniciarEventos, obterApp, observarApp } from "./estado.js";
import { icone } from "./icones.js";

const ROTAS = [
  { caminho: "onboarding/1", tela: "onboarding", passo: 1, titulo: "Esta máquina", lateral: false },
  { caminho: "onboarding/2", tela: "onboarding", passo: 2, titulo: "O que sincronizar", lateral: false },
  { caminho: "onboarding/3", tela: "onboarding", passo: 3, titulo: "Teste", lateral: false },
  { caminho: "dispositivos", tela: "dispositivos", titulo: "Dispositivos", lateral: true, nav: "dispositivos" },
  { caminho: "pareamento", tela: "pareamento", titulo: "Pareamento", lateral: true, nav: "dispositivos" },
  { caminho: "layout", tela: "layout", titulo: "Layout de telas", lateral: true, nav: "layout" },
  { caminho: "clipboard", tela: "clipboard", titulo: "Clipboard", lateral: true, nav: "clipboard" },
  { caminho: "diagnostico", tela: "diagnostico", titulo: "Diagnóstico", lateral: true, nav: "diagnostico" },
  { caminho: "preferencias", tela: "preferencias", titulo: "Preferências", lateral: true, nav: "preferencias" },
];

const ROTA_INICIAL = "dispositivos";
const ROTA_PRIMEIRO_USO = "onboarding/1";

const el = {
  app: document.getElementById("app"),
  conteudo: document.getElementById("conteudo"),
  menu: document.querySelectorAll("#menu a"),
  maquinaNome: document.getElementById("maquina-nome"),
  maquinaImpressao: document.getElementById("maquina-impressao"),
  malha: document.getElementById("malha"),
  pausar: document.getElementById("pausar"),
  pausarRotulo: document.getElementById("pausar-rotulo"),
};

let desmontarAtual = null;
let geracao = 0;
let primeiraRota = true;

const ir = (caminho) => {
  location.hash = `#/${caminho}`;
};

function rotaDoHash() {
  const caminho = location.hash.replace(/^#\/?/, "");
  return ROTAS.find((r) => r.caminho === caminho) ?? null;
}

function marcarMenu(rota) {
  for (const link of el.menu) {
    if (link.dataset.nav === rota.nav) link.setAttribute("aria-current", "page");
    else link.removeAttribute("aria-current");
  }
}

async function navegar() {
  const rota = rotaDoHash();
  if (!rota) {
    location.replace(`#/${ROTA_INICIAL}`);
    return;
  }
  const app = obterApp();
  if (app && !app.initialized && rota.lateral) {
    location.replace(`#/${ROTA_PRIMEIRO_USO}`);
    return;
  }

  const minha = ++geracao;
  desmontarAtual?.();
  desmontarAtual = null;

  el.app.dataset.lateral = rota.lateral ? "visivel" : "oculta";
  document.title = `${rota.titulo} · ClipSync`;
  marcarMenu(rota);

  const pagina = h("div", { class: "pagina" });
  el.conteudo.replaceChildren(pagina);
  el.conteudo.scrollTop = 0;

  try {
    const modulo = await import(`./telas/${rota.tela}.js`);
    const tela = await modulo.default({ rota, ir, assinar });
    if (minha !== geracao) {
      tela.desmontar?.();
      return;
    }
    desmontarAtual = tela.desmontar ?? null;
    pagina.append(tela.no);
  } catch (erro) {
    if (minha !== geracao) return;
    console.error(`falha ao montar a tela ${rota.tela}`, erro);
    pagina.append(estadoErro(erro, navegar));
  }

  if (!primeiraRota) document.getElementById("titulo-pagina")?.focus();
  primeiraRota = false;
}

function pintarRodape(app) {
  el.maquinaNome.textContent = app.device?.name ?? "Sem perfil";
  el.maquinaImpressao.textContent = app.device?.short ?? "";

  let estado;
  if (app.agent === "failed") estado = selo("erro", "Agente com falha");
  else if (app.agent === "stopped") estado = selo("neutro", "Agente parado");
  else if (app.paused) estado = selo("aviso", "Pausada");
  else estado = selo("ok", `${app.mesh.connected} de ${app.mesh.total} conectados`);
  el.malha.replaceChildren(estado);

  el.pausar.disabled = app.agent === "stopped" || app.agent === "failed";
  el.pausar.setAttribute("aria-pressed", String(app.paused));
  el.pausar.querySelector(".icone")?.remove();
  el.pausar.prepend(icone(app.paused ? "retomar" : "pausa"));
  el.pausarRotulo.textContent = app.paused ? "Retomar tudo" : "Pausar tudo";
}

function alternarPausa() {
  return comCarregamento(el.pausar, async () => {
    await api.pauseSet({ paused: !obterApp().paused });
    await atualizarApp();
  });
}

async function iniciar() {
  el.pausar.addEventListener("click", alternarPausa);
  document.querySelector(".pular").addEventListener("click", (evento) => {
    evento.preventDefault();
    el.conteudo.focus();
  });
  observarApp(pintarRodape);
  window.addEventListener("hashchange", navegar);

  try {
    const app = await atualizarApp();
    if (app.initialized) aplicarAparencia((await api.prefsGet()).appearance);
    await iniciarEventos();
  } catch (erro) {
    console.error("falha ao ler o estado inicial", erro);
    el.malha.replaceChildren(selo("erro", "Sem resposta do agente"));
  }
  navegar();
}

iniciar();
