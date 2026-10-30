// Componentes da interface. Toda ação assíncrona passa por comCarregamento:
// o botão permanece focável, impede reentrada e mostra o indicador até concluir.

import { ocupar, criarEscolhaAssincrona, guardarFoco, restaurarFoco, criarAnunciador } from "./acessibilidade.js";
import { h, novoId } from "./dom.js";
import { icone } from "./icones.js";

export async function comCarregamento(botao, tarefa) {
  const liberar = ocupar(botao);
  if (!liberar) return undefined;
  try {
    return await tarefa();
  } catch (erro) {
    console.error("falha em ação assíncrona", erro);
    return undefined;
  } finally {
    liberar();
  }
}

/** variante: primario | secundario | perigo | discreto. */
export function botao({ rotulo, variante = "secundario", icone: nomeIcone, atalho, tipo = "button", desabilitado = false, aoClicar, ...resto }) {
  const no = h(
    "button",
    { class: `btn btn--${variante}`, type: tipo, disabled: desabilitado, ...resto },
    nomeIcone && icone(nomeIcone),
    h("span", { class: "btn__rotulo" }, rotulo),
    atalho && tecla(atalho),
  );
  if (aoClicar) no.addEventListener("click", (evento) => comCarregamento(no, () => aoClicar(evento)));
  return no;
}

export function tecla(texto) {
  return h("kbd", null, texto);
}

/** estado: ok | aviso | erro | neutro | acento. O ícone muda de forma, não só de cor. */
export function selo(estado, texto) {
  return h("span", { class: `selo selo--${estado}` }, icone(`estado-${estado}`), texto);
}

export function cartao({ titulo, acoes, plano = false, filhos = [] }) {
  const cabecalho =
    titulo || acoes
      ? h("div", { class: "cartao__cabecalho" }, titulo && h("h2", null, titulo), acoes && h("div", { class: "linha" }, acoes))
      : null;
  return h("section", { class: `cartao${plano ? " cartao--plano" : ""}` }, cabecalho, filhos);
}

/** Interruptor com rótulo e descrição. aoMudar(marcado) pode ser assíncrono; se lançar, o interruptor volta ao valor anterior (quem chamou mostra o erro). */
export function interruptor({ rotulo, descricao, marcado = false, desabilitado = false, aoMudar, chaveFoco }) {
  const id = novoId("interruptor");
  const idDescricao = `${id}-d`;
  const entrada = h("input", {
    class: "interruptor",
    type: "checkbox",
    role: "switch",
    dataset: chaveFoco ? { focusKey: chaveFoco } : undefined,
    id,
    checked: marcado,
    disabled: desabilitado,
    "aria-describedby": descricao ? idDescricao : null,
  });
  const escolher = criarEscolhaAssincrona({
    valor: marcado,
    aplicar: (valor) => { entrada.checked = valor; },
    disponivel: () => !entrada.disabled,
    aoMudar,
    aoOcupar: (ocupado) => {
      entrada.toggleAttribute("aria-busy", ocupado);
      if (ocupado) {
        entrada.setAttribute("aria-busy", "true");
        entrada.setAttribute("aria-disabled", "true");
      } else entrada.removeAttribute("aria-disabled");
    },
  });
  entrada.addEventListener("click", (evento) => {
    if (entrada.getAttribute("aria-busy") === "true") evento.preventDefault();
  });
  entrada.addEventListener("change", () => { escolher(entrada.checked).catch(() => {}); });

  return h(
    "div",
    { class: "opcao" },
    h("div", { class: "opcao__texto" }, h("label", { class: "opcao__rotulo", for: id }, rotulo), descricao && h("p", { class: "opcao__descricao", id: idDescricao }, descricao)),
    entrada,
  );
}

/** tipo: texto | mono | select. Para select, passe opcoes: [{ valor, rotulo }]. */
export function campo({ rotulo, valor = "", tipo = "texto", opcoes = [], ajuda, erro, placeholder, aoMudar, ...resto }) {
  const id = novoId("campo");
  const idAjuda = `${id}-a`;
  const idErro = `${id}-e`;
  const descritores = [ajuda && idAjuda, erro && idErro].filter(Boolean).join(" ");
  const comuns = { id, class: `entrada${tipo === "mono" ? " entrada--mono" : ""}`, "aria-describedby": descritores || null, "aria-invalid": erro ? "true" : null };

  let controle;
  if (tipo === "select") {
    controle = h("select", { ...comuns, ...resto }, opcoes.map((o) => h("option", { value: o.valor, selected: o.valor === valor }, o.rotulo)));
  } else {
    controle = h("input", { ...comuns, type: "text", value: valor, placeholder, autocomplete: "off", spellcheck: "false", ...resto });
  }
  if (aoMudar) controle.addEventListener(tipo === "select" ? "change" : "input", () => aoMudar(controle.value));

  const no = h(
    "div",
    { class: "campo" },
    h("label", { class: "campo__rotulo", for: id }, rotulo),
    controle,
    ajuda && h("p", { class: "campo__ajuda", id: idAjuda }, ajuda),
    erro && h("p", { class: "campo__erro", id: idErro }, erro),
  );
  no.controle = controle;
  return no;
}

/** Seletor segmentado: grupo de opções exclusivas, navegável por setas. Opção com `desabilitada: true` fica visível e inerte. */
export function segmentado({ rotulo, opcoes, valor, aoMudar, chaveFoco }) {
  const nome = novoId("segmentado");
  const entradas = opcoes.map((o) => h("input", {
    type: "radio", name: nome, value: o.valor, checked: o.valor === valor, disabled: o.desabilitada === true,
    dataset: chaveFoco ? { focusKey: `${chaveFoco}:${o.valor}` } : undefined,
  }));
  const grupo = h("div", { class: "segmentado", role: "radiogroup", "aria-label": rotulo },
    opcoes.map((o, i) => h("label", { class: "segmentado__opcao" }, entradas[i], h("span", null, o.rotulo))));
  const escolher = criarEscolhaAssincrona({
    valor, aoMudar,
    disponivel: (proximo) => entradas.some((entrada) => entrada.value === proximo && !entrada.disabled),
    aplicar: (atual) => entradas.forEach((entrada) => { entrada.checked = entrada.value === atual; }),
    aoOcupar: (ocupado) => {
      if (ocupado) grupo.setAttribute("aria-busy", "true");
      else grupo.removeAttribute("aria-busy");
      for (const entrada of entradas) {
        if (ocupado) entrada.setAttribute("aria-disabled", "true");
        else entrada.removeAttribute("aria-disabled");
      }
    },
  });
  grupo.addEventListener("click", (evento) => {
    if (grupo.getAttribute("aria-busy") === "true") evento.preventDefault();
  });
  for (const entrada of entradas) entrada.addEventListener("change", () => {
    if (entrada.checked) escolher(entrada.value).catch(() => {});
  });
  return grupo;
}

/**
 * colunas: [{ chave, rotulo, tipo?: "texto" | "num" | "mono", render?(linha) }]
 * Com aoSelecionar, a linha vira selecionável por clique, Enter e Espaço.
 */
export function tabela({ legenda, colunas, linhas, chaveLinha = (l) => l.id, selecionada, aoSelecionar, vazio }) {
  if (linhas.length === 0 && vazio) return vazio;

  const corpo = h(
    "tbody",
    null,
    linhas.map((linha) => {
      const chave = chaveLinha(linha);
      const tr = h(
        "tr",
        {
          dataset: { chave },
          tabIndex: aoSelecionar ? 0 : null,
          "aria-selected": aoSelecionar ? String(chave === selecionada) : null,
        },
        colunas.map((c) => h("td", { class: classeColuna(c) }, c.render ? c.render(linha) : (linha[c.chave] ?? ""))),
      );
      if (aoSelecionar) {
        tr.addEventListener("click", () => aoSelecionar(linha));
        tr.addEventListener("keydown", (evento) => {
          if (evento.target !== tr) return;
          if (evento.key === "Enter" || evento.key === " ") {
            evento.preventDefault();
            aoSelecionar(linha);
          }
        });
      }
      return tr;
    }),
  );

  return h(
    "div",
    { class: `tabela${aoSelecionar ? " tabela--selecionavel" : ""}` },
    h(
      "table",
      null,
      legenda && h("caption", { class: "so-leitor" }, legenda),
      h("thead", null, h("tr", null, colunas.map((c) => h("th", { scope: "col", class: classeColuna(c) }, c.rotulo)))),
      corpo,
    ),
  );
}

function classeColuna(coluna) {
  if (coluna.tipo === "mono") return "mono";
  if (coluna.tipo === "num") return "num";
  return "";
}

export function esqueleto({ linhas = 3, largura = "100%" } = {}) {
  return h(
    "div",
    { class: "esqueleto-grupo", role: "status", "aria-label": "Carregando" },
    Array.from({ length: linhas }, () => h("span", { class: "esqueleto", style: { width: largura } })),
  );
}

/**
 * Abre um diálogo modal. Resolve com o `valor` da ação clicada, ou null se fechar com Esc.
 * acoes: [{ rotulo, valor, variante?, foco?, desabilitado?, aoClicar? }], na ordem de leitura (cancelar primeiro).
 * Se aoClicar lançar, o diálogo continua aberto e mostra a `message` do erro.
 * Devolve { promessa, dialogo }, para o chamador poder habilitar ou desabilitar ações.
 */
export function abrirDialogo({ titulo, conteudo, acoes, largo = false, fallback, raizRetorno }) {
  const raiz = raizRetorno ?? document.activeElement?.closest(".pagina") ?? document.body;
  const foco = guardarFoco(raiz);
  const anuncios = criarAnunciador();
  const idTitulo = novoId("dialogo-titulo");
  const areaErro = h("p", { class: "dialogo__erro", hidden: true });
  let ocupado = false;

  const botoes = acoes.map((a) =>
    botao({
      rotulo: a.rotulo,
      variante: a.variante ?? "secundario",
      desabilitado: a.desabilitado ?? false,
      autofocus: a.foco ? "" : null,
      aoClicar: async () => {
        if (ocupado) return;
        ocupado = true;
        areaErro.hidden = true;
        anuncios.limpar();
        const anteriores = botoes.map((b) => b.getAttribute("aria-disabled"));
        botoes.forEach((b) => b.setAttribute("aria-disabled", "true"));
        try {
          if (a.aoClicar) await a.aoClicar();
        } catch (erro) {
          ocupado = false;
          areaErro.textContent = typeof erro?.message === "string" && erro.message ? erro.message : "Não foi possível concluir. Tente de novo.";
          areaErro.hidden = false;
          anuncios.erro(areaErro.textContent);
          return;
        } finally {
          ocupado = false;
          botoes.forEach((b, i) => {
            if (anteriores[i] === null) b.removeAttribute("aria-disabled");
            else b.setAttribute("aria-disabled", anteriores[i]);
          });
        }
        dialogo.close(a.valor);
      },
    }),
  );

  const dialogo = h(
    "dialog",
    { class: `dialogo${largo ? " dialogo--largo" : ""}`, "aria-labelledby": idTitulo },
    h("div", { class: "dialogo__corpo" }, h("h2", { id: idTitulo }, titulo), conteudo, areaErro, anuncios.no),
    h("div", { class: "dialogo__acoes" }, botoes),
  );
  dialogo.botoes = botoes;
  dialogo.addEventListener("cancel", (evento) => {
    if (ocupado) evento.preventDefault();
  });

  const promessa = new Promise((resolver) => {
    dialogo.addEventListener("close", () => {
      const valor = dialogo.returnValue || null;
      dialogo.remove();
      resolver(valor);
      // Espera também o finally do botão acionador e a atualização de quem aguardava o modal.
      queueMicrotask(() => restaurarFoco(foco, raiz, {
        aposDialogo: dialogo,
        fallback: fallback ?? (() => raiz.querySelector("[data-focus-fallback], h1, h2")),
      }));
    });
  });
  document.body.append(dialogo);
  dialogo.showModal();
  return { promessa, dialogo };
}

/**
 * Confirmação simples. Com `digitar`, o botão só habilita quando o texto digitado for igual.
 * Resolve true ao confirmar, false ao cancelar ou fechar.
 */
export async function confirmar({ titulo, mensagem, rotuloConfirmar = "Confirmar", rotuloCancelar = "Cancelar", perigo = false, digitar, aoConfirmar, fallback, raizRetorno }) {
  const conteudo = h("div", { class: "pilha" }, h("p", null, mensagem));
  let entrada = null;
  if (digitar) {
    const c = campo({ rotulo: `Digite ${digitar} para confirmar`, tipo: "mono" });
    entrada = c.controle;
    conteudo.append(c);
  }
  const { promessa, dialogo } = abrirDialogo({
    titulo,
    conteudo,
    fallback,
    raizRetorno,
    acoes: [
      { rotulo: rotuloCancelar, valor: "cancelar", foco: !!perigo },
      { rotulo: rotuloConfirmar, valor: "confirmar", variante: perigo ? "perigo" : "primario", foco: !perigo && !digitar, desabilitado: !!digitar, aoClicar: aoConfirmar },
    ],
  });
  if (entrada) {
    const confirmarBotao = dialogo.botoes[1];
    entrada.addEventListener("input", () => {
      confirmarBotao.disabled = entrada.value !== digitar;
    });
  }
  return (await promessa) === "confirmar";
}
