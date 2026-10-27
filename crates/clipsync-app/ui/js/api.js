// Único ponto de acesso a dados. Dentro do Tauri chama o agente;
// no navegador cai no mock. Os nomes dos comandos são provisórios
// até o contrato §12 sair.

import { invocar as invocarMock } from "./mock.js";

function chamar(comando, args) {
  const invoke = window.__TAURI__?.core?.invoke;
  return invoke ? invoke(comando, args) : invocarMock(comando, args);
}

export const api = {
  estadoGeral: () => chamar("estado_geral"),
  pausarTudo: (pausado) => chamar("pausar_tudo", { pausado }),
};
