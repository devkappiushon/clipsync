// Peças de página: cabeçalho, região assíncrona (carregando, vazio, erro), avisos e estados.

import { criarAnunciador, guardarFoco, restaurarFoco } from "./acessibilidade.js";
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
export function estadoErro(erro, aoTentar, { anunciar = true } = {}) {
  return h(
    "div",
    { class: "estado estado--erro", role: anunciar ? "alert" : null },
    h("h2", { tabIndex: -1 }, "Não foi possível carregar"),
    h("p", null, mensagemDe(erro)),
    aoTentar && botao({ rotulo: "Tentar de novo", icone: "atualizar", dataset: { focusKey: "area-tentar" }, aoClicar: aoTentar }),
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
export function area({ buscar, pintar, linhas = 3, rotulo = "Conteúdo", fallback }) {
  const anuncios = criarAnunciador();
  const conteudo = h("div", { class: "area__conteudo", tabIndex: -1, role: "region", "aria-label": rotulo });
  const no = h("div", { class: "area" }, anuncios.no, conteudo);
  let geracao = 0;
  let desmontada = false;
  let anuncioPendente = false;
  const barreira = coordenarAtualizacao((opcoes) => recarregar(opcoes));

  async function recarregar({ silencioso = false, focoFallback } = {}) {
    if (desmontada) return;
    // A intenção explícita acompanha o resultado mais novo, inclusive quando agrupada com polling.
    if (!silencioso) anuncioPendente = true;
    const opcoes = { silencioso, ...(focoFallback ? { focoFallback } : {}) };
    if (barreira.adiar(opcoes)) return;
    const minha = ++geracao;
    // A ação de tentar novamente fica montada enquanto a resposta está pendente.
    if (conteudo.childElementCount === 0) {
      const espera = esqueleto({ linhas });
      espera.removeAttribute("role");
      espera.setAttribute("aria-hidden", "true");
      trocar(conteudo, espera);
    }
    if (anuncioPendente) {
      conteudo.setAttribute("aria-busy", "true");
      anuncios.status("Carregando.");
    }
    try {
      const dados = await buscar();
      if (minha !== geracao || desmontada || barreira.adiar(opcoes)) return;
      const foco = guardarFoco(conteudo);
      trocar(conteudo, pintar(dados, recarregar));
      restaurarFoco(foco, no.closest(".pagina") ?? no, { fallback: focoFallback ?? fallback ?? (() => conteudo) });
      if (anuncioPendente) anuncios.status("Conteúdo atualizado.");
      anuncioPendente = false;
    } catch (erro) {
      if (minha !== geracao || desmontada || barreira.adiar(opcoes)) return;
      // Um polling falho não remove formulário/valores que o usuário está usando.
      if (!anuncioPendente && silencioso && conteudo.childElementCount > 0) return;
      const foco = guardarFoco(conteudo);
      trocar(conteudo, estadoErro(erro, () => recarregar(), { anunciar: false }));
      restaurarFoco(foco, conteudo, { fallback: () => conteudo.querySelector("button") });
      anuncios.erro(mensagemDe(erro));
      anuncioPendente = false;
    } finally {
      if (minha === geracao && !anuncioPendente) conteudo.removeAttribute("aria-busy");
    }
  }

  // O canal vazio entra no documento antes do primeiro anúncio.
  const inicio = setTimeout(() => { if (!desmontada) recarregar(); }, 0);
  return { no, recarregar, proteger: barreira.proteger,
    desmontar: () => {
      desmontada = true; anuncioPendente = false; geracao++;
      clearTimeout(inicio); barreira.cancelar(); conteudo.removeAttribute("aria-busy");
    } };
}

/** Agrupa atualizações recebidas durante ações para não recriar um controle ainda ocupado. */
export function coordenarAtualizacao(atualizar) {
  let operacoes = 0;
  let pendente = null;
  let encerrada = false;
  return {
    adiar(opcoes) {
      if (operacoes === 0) return false;
      pendente = { ...pendente, ...opcoes };
      return true;
    },
    async proteger(tarefa) {
      if (encerrada) return;
      operacoes++;
      try { return await tarefa(); }
      finally {
        operacoes--;
        if (operacoes === 0 && pendente && !encerrada) {
          const opcoes = pendente;
          pendente = null;
          await atualizar(opcoes);
        }
      }
    },
    cancelar() { encerrada = true; pendente = null; },
  };
}
