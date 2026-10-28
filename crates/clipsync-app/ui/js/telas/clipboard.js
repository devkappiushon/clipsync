// Tela 5 · Clipboard: faixa de estado do módulo, linha do modo (automático, manual; com confirmação indisponível)
// com os interruptores indisponíveis nesta versão, e a caixa de recebidos em tabela com prévia sob demanda, copiar e descartar.

import { api } from "../api.js";
import { area, aviso, cabecalho, estadoVazio } from "../areas.js";
import { abrirDialogo, botao, cartao, interruptor, segmentado, selo, tabela } from "../componentes.js";
import { h, trocar } from "../dom.js";
import { recurso } from "../estado.js";
import { INDISPONIVEL, formatarBytes, formatarMinutos, formatarMomento, mensagemDe, seloClassificacao, seloItemCaixa } from "../formatar.js";

const EVENTOS = ["clip_received", "resync"];
const COLUNAS_CAIXA = 6;

function podeCopiar(item) {
  return item.state === "in_inbox" || item.state === "rejected";
}

export default function montar({ ir, assinar }) {
  const avisoTopo = h("div");
  const previasAbertas = new Set();

  const regiao = area({
    linhas: 6,
    buscar: () => api.clipboardGet(),
    pintar: (dados, recarregar) => h("div", { class: "clipboard__corpo" }, faixaEstado(dados), linhaModo(dados, recarregar), cartaoCaixa(dados, recarregar)),
  });

  function mostrarAviso(tom, texto) {
    trocar(avisoTopo, texto && aviso(tom, h("p", null, texto)));
  }

  function celula(rotulo, valor, classe) {
    return h("div", { class: `clipboard__celula${classe ? ` ${classe}` : ""}` }, h("dt", null, rotulo), h("dd", null, valor));
  }

  function faixaEstado(dados) {
    return h(
      "div",
      { class: "clipboard__estado" },
      !dados.enabled && aviso("aviso", h("p", null, "O módulo de clipboard está desligado nesta máquina. Nada entra nem sai; a caixa fica vazia.")),
      cartao({
        filhos: [
          h(
            "dl",
            { class: "clipboard__faixa" },
            celula("Módulo", dados.enabled ? selo("ok", "ligado") : selo("neutro", "desligado")),
            celula("Adaptador", dados.adapter ? h("span", { class: "mono" }, dados.adapter) : h("span", { class: "muted" }, "nenhum")),
            celula("Plataforma", seloClassificacao(dados.classification)),
            celula("Último evento", dados.last_event ?? h("span", { class: "muted" }, "nenhum"), "clipboard__celula--larga"),
            celula("Último bloqueio", dados.last_block ?? h("span", { class: "muted" }, "nenhum"), "clipboard__celula--larga"),
          ),
        ],
      }),
    );
  }

  function linhaModo(dados, recarregar) {
    const erroModo = h("p", { class: "campo__erro", role: "alert", hidden: true });
    const modo = segmentado({
      rotulo: "Modo do clipboard",
      valor: dados.mode,
      opcoes: [
        { valor: "auto", rotulo: "Automático" },
        { valor: "manual", rotulo: "Manual" },
        { valor: "confirm", rotulo: "Com confirmação", desabilitada: !recurso("clipboard_confirm_mode") },
      ],
      aoMudar: async (valor) => {
        erroModo.hidden = true;
        modo.setAttribute("aria-busy", "true");
        try {
          const r = await api.clipboardSetMode({ mode: valor });
          mostrarAviso("info", r.restart_needed ? "O modo foi gravado. Reinicie o agente do terminal (clipsync run) para ele valer." : null);
          recarregar({ silencioso: true });
        } catch (erro) {
          erroModo.textContent = mensagemDe(erro);
          erroModo.hidden = false;
          recarregar({ silencioso: true });
        } finally {
          modo.removeAttribute("aria-busy");
        }
      },
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
        `Automático aplica o texto recebido e registra na caixa. Manual só guarda na caixa. Com confirmação: ${INDISPONIVEL}.`,
      ),
      erroModo,
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

    const itens = new Map(inbox.map((item) => [item.id, itemCaixa(item, recarregar)]));
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
    // A prévia e o erro de cada item ocupam uma linha própria logo abaixo dele, na largura toda.
    for (const tr of [...grade.querySelectorAll("tbody > tr")]) tr.after(itens.get(tr.dataset.chave).extra);

    return cartao({
      titulo: "Caixa de recebidos",
      plano: true,
      acoes: h("span", { class: "nota" }, `${inbox.length} ${inbox.length === 1 ? "item" : "itens"}`),
      filhos: [grade, rodape],
    });
  }

  function itemCaixa(item, recarregar) {
    const previa = h("div", { class: "clipboard__previa-area" });
    const erroItem = h("p", { class: "campo__erro", role: "alert", hidden: true });
    const extra = h("tr", { class: "clipboard__extra", hidden: true }, h("td", { colspan: COLUNAS_CAIXA }, previa, erroItem));
    const sincronizar = () => {
      extra.hidden = previa.childElementCount === 0 && erroItem.hidden;
    };
    const limparErro = () => {
      erroItem.hidden = true;
      sincronizar();
    };
    const falhar = (erro) => {
      erroItem.textContent = mensagemDe(erro);
      erroItem.hidden = false;
      sincronizar();
    };

    const mostrar = botao({
      rotulo: previasAbertas.has(item.id) ? "Ocultar" : "Mostrar",
      variante: "discreto",
      icone: "olho",
      "aria-expanded": String(previasAbertas.has(item.id)),
      aoClicar: async () => {
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
          previasAbertas.add(item.id);
          trocar(previa, h("pre", { class: "clipboard__previa" }, preview === "" ? h("span", { class: "muted" }, "(sem texto guardado)") : preview));
          sincronizar();
          mostrar.querySelector(".btn__rotulo").textContent = "Ocultar";
          mostrar.setAttribute("aria-expanded", "true");
        } catch (erro) {
          falhar(erro);
        }
      },
    });
    if (previasAbertas.has(item.id)) mostrar.click();

    const copiar = botao({
      rotulo: "Copiar",
      variante: "discreto",
      icone: "copiar",
      desabilitado: !podeCopiar(item),
      aoClicar: async () => {
        limparErro();
        try {
          await api.clipboardCopyItem({ id: item.id });
          recarregar({ silencioso: true });
        } catch (erro) {
          falhar(erro);
        }
      },
    });
    const descartar = botao({
      rotulo: "Descartar",
      variante: "discreto",
      icone: "descartar",
      aoClicar: async () => {
        limparErro();
        try {
          await api.clipboardDiscardItem({ id: item.id });
          previasAbertas.delete(item.id);
          recarregar({ silencioso: true });
        } catch (erro) {
          falhar(erro);
        }
      },
    });

    return { extra, acoes: h("div", { class: "clipboard__item-acoes" }, mostrar, copiar, descartar) };
  }

  async function enviarAgora() {
    mostrarAviso(null);
    let resultado;
    try {
      resultado = await api.clipboardSendNow({ peer_id: null });
    } catch (erro) {
      mostrarAviso("erro", mensagemDe(erro));
      return;
    }
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
        botao({ rotulo: "Enviar agora", icone: "enviar", atalho: "Win+Alt+V", aoClicar: enviarAgora }),
        botao({ rotulo: "Permissões", variante: "discreto", icone: "dispositivos", aoClicar: () => ir("dispositivos") }),
      ],
    }),
    avisoTopo,
    regiao.no,
  );

  return { no, desmontar: cancelar };
}
