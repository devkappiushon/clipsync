// Estado calculado pelo backend. A tela não deduz permissão remota nem saúde dos hooks.
import { aviso } from "./areas.js";
import { botao } from "./componentes.js";
import { h } from "./dom.js";

export function resumoTravessia(app, { peerId = null, peerNome = null, ir, configurar, acoes = true } = {}) {
  const travessia = app?.traversal;
  const estado = peerId ? travessia?.peers?.find((p) => p.peer_id === peerId) : travessia;
  const code = estado?.code ?? "checking";
  const detail = estado?.detail ?? "Conferindo a configuração da passagem do mouse.";
  const tom = ["active", "unverified", "checking"].includes(code) ? "info" : "aviso";
  const rota = code === "no_local_link" ? "layout" : code === "input_off" ? "preferencias" : "diagnostico";
  const acao = code === "no_local_link" ? "Configurar saída deste computador" : "Abrir diagnóstico";
  return aviso(tom,
    h("div", { class: "travessia__texto" },
      h("strong", null, peerId ? `Passagem deste computador para ${peerNome || "o outro computador"}` : "Passagem do mouse"),
      h("p", null, detail),
      estado?.protocol_warning && h("p", null, estado.protocol_warning),
      acoes && code !== "checking" && code !== "active" && botao({ rotulo: rota === "preferencias" ? "Abrir preferências" : acao, variante: "discreto", aoClicar: () => code === "no_local_link" && configurar ? configurar() : ir?.(rota) }),
      !peerId && travessia?.last_block && h("details", null, h("summary", null, "Último bloqueio registrado"), h("p", { class: "nota" }, "Histórico, pode não ser a situação atual."), h("p", null, travessia.last_block)),
    ),
  );
}
