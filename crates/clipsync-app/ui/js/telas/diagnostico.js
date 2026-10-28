// Tela 6 · Diagnóstico em duas colunas: Sistema, Rede e Pacote de suporte à esquerda; Adaptadores, Conflitos e
// Últimos bloqueios (sem conteúdo) à direita. "Exportar pacote de suporte" mostra a prévia redigida antes de gravar em Downloads.

import { api } from "../api.js";
import { area, aviso, cabecalho, estadoVazio, indisponivel } from "../areas.js";
import { abrirDialogo, botao, cartao, selo, tabela } from "../componentes.js";
import { h, trocar } from "../dom.js";
import { recurso } from "../estado.js";
import { formatarBytes, formatarMomento, mensagemDe, seloDiagnostico, seloDispositivo } from "../formatar.js";

const AREAS = {
  sistema: { chave: "sistema", titulo: "Sistema" },
  adaptadores: { chave: "adaptadores", titulo: "Adaptadores" },
  conflitos: { chave: "conflitos", titulo: "Conflitos" },
};

const MODULOS = { clipboard: "clipboard", input: "teclado e mouse" };
const REDACOES = {
  peer_id: ["id de par removido", "ids de pares removidos"],
  peer_name: ["nome de par removido", "nomes de pares removidos"],
  ip: ["endereço IP removido", "endereços IP removidos"],
  device_name: ["nome desta máquina removido", "nomes desta máquina removidos"],
};

function descreverRedacao({ kind, count }) {
  const [singular, plural] = REDACOES[kind] ?? [`${kind} removido`, `${kind} removidos`];
  return `${count} ${count === 1 ? singular : plural}`;
}
const EVENTOS = ["peer_connected", "peer_disconnected", "peer_blocked", "resync"];

export default function montar({ assinar }) {
  const avisoTopo = h("div");

  const regiao = area({
    linhas: 8,
    buscar: () => api.diagnosticsGet(),
    pintar: (dados) =>
      h(
        "div",
        { class: "diagnostico__blocos" },
        h(
          "div",
          { class: "diagnostico__coluna" },
          blocoItens(AREAS.sistema, dados.items),
          blocoRede(dados.network, dados.items.filter((i) => i.area === "rede")),
          blocoSuporte(),
        ),
        h("div", { class: "diagnostico__coluna" }, blocoItens(AREAS.adaptadores, dados.items), blocoItens(AREAS.conflitos, dados.items), blocoBloqueios(dados.blocks)),
      ),
  });

  function blocoItens(areaDef, items) {
    const meus = items.filter((i) => i.area === areaDef.chave);
    const outros = areaDef.chave === "conflitos" ? items.filter((i) => !Object.hasOwn(AREAS, i.area) && i.area !== "rede") : [];
    const lista = [...meus, ...outros];
    return cartao({
      titulo: areaDef.titulo,
      filhos: [
        lista.length === 0
          ? h("p", { class: "muted" }, "Nada informado pelo agente.")
          : h(
              "dl",
              { class: "kv diagnostico__kv" },
              lista.flatMap(itemDiagnostico),
            ),
      ],
    });
  }

  function itemDiagnostico(i) {
    return [h("dt", null, i.name), h("dd", null, h("div", { class: "diagnostico__item" }, seloDiagnostico(i.status), i.detail && h("span", null, i.detail)))];
  }

  function blocoRede(network, itens) {
    const pares = network.peers ?? [];
    return cartao({
      titulo: "Rede",
      filhos: [
        h(
          "dl",
          { class: "kv diagnostico__kv" },
          h("dt", null, "Escuta"),
          h("dd", { class: "mono" }, network.listen ?? "não está escutando"),
          h("dt", null, "Endereços"),
          h("dd", null, (network.addresses ?? []).length > 0 ? h("ul", { class: "diagnostico__enderecos mono" }, network.addresses.map((a) => h("li", null, a))) : h("span", { class: "muted" }, "nenhum")),
          itens.flatMap(itemDiagnostico),
        ),
        pares.length === 0
          ? estadoVazio({ titulo: "Nenhum par", texto: "Pareie uma máquina em Dispositivos para ver estado, endereço e latência aqui." })
          : [
              tabela({
                legenda: "Pares e rede",
                colunas: [
                  { chave: "name", rotulo: "Par", render: (p) => h("strong", null, p.name) },
                  { chave: "state", rotulo: "Estado", render: (p) => seloDispositivo(p.state) },
                  { chave: "addr", rotulo: "Endereço", tipo: "mono", render: (p) => p.addr ?? h("span", { class: "muted" }, "sem endereço") },
                  ...(recurso("peer_rtt")
                    ? [
                        { chave: "rtt", rotulo: "RTT", tipo: "num", render: () => "" },
                        { chave: "p95", rotulo: "Entrada p50 / p95", tipo: "num", render: () => "" },
                      ]
                    : []),
                ],
                linhas: pares,
                chaveLinha: (p) => p.peer_id,
              }),
              !recurso("peer_rtt") && h("p", { class: "nota" }, "RTT e latência de entrada por par: ", indisponivel(), "."),
            ],
      ],
    });
  }

  function blocoSuporte() {
    return cartao({
      titulo: "Pacote de suporte",
      filhos: [
        h(
          "p",
          { class: "nota" },
          "Você vê a prévia antes de gravar em Downloads. Identificadores já saem removidos; conteúdo de clipboard, teclas e chaves nunca entram.",
        ),
      ],
    });
  }

  function blocoBloqueios(blocks) {
    return cartao({
      titulo: "Últimos bloqueios",
      acoes: h("span", { class: "nota" }, "motivo, par e horário, nunca o conteúdo"),
      plano: blocks.length > 0,
      filhos: [
        blocks.length === 0
          ? h("div", { class: "diagnostico__vazio" }, estadoVazio({ titulo: "Nenhum bloqueio recente", texto: "Quando o agente recusar algo (par sem permissão, malha pausada, chave desconhecida), o motivo aparece aqui." }))
          : tabela({
              legenda: "Últimos bloqueios",
              colunas: [
                { chave: "at", rotulo: "Horário", tipo: "num", render: (b) => formatarMomento(b.at_unix_ms) },
                { chave: "peer", rotulo: "Par", render: (b) => (b.peer_name ? h("strong", null, b.peer_name) : h("span", { class: "muted" }, "sem par")) },
                { chave: "module", rotulo: "Módulo", render: (b) => (b.module ? MODULOS[b.module] ?? b.module : h("span", { class: "muted" }, "geral")) },
                { chave: "reason", rotulo: "Motivo", render: (b) => b.reason },
              ],
              linhas: blocks,
              chaveLinha: (b) => `${b.at_unix_ms}-${b.peer_id ?? "x"}`,
            }),
      ],
    });
  }

  async function exportar() {
    trocar(avisoTopo);
    let previa;
    try {
      previa = await api.supportBundlePreview();
    } catch (erro) {
      trocar(avisoTopo, aviso("erro", h("p", null, mensagemDe(erro))));
      return;
    }
    const resultado = h("div");
    const { promessa, dialogo } = abrirDialogo({
      largo: true,
      titulo: "Exportar pacote de suporte",
      conteudo: h(
        "div",
        { class: "pilha" },
        h("p", { class: "muted" }, "É isto que vai no arquivo. Identificadores já foram removidos; conteúdo de clipboard, teclas e chaves nunca entram."),
        h(
          "div",
          { class: "linha" },
          previa.redactions.length === 0 ? h("span", { class: "nota" }, "Nada precisou ser removido.") : previa.redactions.map((r) => selo("neutro", descreverRedacao(r))),
        ),
        h("pre", { class: "diagnostico__previa", tabIndex: 0, "aria-label": "Prévia do pacote de suporte" }, previa.text),
        resultado,
      ),
      acoes: [
        { rotulo: "Cancelar", valor: "cancelar" },
        {
          rotulo: "Gravar em Downloads",
          valor: "exportar",
          variante: "primario",
          foco: true,
          aoClicar: async () => {
            const r = await api.supportBundleExport();
            trocar(avisoTopo, aviso("info", selo("ok", "gravado"), h("p", null, `Pacote gravado em ${r.path} (${formatarBytes(r.bytes)}). Anexe esse arquivo ao pedido de ajuda.`)));
          },
        },
      ],
    });
    void dialogo;
    await promessa;
  }

  const cancelar = assinar(EVENTOS, () => regiao.recarregar({ silencioso: true }));

  const no = h(
    "div",
    { class: "diagnostico" },
    cabecalho({
      titulo: "Diagnóstico",
      descricao: "O que o sistema oferece, o que o ClipSync está usando e por que algo foi bloqueado.",
      acoes: [
        botao({ rotulo: "Atualizar", icone: "atualizar", aoClicar: () => regiao.recarregar() }),
        botao({ rotulo: "Exportar pacote de suporte", variante: "primario", icone: "baixar", aoClicar: exportar }),
      ],
    }),
    avisoTopo,
    regiao.no,
  );

  return { no, desmontar: cancelar };
}
