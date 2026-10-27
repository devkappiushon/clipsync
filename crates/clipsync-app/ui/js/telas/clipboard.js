// Tela 5 · Clipboard: estado do módulo, modo (automático, manual; com confirmação indisponível),
// interruptores indisponíveis nesta versão, caixa de recebidos com prévia sob demanda, copiar e descartar.

import { api } from "../api.js";
import { area, aviso, cabecalho, estadoVazio, indisponivel } from "../areas.js";
import { abrirDialogo, botao, cartao, interruptor, segmentado, selo } from "../componentes.js";
import { h, trocar } from "../dom.js";
import { recurso } from "../estado.js";
import { INDISPONIVEL, formatarBytes, formatarMinutos, formatarMomento, formatarTipo, mensagemDe, seloClassificacao, seloItemCaixa } from "../formatar.js";

const EVENTOS = ["clip_received", "resync"];

function podeCopiar(item) {
  return item.state === "in_inbox" || item.state === "rejected";
}

export default function montar({ ir, assinar }) {
  const avisoTopo = h("div");
  const previasAbertas = new Set();

  const regiao = area({
    linhas: 6,
    buscar: () => api.clipboardGet(),
    pintar: (dados, recarregar) => h("div", { class: "pilha" }, cartaoModulo(dados, recarregar), cartaoCaixa(dados, recarregar)),
  });

  function mostrarAviso(tom, texto) {
    trocar(avisoTopo, texto && aviso(tom, h("p", null, texto)));
  }

  function cartaoModulo(dados, recarregar) {
    const nota = h("p", { class: "nota" }, "Automático aplica o texto recebido e registra na caixa. Manual só guarda na caixa, para você copiar quando quiser.");
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

    return cartao({
      titulo: "Módulo",
      filhos: [
        !dados.enabled && aviso("aviso", h("p", null, "O módulo de clipboard está desligado nesta máquina. Nada entra nem sai; a caixa fica vazia.")),
        h(
          "dl",
          { class: "kv" },
          h("dt", null, "Estado"),
          h("dd", null, dados.enabled ? selo("ok", "ligado") : selo("neutro", "desligado")),
          h("dt", null, "Adaptador"),
          h("dd", null, dados.adapter ? h("span", { class: "mono" }, dados.adapter) : h("span", { class: "muted" }, "nenhum")),
          h("dt", null, "Plataforma"),
          h("dd", null, seloClassificacao(dados.classification)),
          h("dt", null, "Último evento"),
          h("dd", null, dados.last_event ?? h("span", { class: "muted" }, "nenhum")),
          h("dt", null, "Último bloqueio"),
          h("dd", null, dados.last_block ?? h("span", { class: "muted" }, "nenhum")),
        ),
        h("div", { class: "clipboard__modo" }, h("span", { class: "campo__rotulo" }, "Modo"), modo, nota, h("p", { class: "nota" }, `Com confirmação: ${INDISPONIVEL}.`), erroModo),
        h(
          "div",
          { class: "clipboard__interruptores" },
          interruptor({ rotulo: "Pausar captura", descricao: INDISPONIVEL, desabilitado: !recurso("clipboard_pause_capture") }),
          interruptor({ rotulo: "Modo privado", descricao: INDISPONIVEL, desabilitado: !recurso("clipboard_private_mode") }),
        ),
      ],
    });
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

    const lista = h(
      "ul",
      { class: "clipboard__caixa", "aria-label": "Itens recebidos" },
      inbox.map((item) => itemCaixa(item, recarregar)),
    );
    return cartao({ titulo: "Caixa de recebidos", acoes: h("span", { class: "nota" }, `${inbox.length} ${inbox.length === 1 ? "item" : "itens"}`), filhos: [lista, rodape] });
  }

  function itemCaixa(item, recarregar) {
    const previa = h("div", { class: "clipboard__previa-area" });
    const erroItem = h("p", { class: "campo__erro", role: "alert", hidden: true });
    const falhar = (erro) => {
      erroItem.textContent = mensagemDe(erro);
      erroItem.hidden = false;
    };

    const mostrar = botao({
      rotulo: previasAbertas.has(item.id) ? "Ocultar" : "Mostrar",
      variante: "discreto",
      icone: "olho",
      "aria-expanded": String(previasAbertas.has(item.id)),
      aoClicar: async () => {
        erroItem.hidden = true;
        if (previasAbertas.has(item.id)) {
          previasAbertas.delete(item.id);
          trocar(previa);
          mostrar.querySelector(".btn__rotulo").textContent = "Mostrar";
          mostrar.setAttribute("aria-expanded", "false");
          return;
        }
        try {
          const { preview } = await api.clipboardPreview({ id: item.id });
          previasAbertas.add(item.id);
          trocar(previa, h("pre", { class: "clipboard__previa" }, preview === "" ? h("span", { class: "muted" }, "(sem texto guardado)") : preview));
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
      icone: "copiar",
      desabilitado: !podeCopiar(item),
      aoClicar: async () => {
        erroItem.hidden = true;
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
        erroItem.hidden = true;
        try {
          await api.clipboardDiscardItem({ id: item.id });
          previasAbertas.delete(item.id);
          recarregar({ silencioso: true });
        } catch (erro) {
          falhar(erro);
        }
      },
    });

    return h(
      "li",
      { class: "clipboard__item" },
      h(
        "div",
        { class: "clipboard__item-topo" },
        h(
          "div",
          { class: "clipboard__item-meta" },
          h("strong", null, item.from),
          h("span", { class: "num" }, formatarMomento(item.received_unix_ms)),
          h("span", null, formatarTipo(item.mime)),
          h("span", { class: "num" }, formatarBytes(item.bytes)),
          seloItemCaixa(item.state),
          item.reason && h("span", { class: "muted" }, item.reason),
        ),
        h("div", { class: "clipboard__item-acoes" }, mostrar, copiar, descartar),
      ),
      previa,
      erroItem,
    );
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
        botao({ rotulo: "Enviar agora", variante: "primario", icone: "enviar", atalho: "Win+Alt+V", aoClicar: enviarAgora }),
        botao({ rotulo: "Permissões", variante: "discreto", icone: "dispositivos", aoClicar: () => ir("dispositivos") }),
      ],
    }),
    avisoTopo,
    regiao.no,
  );

  return { no, desmontar: cancelar };
}
