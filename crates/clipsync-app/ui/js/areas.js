// Peças de página: cabeçalho, região assíncrona (carregando, vazio, erro) e estados.

import { h } from "./dom.js";
import { botao, esqueleto } from "./componentes.js";
import { mensagemDe } from "./formatar.js";

/** Cabeçalho da tela: h1 (alvo do foco ao navegar), descrição de uma linha e ações no topo. */
export function cabecalho({ titulo, descricao, acoes = [] }) {
  return h(
    "header",
    { class: "cabecalho" },
    h("div", { class: "cabecalho__texto" }, h("h1", { id: "titulo-pagina", tabIndex: -1 }, titulo), descricao && h("p", null, descricao)),
    acoes.length > 0 && h("div", { class: "cabecalho__acoes" }, acoes),
  );
}

export function estadoVazio({ titulo, texto, acao }) {
  return h("div", { class: "estado" }, h("h2", null, titulo), texto && h("p", { class: "muted" }, texto), acao);
}

/** Erro de uma busca. `erro` é o UiError ({ message }) ou um erro de JS. */
export function estadoErro(erro, aoTentar) {
  return h(
    "div",
    { class: "estado estado--erro", role: "alert" },
    h("h2", null, "Não foi possível carregar"),
    h("p", null, mensagemDe(erro)),
    aoTentar && botao({ rotulo: "Tentar de novo", aoClicar: aoTentar }),
  );
}

/**
 * Região que busca dados e pinta o resultado.
 * buscar() -> dados; pintar(dados) -> Node (ou estadoVazio). Mostra esqueleto enquanto espera
 * e estadoErro se falhar. recarregar({ silencioso: true }) mantém o conteúdo atual durante a busca.
 */
export function area({ buscar, pintar, linhas = 3 }) {
  const no = h("div", { class: "area" });
  let geracao = 0;

  async function recarregar({ silencioso = false } = {}) {
    const minha = ++geracao;
    if (!silencioso || no.childElementCount === 0) no.replaceChildren(esqueleto({ linhas }));
    try {
      const dados = await buscar();
      if (minha !== geracao) return;
      no.replaceChildren(pintar(dados, recarregar));
    } catch (erro) {
      if (minha !== geracao) return;
      no.replaceChildren(estadoErro(erro, () => recarregar()));
    }
  }

  recarregar();
  return { no, recarregar };
}
