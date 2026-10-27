import { api } from "./api.js";

const ROTAS = [
  {
    caminho: "onboarding/1",
    titulo: "Esta máquina",
    descricao: "O nome serve só para você reconhecer a máquina. A identidade é a chave criptográfica.",
    lateral: false,
  },
  {
    caminho: "onboarding/2",
    titulo: "O que sincronizar",
    descricao: "Escolha o que o ClipSync pode sincronizar. Nada é transmitido antes de você concluir.",
    lateral: false,
  },
  {
    caminho: "onboarding/3",
    titulo: "Teste",
    descricao: "Rode um teste com conteúdo sintético para ver o que funciona nesta máquina.",
    lateral: false,
  },
  {
    caminho: "dispositivos",
    titulo: "Dispositivos",
    descricao: "Quem pode enviar, quem pode receber e quem pode controlar esta máquina.",
    lateral: true,
    nav: "dispositivos",
  },
  {
    caminho: "pareamento",
    titulo: "Pareamento",
    descricao: "Confirme o mesmo código nas duas máquinas. O código vem da conexão, não da rede.",
    lateral: true,
    nav: "dispositivos",
  },
  {
    caminho: "layout",
    titulo: "Layout de telas",
    descricao: "Ligue um trecho de borda de um monitor à borda de um monitor de outra máquina.",
    lateral: true,
    nav: "layout",
  },
  {
    caminho: "clipboard",
    titulo: "Clipboard",
    descricao: "Texto copiado aqui vai aos pares autorizados. O que chega fica na caixa até você decidir.",
    lateral: true,
    nav: "clipboard",
  },
  {
    caminho: "diagnostico",
    titulo: "Diagnóstico",
    descricao: "O que o sistema oferece, o que o ClipSync está usando e por que algo foi bloqueado.",
    lateral: true,
    nav: "diagnostico",
  },
  {
    caminho: "preferencias",
    titulo: "Preferências",
    descricao: "Atalhos, inicialização, notificações e aparência.",
    lateral: true,
    nav: "preferencias",
  },
];

const ROTA_INICIAL = "dispositivos";

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

let estadoGeral = null;
let primeiraRota = true;

function criarCabecalho(rota) {
  const cabecalho = document.createElement("header");
  cabecalho.className = "cabecalho";

  const texto = document.createElement("div");
  texto.className = "cabecalho__texto";

  const titulo = document.createElement("h1");
  titulo.id = "titulo-pagina";
  titulo.tabIndex = -1;
  titulo.textContent = rota.titulo;

  const descricao = document.createElement("p");
  descricao.textContent = rota.descricao;

  texto.append(titulo, descricao);
  cabecalho.append(texto);
  return cabecalho;
}

function renderizarRota(rota) {
  const pagina = document.createElement("div");
  pagina.className = "pagina";
  pagina.append(criarCabecalho(rota));
  el.conteudo.replaceChildren(pagina);
}

function marcarMenu(rota) {
  for (const link of el.menu) {
    if (link.dataset.nav === rota.nav) link.setAttribute("aria-current", "page");
    else link.removeAttribute("aria-current");
  }
}

function rotaDoHash() {
  const caminho = location.hash.replace(/^#\/?/, "");
  return ROTAS.find((r) => r.caminho === caminho) ?? null;
}

function navegar() {
  const rota = rotaDoHash();
  if (!rota) {
    location.replace(`#/${ROTA_INICIAL}`);
    return;
  }
  el.app.dataset.lateral = rota.lateral ? "visivel" : "oculta";
  document.title = `${rota.titulo} · ClipSync`;
  marcarMenu(rota);
  renderizarRota(rota);
  el.conteudo.scrollTop = 0;
  if (!primeiraRota) document.getElementById("titulo-pagina").focus();
  primeiraRota = false;
}

function pintarRodape() {
  if (!estadoGeral) return;
  const { maquina, malha, pausado } = estadoGeral;
  el.maquinaNome.textContent = maquina.nome;
  el.maquinaImpressao.textContent = maquina.impressao;

  el.malha.className = "selo " + (pausado ? "selo--aviso" : "selo--ok");
  el.malha.textContent = pausado
    ? "Pausada"
    : `${malha.conectados} de ${malha.total} conectados`;

  el.pausar.disabled = false;
  el.pausar.setAttribute("aria-pressed", String(pausado));
  el.pausarRotulo.textContent = pausado ? "Retomar tudo" : "Pausar tudo";
}

async function alternarPausa() {
  const alvo = !estadoGeral.pausado;
  el.pausar.disabled = true;
  el.pausar.setAttribute("aria-busy", "true");
  try {
    estadoGeral = await api.pausarTudo(alvo);
    pintarRodape();
  } catch (erro) {
    console.error("falha ao alternar a pausa", erro);
  } finally {
    el.pausar.disabled = false;
    el.pausar.removeAttribute("aria-busy");
  }
}

async function iniciar() {
  el.pausar.addEventListener("click", alternarPausa);
  document.querySelector(".pular").addEventListener("click", (evento) => {
    evento.preventDefault();
    el.conteudo.focus();
  });
  window.addEventListener("hashchange", navegar);
  navegar();
  try {
    estadoGeral = await api.estadoGeral();
    pintarRodape();
  } catch (erro) {
    console.error("falha ao ler o estado geral", erro);
  }
}

iniciar();
