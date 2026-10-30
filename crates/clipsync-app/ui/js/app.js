// Casca da janela: rotas por hash, barra lateral e eventos globais.
// Cada tela é um módulo em js/telas/<nome>.js com `export default function montar(ctx)`,
// que devolve { no, desmontar? } (ou uma promessa disso).

import { criarAnunciador, guardarFoco, restaurarFoco, interacaoAtual } from "./acessibilidade.js";
import { api } from "./api.js";
import { estadoErro } from "./areas.js";
import { botao, comCarregamento } from "./componentes.js";
import { h, trocar } from "./dom.js";
import { aplicarAparencia, assinar, atualizarApp, iniciarEventos, obterApp, observarApp, pausaObservada } from "./estado.js";
import { mensagemDe } from "./formatar.js";
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
  tela: document.getElementById("tela"),
  avisoAgente: document.getElementById("aviso-agente"),
  menu: document.querySelectorAll("#menu a"),
  versao: document.getElementById("app-versao"),
  maquinaPonto: document.getElementById("maquina-ponto"),
  maquinaNome: document.getElementById("maquina-nome"),
  malha: document.getElementById("malha"),
  pausar: document.getElementById("pausar"),
  pausarRotulo: document.getElementById("pausar-rotulo"),
};

const anuncios = criarAnunciador();
const avisoOperacao = h("div", { class: "aviso-agente", hidden: true });
el.conteudo.prepend(anuncios.no, avisoOperacao);
function erroOperacao(erro) {
  const texto = mensagemDe(erro);
  avisoOperacao.hidden = false;
  trocar(avisoOperacao, h("div", { class: "aviso aviso--erro" }, h("p", null, texto)));
  anuncios.erro(texto);
}

let desmontarAtual = null;
let geracao = 0;
let primeiraRota = true;
let falhaConsultaPausa = null;

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
  const focoNavegacao = guardarFoco(document.body);
  desmontarAtual?.();
  desmontarAtual = null;

  el.app.dataset.lateral = rota.lateral ? "visivel" : "oculta";
  document.title = `${rota.titulo} · ClipSync`;
  marcarMenu(rota);

  const pagina = h("div", { class: "pagina" });
  trocar(el.tela, pagina);
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

  if (!primeiraRota && (!focoNavegacao || interacaoAtual(focoNavegacao, document.body))) document.getElementById("titulo-pagina")?.focus();
  primeiraRota = false;
}

function pintarRodape(app) {
  el.versao.textContent = app.version ? `v${app.version}` : "";
  el.maquinaNome.textContent = app.device?.name ?? "Sem perfil";
  if (app.device?.short) el.maquinaNome.title = `Impressão digital ${app.device.short}`;
  else el.maquinaNome.removeAttribute("title");

  let estado;
  let texto;
  if (app.agent === "failed") [estado, texto] = ["erro", "Malha: agente com falha"];
  else if (app.agent === "stopped") [estado, texto] = ["neutro", "Malha: agente parado"];
  else if (falhaConsultaPausa || pausaObservada(app) === null) [estado, texto] = ["aviso", "Malha: estado não confirmado"];
  else if (pausaObservada(app)) [estado, texto] = ["aviso", "Malha: pausada"];
  else [estado, texto] = ["ok", `Malha: ${app.mesh.connected} de ${app.mesh.total} conectados`];
  el.maquinaPonto.dataset.estado = app.device ? estado : "neutro";
  if (el.malha.textContent !== texto) el.malha.textContent = texto;

  const pausa = falhaConsultaPausa ? null : pausaObservada(app);
  el.pausar.disabled = pausa === null;
  el.pausar.querySelector(".icone")?.remove();
  el.pausar.prepend(icone(pausa === true ? "retomar" : "pausa"));
  el.pausarRotulo.textContent = pausa === null && !["stopped", "failed"].includes(app.agent) ? "Pausa não confirmada" : pausa ? "Retomar tudo" : "Pausar tudo";
}

/** Faixa persistente: polling atualiza texto, sem trocar o botão que está em uso. */
const textoAgente = h("p");
const iniciarAgente = botao({
  rotulo: "Iniciar agente", variante: "primario", icone: "retomar", dataset: { focusKey: "agente-iniciar" },
  aoClicar: async () => {
    const foco = guardarFoco(document.body);
    avisoOperacao.hidden = true;
    anuncios.limpar();
    try {
      await api.agentStart();
      await atualizarApp();
      restaurarFoco(foco, document.body, { fallback: () => document.getElementById("titulo-pagina") });
    } catch (erro) { erroOperacao(erro); }
  },
});
const faixaAgente = h("div", { class: "aviso aviso--aviso" }, textoAgente, iniciarAgente);
el.avisoAgente.append(faixaAgente);

function pintarAvisoAgente(app) {
  const mostrar = app.initialized && (app.agent === "stopped" || app.agent === "failed");
  el.avisoAgente.hidden = !mostrar;
  if (!mostrar) return;
  const texto = app.agent === "failed"
    ? `O agente parou com erro: ${app.agent_error ?? "causa desconhecida"}`
    : "O agente está parado. Nada é enviado nem recebido até você iniciá-lo.";
  faixaAgente.className = `aviso aviso--${app.agent === "failed" ? "erro" : "aviso"}`;
  if (textoAgente.textContent !== texto) {
    textoAgente.textContent = texto;
    if (app.agent === "failed") anuncios.erro(texto);
    else anuncios.status(texto);
  }
}

const textoPausa = h("p");
const avisoPausa = h("div", { class: "aviso aviso--aviso", hidden: true }, textoPausa);
el.conteudo.insertBefore(avisoPausa, avisoOperacao);
let ultimoAvisoPausa = null;
function pintarAvisoPausa(app) {
  const avisos = [...new Set([falhaConsultaPausa, app.pause_warning].filter(Boolean))];
  const texto = avisos.join(" ") || (app.agent === "in_app" && app.agent_error ? app.agent_error : null);
  avisoPausa.hidden = !texto;
  if (texto !== ultimoAvisoPausa) {
    textoPausa.textContent = texto ?? "";
    if (texto) anuncios.erro(texto);
    else anuncios.limpar();
    ultimoAvisoPausa = texto;
  }
}

function alternarPausa() {
  return comCarregamento(el.pausar, async () => {
    const pausa = pausaObservada(obterApp());
    if (pausa === null) return;
    avisoOperacao.hidden = true;
    try {
      await api.pauseSet({ paused: !pausa });
      await atualizarApp();
    } catch (erro) {
      // Uma falha pode ter alterado memória sem persistir. Nunca pintar o valor pedido.
      try { await atualizarApp(); }
      catch {
        falhaConsultaPausa = `Não foi possível confirmar ${pausa ? "a retomada" : "a pausa"}, nem consultar o estado atual. Aguarde uma nova consulta antes de repetir a ação.`;
        pintarAvisoPausa(obterApp());
        pintarRodape(obterApp());
      }
      if (!obterApp()?.pause_warning && !falhaConsultaPausa) erroOperacao(erro);
    }
  });
}

async function iniciar() {
  el.pausar.addEventListener("click", alternarPausa);
  document.querySelector(".pular").addEventListener("click", (evento) => {
    evento.preventDefault();
    el.conteudo.focus();
  });
  observarApp((app, consultado) => {
    if (consultado) falhaConsultaPausa = null;
    pintarAvisoPausa(app);
  });
  observarApp(pintarRodape);
  observarApp(pintarAvisoAgente);
  window.addEventListener("hashchange", navegar);

  try {
    await atualizarApp();
    // A aparência vem do ui.toml e vale em todas as rotas, onboarding incluso.
    // Se falhar, a janela abre no tema do sistema e segue.
    try {
      aplicarAparencia((await api.prefsGet()).appearance);
    } catch (erro) {
      console.warn("sem preferências de aparência", erro);
    }
    await iniciarEventos();
  } catch (erro) {
    console.error("falha ao ler o estado inicial", erro);
    el.maquinaPonto.dataset.estado = "erro";
    el.malha.textContent = "Malha: sem resposta do agente";
  }
  navegar();
}

iniciar();
