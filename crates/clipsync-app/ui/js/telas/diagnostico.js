// Tela 6 · Diagnóstico em duas colunas: Sistema, Rede e Pacote de suporte à esquerda; Adaptadores, Conflitos e
// Últimos bloqueios (sem conteúdo) à direita. "Exportar pacote de suporte" mostra a prévia redigida antes de gravar em Downloads.

import { criarAnunciador, guardarFoco, interacaoAtual } from "../acessibilidade.js";
import { api } from "../api.js";
import { area, cabecalho, estadoVazio, indisponivel } from "../areas.js";
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
  const anuncios = criarAnunciador();
  function mostrarAviso(tom, texto) {
    trocar(avisoTopo, texto && h("div", { class: `aviso aviso--${tom}` }, h("p", null, texto)));
    if (!texto) anuncios.limpar();
    else if (tom === "erro") anuncios.erro(texto);
    else anuncios.status(texto);
  }

  const regiao = area({
    linhas: 8,
    rotulo: "Resultado do diagnóstico",
    fallback: () => no.querySelector("[data-focus-fallback]"),
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
          ? estadoVazio({ titulo: "Nenhum par", texto: "Pareie um computador em Dispositivos para acompanhar o estado e o endereço disponível." })
          : [
              tabela({
                legenda: "Pares e rede",
                colunas: [
                  { chave: "name", rotulo: "Par", render: (p) => h("strong", null, p.name) },
                  { chave: "state", rotulo: "Estado", render: (p) => seloDispositivo(p.state) },
                  { chave: "addr", rotulo: "Endereço", tipo: "mono", render: (p) => p.addr ?? h("span", { class: "muted" }, "sem endereço") },
                  ...(recurso("peer_rtt")
                    ? [
                        { chave: "rtt", rotulo: "RTT", tipo: "num", render: () => "sem medição" },
                        { chave: "p95", rotulo: "Entrada p50 / p95", tipo: "num", render: () => "sem medição" },
                      ]
                    : []),
                ],
                linhas: pares,
                chaveLinha: (p) => p.peer_id,
              }),
              !recurso("peer_rtt") && h("p", { class: "nota" }, "RTT e latência de entrada por computador: ", indisponivel(), ". Essas medições ainda não foram implementadas."),
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
      acoes: h("span", { class: "nota" }, "ocorrências observadas, motivo, par e horários, nunca o conteúdo"),
      plano: blocks.length > 0,
      filhos: [
        blocks.length === 0
          ? h("div", { class: "diagnostico__vazio" }, estadoVazio({ titulo: "Nenhum bloqueio recente", texto: "Quando o agente recusar algo (par sem permissão, malha pausada, chave desconhecida), o motivo aparece aqui." }))
          : tabela({
              legenda: "Últimos bloqueios",
              colunas: [
                { chave: "at", rotulo: "Primeira ocorrência", tipo: "num", render: (b) => formatarMomento(b.at_unix_ms) },
                { chave: "last", rotulo: "Última ocorrência", tipo: "num", render: (b) => formatarMomento(b.last_unix_ms || b.at_unix_ms) },
                { chave: "occurrences", rotulo: "Ocorrências observadas", tipo: "num", render: (b) => {
                  const count = b.occurrences ?? 1;
                  // Acima deste limite o Number pode arredondar para cima: exiba um limite inferior exato.
                  return count > Number.MAX_SAFE_INTEGER
                    ? `pelo menos ${Number.MAX_SAFE_INTEGER.toLocaleString("pt-BR")}`
                    : count.toLocaleString("pt-BR");
                } },
                { chave: "peer", rotulo: "Par", render: (b) => (b.peer_name ? h("strong", null, b.peer_name) : h("span", { class: "muted" }, "sem par")) },
                { chave: "module", rotulo: "Módulo", render: (b) => (b.module ? MODULOS[b.module] ?? b.module : h("span", { class: "muted" }, "geral")) },
                { chave: "reason", rotulo: "Motivo", render: (b) => b.reason },
              ],
              linhas: blocks,
              chaveLinha: (b) => JSON.stringify([b.peer_id ?? null, b.module ?? null, b.reason, b.at_unix_ms]),
            }),
      ],
    });
  }

  async function exportar() {
    const foco = guardarFoco(no);
    mostrarAviso(null);
    let previa;
    try {
      previa = await api.supportBundlePreview();
    } catch (erro) {
      mostrarAviso("erro", mensagemDe(erro));
      return;
    }
    if (!no.isConnected) return;
    if (!interacaoAtual(foco, no)) {
      mostrarAviso("info", "Prévia pronta para consulta. Nada foi gravado.");
      avisoTopo.append(botao({ rotulo: "Ver prévia do pacote", aoClicar: () => mostrarPrevia(previa) }));
      return;
    }
    await mostrarPrevia(previa);
  }

  async function mostrarPrevia(previa) {
    const { promessa } = abrirDialogo({
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
        h("pre", { class: "diagnostico__previa", role: "region", tabIndex: 0, "aria-label": "Prévia do pacote de suporte" }, previa.text),
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
            mostrarAviso("info", `Pacote gravado em ${r.path} (${formatarBytes(r.bytes)}). Anexe esse arquivo ao pedido de ajuda.`);
          },
        },
      ],
    });
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
        botao({ rotulo: "Atualizar", icone: "atualizar", dataset: { focusKey: "diagnostico-atualizar", focusFallback: "" }, aoClicar: () => regiao.recarregar() }),
        botao({ rotulo: "Exportar pacote de suporte", icone: "baixar", dataset: { focusKey: "diagnostico-exportar" }, aoClicar: exportar }),
      ],
    }),
    anuncios.no,
    avisoTopo,
    regiao.no,
  );

  return { no, desmontar: () => { cancelar(); regiao.desmontar(); } };
}
