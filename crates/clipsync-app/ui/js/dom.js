// Construtor de elementos. Texto entra sempre como nó de texto, nunca como HTML.

import { guardarFoco, restaurarFoco } from "./acessibilidade.js";

const PROPRIEDADES = new Set(["value", "checked", "disabled", "selected", "tabIndex", "id", "name", "type"]);

export function h(tag, props, ...filhos) {
  const no = document.createElement(tag);
  for (const [chave, valor] of Object.entries(props ?? {})) {
    if (valor == null || valor === false) continue;
    if (chave === "class") no.className = valor;
    else if (chave === "dataset") Object.assign(no.dataset, valor);
    else if (chave === "style") Object.assign(no.style, valor);
    else if (chave.startsWith("on")) no.addEventListener(chave.slice(2).toLowerCase(), valor);
    else if (PROPRIEDADES.has(chave)) no[chave] = valor;
    else no.setAttribute(chave, valor === true ? "" : valor);
  }
  no.append(...limpar(filhos));
  return no;
}

/** Tira null, undefined e booleanos de uma lista de filhos: condição `x && no` nunca vira texto "false". */
export function limpar(filhos) {
  return filhos.flat(Infinity).filter((f) => f != null && typeof f !== "boolean");
}

/** replaceChildren com a mesma limpeza de h(): use no lugar de no.replaceChildren(...). */
export function trocar(no, ...filhos) {
  const foco = guardarFoco(no);
  const estados = [...no.querySelectorAll("[data-scroll-key], details[data-focus-key]")].map((alvo) => ({
    chave: alvo.dataset.scrollKey ?? alvo.dataset.focusKey,
    top: alvo.scrollTop, left: alvo.scrollLeft, aberto: alvo.tagName === "DETAILS" ? alvo.open : undefined,
  }));
  const top = no.scrollTop;
  const left = no.scrollLeft;
  no.replaceChildren(...limpar(filhos));
  for (const alvo of no.querySelectorAll("[data-scroll-key], details[data-focus-key]")) {
    const estado = estados.find((e) => e.chave === (alvo.dataset.scrollKey ?? alvo.dataset.focusKey));
    if (!estado) continue;
    if (estado.aberto !== undefined) alvo.open = estado.aberto;
    alvo.scrollTop = estado.top;
    alvo.scrollLeft = estado.left;
  }
  no.scrollTop = top;
  no.scrollLeft = left;
  restaurarFoco(foco, no);
}

let contador = 0;
export function novoId(prefixo = "c") {
  contador += 1;
  return `${prefixo}-${contador}`;
}
