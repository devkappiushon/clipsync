// Construtor de elementos. Texto entra sempre como nó de texto, nunca como HTML.

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
  no.append(...filhos.flat(Infinity).filter((f) => f != null && f !== false));
  return no;
}

let contador = 0;
export function novoId(prefixo = "c") {
  contador += 1;
  return `${prefixo}-${contador}`;
}
