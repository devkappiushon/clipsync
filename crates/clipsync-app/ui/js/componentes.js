// Componentes da interface. Toda ação assíncrona passa por comCarregamento:
// o botão fica desabilitado, mostra o indicador e volta ao normal no fim.

import { h, novoId } from "./dom.js";
import { icone } from "./icones.js";

export async function comCarregamento(botao, tarefa) {
  if (botao.getAttribute("aria-busy") === "true") return undefined;
  botao.disabled = true;
  botao.setAttribute("aria-busy", "true");
  try {
    return await tarefa();
  } catch (erro) {
    console.error("falha em ação assíncrona", erro);
    return undefined;
  } finally {
    botao.disabled = false;
    botao.removeAttribute("aria-busy");
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

/** Interruptor com rótulo e descrição. aoMudar(marcado) pode ser assíncrono; se falhar, volta ao valor anterior. */
export function interruptor({ rotulo, descricao, marcado = false, desabilitado = false, aoMudar }) {
  const id = novoId("interruptor");
  const idDescricao = `${id}-d`;
  const entrada = h("input", {
    class: "interruptor",
    type: "checkbox",
    role: "switch",
    id,
    checked: marcado,
    disabled: desabilitado,
    "aria-describedby": descricao ? idDescricao : null,
  });
  entrada.addEventListener("change", async () => {
    if (!aoMudar) return;
    const valor = entrada.checked;
    entrada.disabled = true;
    entrada.setAttribute("aria-busy", "true");
    try {
      await aoMudar(valor);
    } catch (erro) {
      console.error("falha ao mudar o interruptor", erro);
      entrada.checked = !valor;
    } finally {
      entrada.disabled = desabilitado;
      entrada.removeAttribute("aria-busy");
    }
  });
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
    controle = h("select", comuns, opcoes.map((o) => h("option", { value: o.valor, selected: o.valor === valor }, o.rotulo)));
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

/** Seletor segmentado: grupo de opções exclusivas, navegável por setas. */
export function segmentado({ rotulo, opcoes, valor, aoMudar }) {
  const nome = novoId("segmentado");
  const grupo = h(
    "div",
    { class: "segmentado", role: "radiogroup", "aria-label": rotulo },
    opcoes.map((o) =>
      h(
        "label",
        { class: "segmentado__opcao" },
        h("input", { type: "radio", name: nome, value: o.valor, checked: o.valor === valor, onChange: () => aoMudar?.(o.valor) }),
        h("span", null, o.rotulo),
      ),
    ),
  );
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
 * Devolve { promessa, dialogo }, para o chamador poder habilitar ou desabilitar ações.
 */
export function abrirDialogo({ titulo, conteudo, acoes }) {
  const idTitulo = novoId("dialogo-titulo");
  let ocupado = false;

  const botoes = acoes.map((a) =>
    botao({
      rotulo: a.rotulo,
      variante: a.variante ?? "secundario",
      desabilitado: a.desabilitado ?? false,
      autofocus: a.foco ? "" : null,
      aoClicar: async () => {
        ocupado = true;
        try {
          if (a.aoClicar) await a.aoClicar();
        } catch (erro) {
          ocupado = false;
          throw erro;
        }
        ocupado = false;
        dialogo.close(a.valor);
      },
    }),
  );

  const dialogo = h(
    "dialog",
    { class: "dialogo", "aria-labelledby": idTitulo },
    h("div", { class: "dialogo__corpo" }, h("h2", { id: idTitulo }, titulo), conteudo),
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
export async function confirmar({ titulo, mensagem, rotuloConfirmar = "Confirmar", rotuloCancelar = "Cancelar", perigo = false, digitar, aoConfirmar }) {
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
