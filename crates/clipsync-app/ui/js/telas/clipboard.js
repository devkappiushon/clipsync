// Tela 5 · Clipboard: faixa de estado do módulo, linha do modo (automático, manual; com confirmação indisponível)
// com os interruptores indisponíveis nesta versão, e a caixa de recebidos em tabela com prévia sob demanda, copiar e descartar.

import { criarAnunciador, guardarFoco, interacaoAtual } from "../acessibilidade.js";
import { api } from "../api.js";
import { area, cabecalho, estadoVazio } from "../areas.js";
import { abrirDialogo, botao, cartao, interruptor, segmentado, selo, tabela } from "../componentes.js";
import { h, trocar, novoId } from "../dom.js";
import { atualizarApp, recurso } from "../estado.js";
import { INDISPONIVEL, formatarBytes, formatarMinutos, formatarMomento, mensagemDe, seloClassificacao, seloItemCaixa } from "../formatar.js";

import { resumoEfeitos } from "./preferencias.js";

export function feedbackModo(resposta) {
  if (resposta.effects) {
    const e = resposta.effects;
    if (e.config_saved === false && e.pause === "not_requested" && e.stop === "not_requested" && e.start === "not_requested") {
      return resposta.restart_needed
        ? "O modo já estava configurado assim. O agente externo pode precisar de reinício para usar essa configuração."
        : "O modo já estava configurado assim. Nenhuma alteração foi solicitada.";
    }
    const resumo = resumoEfeitos(resposta);
    return resposta.restart_needed ? `${resumo} Reinicie o agente externo para aplicar o modo salvo.` : resumo;
  }
  return resposta.restart_needed
    ? "O modo foi salvo na configuração. Reinicie o agente externo para aplicá-lo."
    : "Modo salvo. Atualize para conferir o estado do agente.";
}

const EVENTOS = ["clip_received", "resync"];
const COLUNAS_CAIXA = 6;

function podeCopiar(item) {
  return item.state === "in_inbox" || item.state === "rejected";
}

export default function montar({ ir, assinar }) {
  const avisoTopo = h("div");
  const anuncios = criarAnunciador();
  const previasAbertas = new Map();
  const errosDaCaixa = new Map();
  const idsPrevias = new Map();

  const regiao = area({
    linhas: 6,
    rotulo: "Estado e caixa do clipboard",
    buscar: () => api.clipboardGet(),
    pintar: (dados, recarregar) => {
      for (const guardados of [previasAbertas, errosDaCaixa, idsPrevias]) {
        for (const id of guardados.keys()) if (!dados.inbox.some((item) => item.id === id)) guardados.delete(id);
      }
      return h("div", { class: "clipboard__corpo" }, faixaEstado(dados), linhaModo(dados, recarregar), cartaoCaixa(dados, recarregar));
    },
  });

  const proteger = (tarefa) => (...args) => regiao.proteger(() => tarefa(...args));

  function mostrarAviso(tom, texto) {
    trocar(avisoTopo, texto && h("div", { class: `aviso aviso--${tom}` }, h("p", null, texto)));
    if (!texto) anuncios.limpar();
    else if (tom === "erro") anuncios.erro(texto);
    else anuncios.status(texto);
  }

  function celula(rotulo, valor, classe) {
    return h("div", { class: `clipboard__celula${classe ? ` ${classe}` : ""}` }, h("dt", null, rotulo), h("dd", null, valor));
  }

  function faixaEstado(dados) {
    return h(
      "div",
      { class: "clipboard__estado" },
      dados.enabled === false && h("div", { class: "aviso aviso--aviso" }, h("p", null, "O módulo de clipboard está desligado neste computador. Nada entra nem sai; a caixa fica vazia.")),
      cartao({
        filhos: [
          h(
            "dl",
            { class: "clipboard__faixa" },
            celula("Módulo", dados.enabled === true ? selo("ok", "ligado") : selo("neutro", dados.enabled === false ? "desligado" : "não confirmado")),
            celula("Adaptador", dados.adapter ? h("span", { class: "mono" }, dados.adapter) : h("span", { class: "muted" }, "nenhum")),
            celula("Plataforma", dados.enabled === true ? seloClassificacao(dados.classification)
              : selo("neutro", dados.enabled === false ? "não avaliada com o módulo desligado" : "estado da plataforma não confirmado")),
            celula("Último evento", dados.last_event ?? h("span", { class: "muted" }, "nenhum"), "clipboard__celula--larga"),
            celula("Último bloqueio", dados.last_block ?? h("span", { class: "muted" }, "nenhum"), "clipboard__celula--larga"),
          ),
        ],
      }),
    );
  }

  function linhaModo(dados, recarregar) {
    const modo = segmentado({
      rotulo: "Modo do clipboard",
      chaveFoco: "clipboard-modo",
      valor: dados.mode,
      opcoes: [
        { valor: "auto", rotulo: "Automático" },
        { valor: "manual", rotulo: "Manual" },
        { valor: "confirm", rotulo: "Com confirmação", desabilitada: !recurso("clipboard_confirm_mode") },
      ],
      aoMudar: proteger(async (valor) => {
        mostrarAviso(null);
        try {
          const r = await api.clipboardSetMode({ mode: valor });
          mostrarAviso("info", feedbackModo(r));
          await atualizarApp().catch(() => {});
          await recarregar({ silencioso: true });
        } catch (erro) {
          mostrarAviso("erro", `${mensagemDe(erro)} ${resumoEfeitos(erro.details)}`.trim());
          await atualizarApp().catch(() => {});
          await recarregar({ silencioso: true });
          throw erro;
        }
      }),
    });

    return h(
      "div",
      { class: "clipboard__modo" },
      h(
        "div",
        { class: "clipboard__modo-linha" },
        h("div", { class: "clipboard__modo-escolha" }, h("span", { class: "campo__rotulo" }, "Modo"), modo),
        h(
          "div",
          { class: "clipboard__interruptores" },
          interruptor({ rotulo: "Pausar captura", descricao: INDISPONIVEL, desabilitado: !recurso("clipboard_pause_capture") }),
          interruptor({ rotulo: "Modo privado", descricao: INDISPONIVEL, desabilitado: !recurso("clipboard_private_mode") }),
        ),
      ),
      h(
        "p",
        { class: "nota" },
        `Automático aplica o texto recebido e registra na caixa. Manual só guarda na caixa. Com confirmação: ${INDISPONIVEL}. Se a troca reiniciar o agente deste app, a malha ficará pausada. Use Retomar tudo quando quiser.`,
      ),
    );
  }

  function cartaoCaixa(dados, recarregar) {
    const { inbox, retention } = dados;
    const rodape = h(
      "p",
      { class: "nota clipboard__retencao" },
      `A caixa guarda até ${retention.max_items} itens por ${formatarMinutos(retention.ttl_ms)}, só em memória. Nada é gravado em disco nesta versão.`,
    );

    if (inbox.length === 0) {
      return cartao({
        titulo: "Caixa de recebidos",
        filhos: [
          estadoVazio({
            titulo: "Nenhum item recebido",
            texto: "Quando um par autorizado copiar texto, ele aparece aqui com origem, horário, tipo e tamanho. A prévia só abre quando você pedir.",
          }),
          rodape,
        ],
      });
    }

    const itens = new Map(inbox.map((item, i) => [item.id, itemCaixa(item, recarregar, inbox[i + 1]?.id ?? inbox[i - 1]?.id)]));
    const grade = tabela({
      legenda: "Itens recebidos",
      colunas: [
        { chave: "from", rotulo: "Origem", render: (i) => h("strong", null, i.from) },
        { chave: "received_unix_ms", rotulo: "Horário", tipo: "num", render: (i) => formatarMomento(i.received_unix_ms) },
        { chave: "mime", rotulo: "Tipo", tipo: "mono" },
        { chave: "bytes", rotulo: "Tamanho", tipo: "num", render: (i) => formatarBytes(i.bytes) },
        { chave: "state", rotulo: "Estado", render: (i) => h("div", { class: "clipboard__estado-item" }, seloItemCaixa(i.state), i.reason && h("span", { class: "nota" }, i.reason)) },
        { chave: "acoes", rotulo: "Prévia e ações", render: (i) => itens.get(i.id).acoes },
      ],
      linhas: inbox,
      chaveLinha: (i) => i.id,
    });
    grade.dataset.scrollKey = "clipboard-caixa";
    // A prévia e o erro de cada item ocupam uma linha própria logo abaixo dele, na largura toda.
    for (const tr of [...grade.querySelectorAll("tbody > tr")]) tr.after(itens.get(tr.dataset.chave).extra);

    return cartao({
      titulo: "Caixa de recebidos",
      plano: true,
      acoes: h("span", { class: "nota" }, `${inbox.length} ${inbox.length === 1 ? "item" : "itens"}`),
      filhos: [grade, rodape],
    });
  }

  function itemCaixa(item, recarregar, proximoId) {
    const chave = (acao, id = item.id) => JSON.stringify(["clipboard-item", id, acao]);
    const buscarAcao = (acao, id = item.id) => [...regiao.no.querySelectorAll("[data-focus-key]")]
      .find((alvo) => alvo.dataset.focusKey === chave(acao, id));
    if (!idsPrevias.has(item.id)) idsPrevias.set(item.id, novoId("clipboard-previa"));
    const idPrevia = idsPrevias.get(item.id);
    const previa = h("div", { class: "clipboard__previa-area", id: idPrevia });
    const pintarPrevia = (texto) => trocar(previa, h("pre", { class: "clipboard__previa" }, texto === "" ? "(sem texto guardado)" : texto));
    const erroItem = h("p", { class: "campo__erro", hidden: !errosDaCaixa.has(item.id) }, errosDaCaixa.get(item.id));
    const extra = h("tr", { class: "clipboard__extra", hidden: true }, h("td", { colspan: COLUNAS_CAIXA }, previa, erroItem));
    const sincronizar = () => {
      extra.hidden = previa.childElementCount === 0 && erroItem.hidden;
    };
    const limparErro = () => {
      errosDaCaixa.delete(item.id);
      erroItem.hidden = true;
      sincronizar();
    };
    const falhar = (erro) => {
      erroItem.textContent = mensagemDe(erro);
      errosDaCaixa.set(item.id, erroItem.textContent);
      erroItem.hidden = false;
      anuncios.erro(erroItem.textContent);
      sincronizar();
    };

    const mostrar = botao({
      rotulo: previasAbertas.has(item.id) ? "Ocultar" : "Mostrar",
      variante: "discreto",
      icone: "olho",
      "aria-expanded": String(previasAbertas.has(item.id)),
      "aria-controls": idPrevia,
      dataset: { focusKey: chave("mostrar") },
      aoClicar: proteger(async () => {
        limparErro();
        if (previasAbertas.has(item.id)) {
          previasAbertas.delete(item.id);
          trocar(previa);
          sincronizar();
          mostrar.querySelector(".btn__rotulo").textContent = "Mostrar";
          mostrar.setAttribute("aria-expanded", "false");
          return;
        }
        try {
          const { preview } = await api.clipboardPreview({ id: item.id });
          previasAbertas.set(item.id, preview);
          pintarPrevia(preview);
          sincronizar();
          mostrar.querySelector(".btn__rotulo").textContent = "Ocultar";
          mostrar.setAttribute("aria-expanded", "true");
        } catch (erro) {
          falhar(erro);
        }
      }),
    });
    if (previasAbertas.has(item.id)) {
      pintarPrevia(previasAbertas.get(item.id));
      sincronizar();
    }

    const idMotivoCopiar = novoId("clipboard-copiar-motivo");
    const motivoCopiar = !podeCopiar(item) && h("span", { id: idMotivoCopiar, class: "so-leitor" },
      "Este item já foi aplicado ou não está disponível para copiar novamente.");
    const copiar = botao({
      rotulo: "Copiar",
      dataset: { focusKey: chave("copiar") },
      variante: "discreto",
      icone: "copiar",
      desabilitado: !podeCopiar(item),
      "aria-describedby": !podeCopiar(item) ? idMotivoCopiar : null,
      aoClicar: proteger(async () => {
        limparErro();
        try {
          await api.clipboardCopyItem({ id: item.id });
          await recarregar({ silencioso: true, focoFallback: () => buscarAcao("mostrar") });
          anuncios.status("Item copiado para este computador.");
        } catch (erro) {
          falhar(erro);
        }
      }),
    });
    const descartar = botao({
      rotulo: "Descartar",
      dataset: { focusKey: chave("descartar") },
      variante: "discreto",
      icone: "descartar",
      aoClicar: proteger(async () => {
        limparErro();
        try {
          await api.clipboardDiscardItem({ id: item.id });
          previasAbertas.delete(item.id);
          await recarregar({ silencioso: true, focoFallback: () => buscarAcao("mostrar", proximoId) ?? no.querySelector("[data-focus-fallback]") });
          anuncios.status("Item descartado da caixa.");
        } catch (erro) {
          falhar(erro);
        }
      }),
    });

    sincronizar();
    return { extra, acoes: h("div", { class: "clipboard__item-acoes" }, mostrar, copiar, motivoCopiar, descartar) };
  }

  async function enviarAgora() {
    const foco = guardarFoco(no);
    mostrarAviso(null);
    let resultado;
    try {
      resultado = await api.clipboardSendNow({ peer_id: null });
    } catch (erro) {
      mostrarAviso("erro", mensagemDe(erro));
      return;
    }
    if (!no.isConnected) return;
    if (!interacaoAtual(foco, no)) {
      mostrarAviso("info", "Envio concluído. O resultado está disponível para consulta.");
      avisoTopo.append(botao({ rotulo: "Ver resultado do envio", aoClicar: () => mostrarEnvio(resultado) }));
      return;
    }
    await mostrarEnvio(resultado);
  }

  async function mostrarEnvio(resultado) {
    const { promessa } = abrirDialogo({
      titulo: "Clipboard enviado",
      conteudo: h(
        "ul",
        { class: "clipboard__resultados" },
        resultado.outcomes.map((o) => h("li", null, h("strong", null, o.name), " ", seloItemCaixa(o.state), o.reason && h("span", { class: "muted" }, ` ${o.reason}`))),
      ),
      acoes: [{ rotulo: "Fechar", valor: "fechar", variante: "primario", foco: true }],
    });
    await promessa;
    regiao.recarregar({ silencioso: true });
  }

  const cancelar = assinar(EVENTOS, () => regiao.recarregar({ silencioso: true }));

  const no = h(
    "div",
    { class: "clipboard" },
    cabecalho({
      titulo: "Clipboard",
      descricao: "Texto copiado aqui vai aos pares autorizados. O que chega fica na caixa até você decidir.",
      acoes: [
        botao({ rotulo: "Enviar agora", icone: "enviar", dataset: { focusKey: "clipboard-enviar", focusFallback: "" }, aoClicar: enviarAgora }),
        botao({ rotulo: "Permissões", variante: "discreto", icone: "dispositivos", aoClicar: () => ir("dispositivos") }),
      ],
    }),
    anuncios.no,
    avisoTopo,
    regiao.no,
  );

  return { no, desmontar: () => { cancelar(); regiao.desmontar(); previasAbertas.clear(); errosDaCaixa.clear(); idsPrevias.clear(); } };
}
