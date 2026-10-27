// Roda antes da pintura para o tema salvo não piscar. A lógica de troca fica em tema.js.
(function () {
  try {
    var modo = localStorage.getItem("clipsync.tema");
    if (modo === "claro") document.documentElement.setAttribute("data-theme", "light");
    else if (modo === "escuro") document.documentElement.setAttribute("data-theme", "dark");
  } catch (e) {
    /* sem armazenamento: segue o tema do sistema */
  }
})();
