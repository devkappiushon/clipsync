export const CHAVE_TEMA = "clipsync.tema";

export function lerTema() {
  try {
    return localStorage.getItem(CHAVE_TEMA) || "sistema";
  } catch {
    return "sistema";
  }
}

export function aplicarTema(modo) {
  const raiz = document.documentElement;
  if (modo === "claro") raiz.setAttribute("data-theme", "light");
  else if (modo === "escuro") raiz.setAttribute("data-theme", "dark");
  else raiz.removeAttribute("data-theme");
  try {
    localStorage.setItem(CHAVE_TEMA, modo);
  } catch {
    /* sem armazenamento: o tema vale só nesta sessão */
  }
}
