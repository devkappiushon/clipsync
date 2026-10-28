// Peças de página: cabeçalho, região assíncrona (carregando, vazio, erro), avisos e estados.

import { h, trocar } from "./dom.js";
import { botao, esqueleto } from "./componentes.js";
import { INDISPONIVEL, mensagemDe } from "./formatar.js";

/** Cabeçalho da tela: h1 (alvo do foco ao navegar), descrição de uma linha e ações no topo. */
export function cabecalho({ titulo, descricao, acoes = [] }) {
  return h(
    "header",
    { class: "cabecalho" },
    h("div", { class: "cabecalho__texto" }, h("h1", { id: "titulo-pagina", tabIndex: -1 }, titulo), descricao && h("p", null, descricao)),
    acoes.length > 0 && h("div", { class: "cabecalho__acoes" }, acoes),
  );
}

/** `texto` pode ser uma frase ou nós prontos (lista de passos, por exemplo). */
export function estadoVazio({ titulo, texto, acao }) {
  const corpo = typeof texto === "string" ? h("p", { class: "muted" }, texto) : texto;
  return h("div", { class: "estado" }, h("h2", null, titulo), corpo, acao);
}

/** Erro de uma busca. `erro` é o UiError ({ message }) ou um erro de JS. */
export function estadoErro(erro, aoTentar) {
  return h(
    "div",
    { class: "estado estado--erro", role: "alert" },
    h("h2", null, "Não foi possível carregar"),
    h("p", null, mensagemDe(erro)),
    aoTentar && botao({ rotulo: "Tentar de novo", icone: "atualizar", aoClicar: aoTentar }),
  );
}

/** Mensagem em linha. tom: info | aviso | erro. Erro é anunciado na hora pelo leitor de tela. */
export function aviso(tom, ...filhos) {
  return h("div", { class: `aviso aviso--${tom}`, role: tom === "erro" ? "alert" : "status" }, filhos);
}

/** Texto padrão de recurso sem backend nesta versão (contrato §12.2 regra 6). */
export function indisponivel(texto = INDISPONIVEL) {
  return h("span", { class: "nota" }, texto);
}

/**
 * Região que busca dados e pinta o resultado.
 * buscar() -> dados; pintar(dados, recarregar) -> Node (ou estadoVazio). Mostra esqueleto enquanto espera
 * e estadoErro se falhar. recarregar({ silencioso: true }) mantém o conteúdo atual durante a busca.
 */
export function area({ buscar, pintar, linhas = 3 }) {
  const no = h("div", { class: "area" });
  let geracao = 0;

  async function recarregar({ silencioso = false } = {}) {
    const minha = ++geracao;
    if (!silencioso || no.childElementCount === 0) trocar(no, esqueleto({ linhas }));
    try {
      const dados = await buscar();
      if (minha !== geracao) return;
      trocar(no, pintar(dados, recarregar));
    } catch (erro) {
      if (minha !== geracao) return;
      trocar(no, estadoErro(erro, () => recarregar()));
    }
  }

  recarregar();
  return { no, recarregar };
}
