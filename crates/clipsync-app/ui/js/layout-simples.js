// Configuração por computador: o usuário escolhe a posição; os monitores vêm do layout real.
import { aviso } from "./areas.js";
import { botao, campo, cartao } from "./componentes.js";
import { h, trocar } from "./dom.js";
import { ligacoesDoPar, posicaoRelativa, proporLigacao, resultadoDoTeste } from "./travessia.js";

const POSICOES = { left: "À esquerda", right: "À direita", top: "Acima", bottom: "Abaixo" };

export function configurarPosicao({ layout, escolha, ocupado, salvando, resultado, aoGravar, aoTestar, aoAvancado, aoParear, aoDiagnostico, aoPrevia }) {
  const pares = layout.devices.filter((d) => !d.is_self);
  const elegiveis = pares.filter((d) => d.state === "connected" && d.monitors.length > 0);
  if (!pares.some((d) => d.peer_id === escolha.peer_id)) escolha.peer_id = elegiveis[0]?.peer_id ?? pares[0]?.peer_id ?? "";
  if (!escolha.edge) escolha.edge = posicaoRelativa(layout, escolha.peer_id) ?? "right";
  if (!pares.length) return cartao({ titulo: "Posição do computador", filhos: [h("p", null, "Pareie outro computador para configurar a passagem do mouse."), botao({ rotulo: "Parear computador", variante: "primario", dataset: { focusKey: "layout-parear" }, aoClicar: aoParear })] });
  const previa = h("div", { class: "pilha layout__previa" });
  const botoes = [];
  let proposta = null;

  const gravar = botao({
    rotulo: "Ligar nessa posição", dataset: { focusKey: "layout-ligar" }, variante: "primario", icone: "avancar",
    aoClicar: async () => { if (proposta) await aoGravar(proposta); },
  });
  const testar = botao({
    rotulo: ocupado ? "Teste em andamento..." : "Testar passagem (5 s)", dataset: { focusKey: "layout-testar" }, icone: "teclado",
    aoClicar: () => aoTestar(escolha.peer_id),
  });
  const destino = campo({
    rotulo: "Outro computador", tipo: "select", dataset: { focusKey: "layout-destino" }, valor: escolha.peer_id,
    opcoes: pares.length ? pares.map((d) => ({ valor: d.peer_id, rotulo: `${d.name}${d.state !== "connected" ? " (offline)" : ""}` })) : [{ valor: "", rotulo: "Pareie um computador primeiro" }],
    aoMudar: (peerId) => { escolha.posicao = null; escolha.peer_id = peerId; escolha.edge = posicaoRelativa(layout, peerId) ?? "right"; atualizar(); },
  });
  const posicoes = h("div", { class: "layout__posicoes", role: "group", "aria-label": "Posição do outro computador" },
    Object.entries(POSICOES).map(([edge, rotulo]) => {
      const b = botao({ rotulo, dataset: { focusKey: `layout-posicao:${edge}` }, "aria-pressed": String(escolha.edge === edge), aoClicar: () => { escolha.posicao = null; escolha.edge = edge; atualizar(); } });
      botoes.push([edge, b]);
      return b;
    }),
  );

  function atualizar() {
    proposta = proporLigacao(layout, escolha);
    const par = pares.find((d) => d.peer_id === escolha.peer_id);
    const ligacoes = ligacoesDoPar(layout, escolha.peer_id);
    gravar.disabled = !proposta || ocupado;
    gravar.setAttribute("aria-disabled", String(!proposta || ocupado || salvando));
    gravar.setAttribute("aria-busy", String(salvando));
    testar.setAttribute("aria-disabled", String(ocupado || salvando));
    testar.setAttribute("aria-busy", String(ocupado));
    testar.disabled = salvando || !par || par.state !== "connected" || ligacoes.length === 0;
    for (const [edge, b] of botoes) {
      b.setAttribute("aria-pressed", String(escolha.edge === edge));
      b.disabled = ocupado || salvando || !par;
    }
    let mensagem;
    if (!par) mensagem = "Pareie o computador que ficará ao lado deste em Dispositivos.";
    else if (par.state !== "connected") mensagem = "Conecte o outro computador para configurar a passagem.";
    else if (!par.monitors.length || !layout.devices.find((d) => d.is_self)?.monitors.length) mensagem = "Aguardando os monitores reais dos computadores. Confira o módulo de teclado e mouse e as permissões no destino.";
    else if (ligacoes.length > 1) mensagem = "Este computador tem várias ligações personalizadas. Use Ajustes avançados para alterá-las sem apagar sua configuração.";
    else if (!proposta) mensagem = "Não há trecho livre nessa posição. Escolha outro lado ou confira as ligações em Ajustes avançados.";
    else {
      const bordas = { left: "esquerda", right: "direita", top: "superior", bottom: "inferior" };
      const nome = (p) => layout.devices.find((d) => d.peer_id === p.peer_id)?.monitors.find((m) => m.ref === p.monitor)?.name || p.monitor;
      mensagem = `O mouse sai pela borda ${bordas[proposta.a.edge]} de ${nome(proposta.a)} deste computador e entra pela borda ${bordas[proposta.b.edge]} de ${nome(proposta.b)} de ${par.name}.`;
    }
    aoPrevia?.();
    trocar(previa, h("p", null, mensagem), ligacoes.length > 1 && botao({ rotulo: "Abrir ajustes avançados", variante: "discreto", aoClicar: aoAvancado }));
  }
  atualizar();
  const res = resultado && resultadoDoTeste(resultado.stats);
  return cartao({
    titulo: "Posição do computador",
    filhos: [
      h("p", { class: "nota" }, "Escolha um lado ou arraste o computador no desenho."),
      h("div", { class: "layout__configurar" }, destino, h("div", { class: "pilha" }, h("span", { class: "campo__rotulo" }, "Em relação a este computador"), posicoes)),
      previa,
      h("div", { class: "linha" }, gravar),
      h("details", { class: "layout__teste", dataset: { focusKey: "layout-teste-detalhes" } },
        h("summary", null, "Conferir a passagem do mouse"),
        h("p", { class: "nota" }, "Após clicar em Testar, empurre a borda ligada e mova o mouse durante 5 segundos. O teste pode controlar o destino e abrir um pedido de permissão do sistema; você precisa aceitá-lo."),
        testar,
        h("p", { class: "nota" }, "O resultado conta eventos recebidos. Confira a ida e a volta manualmente; este teste não verifica o retorno pela borda."),
      ),
      salvando && aviso("info", h("p", null, "Salvando a posição e a passagem...")),
      ocupado && aviso("info", h("p", null, "Teste em andamento. Cruze a borda ligada e mova o mouse. Aguardando o resultado do agente.")),
      res && h("div", { class: `aviso aviso--${res.tom}` }, h("strong", null, `${res.titulo}${resultado.nome ? `: ${resultado.nome}` : ""}`), h("p", null, res.texto), resultado.stats.received === 0 && botao({ rotulo: "Abrir diagnóstico", variante: "discreto", aoClicar: aoDiagnostico })),
    ],
  });
}
