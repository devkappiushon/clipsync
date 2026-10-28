// Tela 4 · Layout de telas: canvas em SVG com os monitores de cada máquina sobre uma malha de 16 px,
// arrastar encaixa na malha, clicar numa borda seleciona um trecho (alças para encurtar) e o painel pede
// o destino num popover sobre o canvas. Ligações como linhas entre bordas; bordas internas travadas.
// Duas colunas como no quadro: canvas com a barra de ações à esquerda, painel Ligações à direita.
// Toda mudança grava na hora (layout_set_*), então não há botão "Salvar layout".

import { api } from "../api.js";
import { aviso, cabecalho, estadoErro, indisponivel } from "../areas.js";
import { abrirDialogo, botao, cartao, esqueleto, selo } from "../componentes.js";
import { h, novoId, trocar } from "../dom.js";
import { obterApp, recurso } from "../estado.js";
import { INDISPONIVEL, INDISPONIVEL_NESTE_SISTEMA, mensagemDe, seloDispositivo } from "../formatar.js";
import { icone, svg } from "../icones.js";

const MALHA_PX = 16;
const MARGEM = 160; // unidades do plano em volta dos monitores
const ALTURA_MINIMA_PX = 440; // o popover de destino cabe inteiro sobre o canvas
const MARGEM_CIMA_PX = 32; // espaço mínimo, em px, entre o topo do canvas e o primeiro monitor
const LARGURA_LETRA_PX = 7.5; // estimativa para o rótulo do dispositivo caber no canvas
const PASSO_FRACAO = 1 / 16;
const BORDAS = { left: "esquerda", right: "direita", top: "superior", bottom: "inferior" };
const OPOSTA = { left: "right", right: "left", top: "bottom", bottom: "top" };
const EVENTOS_RECARREGAR = ["layout_changed", "peer_connected", "peer_disconnected", "resync"];
const ESTADOS = { connected: "conectado", connecting: "conectando", offline: "offline", blocked: "bloqueado" };
const ERROS_DE_OFERTA = new Set(["no_offer", "stale_offer"]);

const pct = (f) => `${Math.round(f * 100)}%`;
const encaixar = (px) => Math.round(px / MALHA_PX) * MALHA_PX;
const limitar = (v, min, max) => Math.min(max, Math.max(min, v));
const arredondarFracao = (f) => limitar(Math.round(f / PASSO_FRACAO) * PASSO_FRACAO, 0, 1);
const vertical = (edge) => edge === "left" || edge === "right";
const mesmaBorda = (p, q) => p.peer_id === q.peer_id && p.monitor === q.monitor && p.edge === q.edge;
const sobrepoe = (p, q) => mesmaBorda(p, q) && p.from < q.to && q.from < p.to;

/** Nome do monitor para mostrar: o amigável (no Windows, o modelo da tela) ou, sem ele, o `ref`. */
const nomeDoMonitor = (m) => m.name || m.ref;
/** "150%" quando a escala não é 100%; senão nada. */
const escalaDoMonitor = (m) => (m.scale !== 1 ? `${Math.round(m.scale * 100)}%` : null);
const medidasDoMonitor = (m) => [`${m.width}×${m.height}`, escalaDoMonitor(m)].filter(Boolean).join(" · ");
const nomeDoMonitorEm = (layout, peerId, ref) => {
  const m = layout.devices.find((x) => x.peer_id === peerId)?.monitors.find((x) => x.ref === ref);
  return m ? nomeDoMonitor(m) : ref;
};
/** Cabe `texto` em `largura` px de SVG? Senão corta com reticências. */
const caber = (texto, largura) => {
  const max = Math.max(3, Math.floor(largura / LARGURA_LETRA_PX));
  return texto.length <= max ? texto : `${texto.slice(0, max - 1)}…`;
};

function descreverPonta(layout, ponta) {
  const d = layout.devices.find((x) => x.peer_id === ponta.peer_id);
  const nome = d?.name ?? ponta.peer_id.slice(0, 8);
  return `${nome} · ${nomeDoMonitorEm(layout, ponta.peer_id, ponta.monitor)} · borda ${BORDAS[ponta.edge]}${ponta.from > 0 || ponta.to < 1 ? ` · de ${pct(ponta.from)} a ${pct(ponta.to)}` : ""}`;
}

export default function montar({ ir, assinar }) {
  const app = obterApp();
  const corpo = h("div", { class: "layout__corpo" });
  const no = h("div", { class: "layout" });

  if (!recurso("input")) {
    const motivo = app?.input_supported === false ? INDISPONIVEL : "O módulo de teclado e mouse está desligado nesta máquina.";
    no.append(
      cabecalho({ titulo: "Layout de telas", descricao: "Ligue um trecho de borda de um monitor à borda de um monitor de outra máquina." }),
      aviso("aviso", h("p", null, `Layout de telas: ${motivo}`)),
      cartao({ filhos: [h("div", { class: "layout__canvas layout__canvas--inerte", "aria-hidden": "true" }), h("p", { class: "nota" }, "O canvas, as ligações, a trava de tela e os cantos mortos ficam desabilitados até o módulo de teclado e mouse funcionar nesta máquina.")] }),
    );
    return { no };
  }

  let layout = null;
  let selecao = null; // { tipo: "borda", ponta, linkId, destino } | { tipo: "dispositivo", peerId }
  let escala = 1;
  let origem = { x: 0, y: 0 };
  let avisoPainel = null;
  let resultadoTeste = null;
  let idMalha = novoId("layout-malha");
  let focarPopover = false;

  const envoltorio = h("div", { class: "layout__canvas" });
  const painel = h("div", { class: "layout__painel" });
  const popover = h("div", { class: "layout__popover-area" });
  const areaLigacoes = h("div", { class: "layout__area-ligacoes" });
  const areaRodape = h("div");
  const areaOferta = h("div", { class: "layout__oferta" });
  const avisoTopo = h("div");

  let larguraPintada = 0;
  const observador = new ResizeObserver(() => {
    // Só repinta se a largura mudou: a troca do SVG muda a altura e dispararia o observador de novo.
    if (layout && Math.abs(envoltorio.clientWidth - larguraPintada) > 1) pintarCanvas();
  });
  observador.observe(envoltorio);

  async function carregar({ silencioso = false } = {}) {
    if (!silencioso) trocar(corpo, esqueleto({ linhas: 6 }));
    try {
      layout = await api.layoutGet();
    } catch (erro) {
      trocar(corpo, estadoErro(erro, () => carregar()));
      return;
    }
    // Seleção de ligação que já não existe cai.
    if (selecao?.tipo === "borda" && selecao.linkId && !layout.links.some((l) => l.id === selecao.linkId)) selecao = null;
    if (selecao?.tipo === "dispositivo" && !layout.devices.some((d) => d.peer_id === selecao.peerId)) selecao = null;
    try {
      pintarTudo();
    } catch (erro) {
      // Falha ao desenhar nunca fica muda: a tela mostra o erro em vez de um canvas vazio.
      console.error("falha ao pintar o layout", erro);
      trocar(corpo, estadoErro(erro, () => carregar()));
    }
  }

  function pintarTudo() {
    trocar(
      corpo,
      areaOferta,
      h(
        "div",
        { class: "layout__principal" },
        cartao({
          plano: true,
          filhos: [
            h("div", { class: "layout__palco" }, envoltorio, popover),
            h("p", { class: "nota layout__legenda" }, `Malha de ${MALHA_PX} px: ao arrastar, a máquina encaixa no ponto mais próximo. Clique numa borda para ligar um trecho dela a outra máquina.`),
          ],
        }),
        areaRodape,
      ),
      h("div", { class: "layout__lateral" }, painel, areaLigacoes),
    );
    pintarOferta();
    pintarCanvas();
    pintarPainel();
    pintarLigacoes();
    pintarRodape();
  }

  function mostrarAvisoTopo(tom, texto) {
    trocar(avisoTopo, texto && aviso(tom, h("p", null, texto)));
  }

  async function executar(fn, { depois = "recarregar" } = {}) {
    avisoPainel = null;
    try {
      const r = await fn();
      if (depois === "recarregar") await carregar({ silencioso: true });
      return r;
    } catch (erro) {
      if (erro?.code === "stale_rev") {
        mostrarAvisoTopo("aviso", mensagemDe(erro));
        await carregar({ silencioso: true });
        return undefined;
      }
      if (ERROS_DE_OFERTA.has(erro?.code)) {
        // A oferta mudou ou sumiu depois que a tela a mostrou: o texto do erro fica no topo e a oferta é lida de novo.
        mostrarAvisoTopo("aviso", mensagemDe(erro));
        await carregar({ silencioso: true });
        return undefined;
      }
      avisoPainel = { tom: "erro", texto: mensagemDe(erro) };
      pintarPainel();
      return undefined;
    }
  }

  // ----- geometria -----

  function retangulos() {
    const lista = [];
    for (const d of layout.devices) {
      const monitores = d.monitors.length > 0 ? d.monitors : [{ ref: null, name: "sem monitores conhecidos", index: 0, x: 0, y: 0, width: 1920, height: 1080, scale: 1 }];
      for (const m of monitores) lista.push({ d, m, x: d.x + m.x, y: d.y + m.y, w: m.width, h: m.height, fantasma: m.ref === null });
    }
    return lista;
  }

  function calcularEscala(rects) {
    const minX = Math.min(...rects.map((r) => r.x)) - MARGEM;
    const minY = Math.min(...rects.map((r) => r.y)) - MARGEM;
    const maxX = Math.max(...rects.map((r) => r.x + r.w)) + MARGEM;
    const maxY = Math.max(...rects.map((r) => r.y + r.h)) + MARGEM;
    const largura = Math.max(320, envoltorio.clientWidth || 960);
    escala = largura / (maxX - minX);
    // O nome da máquina fica acima do primeiro monitor: com a escala pequena de uma coluna estreita, a margem
    // em unidades do plano vira poucos px e cortaria o rótulo. A escala só depende da largura, então dá para
    // acrescentar altura em cima sem mudá-la.
    const extraCima = Math.max(0, MARGEM_CIMA_PX / escala - MARGEM);
    origem = { x: minX, y: minY - extraCima };
    return { largura, altura: Math.max(ALTURA_MINIMA_PX, Math.round((maxY - minY + extraCima) * escala)) };
  }

  const tela = (vx, vy) => ({ x: (vx - origem.x) * escala, y: (vy - origem.y) * escala });
  const plano = (px, py) => ({ x: px / escala + origem.x, y: py / escala + origem.y });

  /** Ponto na tela da fração f de uma borda do retângulo r (já em px de tela). */
  function pontoNaBorda(r, edge, f) {
    if (edge === "left") return { x: r.x, y: r.y + f * r.h };
    if (edge === "right") return { x: r.x + r.w, y: r.y + f * r.h };
    if (edge === "top") return { x: r.x + f * r.w, y: r.y };
    return { x: r.x + f * r.w, y: r.y + r.h };
  }

  function retanguloDe(ponta, rects) {
    const r = rects.find((x) => x.d.peer_id === ponta.peer_id && x.m.ref === ponta.monitor);
    if (!r) return null;
    const p = tela(r.x, r.y);
    return { x: p.x, y: p.y, w: r.w * escala, h: r.h * escala };
  }

  function trechosLivres(ponta) {
    const ocupados = [
      ...layout.locked_edges.filter((e) => mesmaBorda(e, ponta)),
      ...layout.links.filter((l) => l.id !== selecao?.linkId).flatMap((l) => [l.a, l.b]).filter((p) => mesmaBorda(p, ponta)),
    ].sort((p, q) => p.from - q.from);
    const livres = [];
    let cursor = 0;
    for (const o of ocupados) {
      if (o.from > cursor) livres.push({ from: cursor, to: o.from });
      cursor = Math.max(cursor, o.to);
    }
    if (cursor < 1) livres.push({ from: cursor, to: 1 });
    return livres.filter((t) => t.to - t.from >= PASSO_FRACAO);
  }

  // ----- canvas -----

  function pintarCanvas() {
    const rects = retangulos();
    const { largura, altura } = calcularEscala(rects);
    larguraPintada = envoltorio.clientWidth;
    const desenho = svg("svg", { class: "layout__svg", width: largura, height: altura, role: "group", "aria-label": "Monitores em escala sobre malha de 16 px; arrastar encaixa na malha" });
    desenho.append(
      svg(
        "defs",
        null,
        svg("pattern", { id: idMalha, width: MALHA_PX, height: MALHA_PX, patternUnits: "userSpaceOnUse" }, svg("circle", { class: "layout__ponto", cx: 1, cy: 1, r: 1 })),
      ),
      svg("rect", { class: "layout__fundo", width: largura, height: altura, fill: `url(#${idMalha})` }),
    );

    // Ligações primeiro, para ficarem sob os monitores das pontas? Não: por cima, mais legíveis. Desenha depois dos monitores.
    for (const d of layout.devices) desenho.append(grupoDispositivo(d, rects));
    for (const l of layout.links) desenho.append(ligacao(l, rects));
    if (selecao?.tipo === "borda") desenho.append(selecaoBorda(rects));

    trocar(envoltorio, desenho);
  }

  function grupoDispositivo(d, rects) {
    const meus = rects.filter((r) => r.d === d);
    const ativo = layout.active_peer_id === d.peer_id;
    const grupo = svg("g", {
      class: `layout__dispositivo${d.is_self ? " layout__dispositivo--eu" : ""}${d.state !== "connected" && !d.is_self ? " layout__dispositivo--fora" : ""}${selecao?.tipo === "dispositivo" && selecao.peerId === d.peer_id ? " layout__dispositivo--selecionado" : ""}`,
      tabindex: 0,
      role: "button",
      "aria-label": `${d.name}${d.is_self ? " (esta máquina)" : ""}, ${ESTADOS[d.state] ?? d.state}${d.hotkey ? `, atalho Win+Alt+${d.hotkey}` : ""}. Enter seleciona; setas movem.`,
    });

    const primeiro = tela(Math.min(...meus.map((r) => r.x)), Math.min(...meus.map((r) => r.y)));
    const texto = `${d.name}${d.is_self ? " · esta máquina" : ""}${d.hotkey ? ` · Win+Alt+${d.hotkey}` : ""}${ativo ? " · ativo" : ""}${d.state !== "connected" && !d.is_self ? ` · ${ESTADOS[d.state] ?? d.state}` : ""}`;
    // Rótulo que passaria da borda direita do canvas encosta nela, alinhado à direita, em vez de ser cortado.
    const passa = primeiro.x + texto.length * LARGURA_LETRA_PX > envoltorio.clientWidth;
    const rotulo = svg("text", { class: "layout__nome", x: passa ? envoltorio.clientWidth - 8 : primeiro.x, y: primeiro.y - 8, ...(passa && { "text-anchor": "end" }) });
    rotulo.textContent = texto;
    grupo.append(rotulo);

    for (const r of meus) {
      const p = tela(r.x, r.y);
      const w = r.w * escala;
      const hh = r.h * escala;
      grupo.append(svg("rect", { class: `layout__monitor${r.fantasma ? " layout__monitor--fantasma" : ""}`, x: p.x, y: p.y, width: w, height: hh, rx: 4 }));
      const t1 = svg("text", { class: "layout__texto", x: p.x + 8, y: p.y + 18 });
      t1.textContent = caber(r.fantasma ? r.m.name : nomeDoMonitor(r.m), w - 16);
      grupo.append(t1);
      if (!r.fantasma && w > 72 && hh > 40) {
        const t2 = svg("text", { class: "layout__texto layout__texto--fraco", x: p.x + 8, y: p.y + 34 });
        t2.textContent = caber(medidasDoMonitor(r.m), w - 16);
        grupo.append(t2);
      }
      if (!r.fantasma) for (const edge of Object.keys(BORDAS)) grupo.append(zonaBorda(d, r, { x: p.x, y: p.y, w, h: hh }, edge));
    }

    // Bordas internas travadas
    for (const e of layout.locked_edges.filter((x) => x.peer_id === d.peer_id)) {
      const rr = retanguloDe(e, rects);
      if (!rr) continue;
      const a = pontoNaBorda(rr, e.edge, e.from);
      const b = pontoNaBorda(rr, e.edge, e.to);
      const linha = svg("line", { class: "layout__travada", x1: a.x, y1: a.y, x2: b.x, y2: b.y });
      const titulo = svg("title");
      titulo.textContent = "Borda entre monitores desta máquina: não cruza para outra.";
      linha.append(titulo);
      grupo.append(linha);
    }

    ligarArrasto(grupo, d);
    grupo.addEventListener("keydown", (evento) => {
      if (evento.target !== grupo) return;
      const passo = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[evento.key];
      if (passo) {
        evento.preventDefault();
        const dx = (passo[0] * MALHA_PX) / escala;
        const dy = (passo[1] * MALHA_PX) / escala;
        executar(() => api.layoutSetDevice({ peer_id: d.peer_id, x: Math.round(d.x + dx), y: Math.round(d.y + dy), hotkey: d.hotkey, base_rev: layout.rev }));
      } else if (evento.key === "Enter" || evento.key === " ") {
        evento.preventDefault();
        selecionarDispositivo(d);
      }
    });
    return grupo;
  }

  function ligarArrasto(grupo, d) {
    grupo.addEventListener("pointerdown", (evento) => {
      if (evento.button !== 0) return;
      if (evento.target.closest(".layout__borda, .layout__alca, .layout__segmento, .layout__travada")) return;
      evento.preventDefault();
      const inicio = { x: evento.clientX, y: evento.clientY };
      let dx = 0;
      let dy = 0;
      let moveu = false;
      grupo.setPointerCapture(evento.pointerId);
      grupo.classList.add("layout__dispositivo--arrastando");
      const mover = (e) => {
        dx = encaixar(e.clientX - inicio.x);
        dy = encaixar(e.clientY - inicio.y);
        if (dx !== 0 || dy !== 0) moveu = true;
        grupo.setAttribute("transform", `translate(${dx} ${dy})`);
      };
      const soltar = () => {
        grupo.removeEventListener("pointermove", mover);
        grupo.removeEventListener("pointerup", soltar);
        grupo.removeEventListener("pointercancel", soltar);
        grupo.classList.remove("layout__dispositivo--arrastando");
        if (!moveu) {
          selecionarDispositivo(d);
          return;
        }
        executar(() => api.layoutSetDevice({ peer_id: d.peer_id, x: Math.round(d.x + dx / escala), y: Math.round(d.y + dy / escala), hotkey: d.hotkey, base_rev: layout.rev }));
      };
      grupo.addEventListener("pointermove", mover);
      grupo.addEventListener("pointerup", soltar);
      grupo.addEventListener("pointercancel", soltar);
    });
  }

  function zonaBorda(d, r, rr, edge) {
    const espessura = 10;
    const geo = vertical(edge)
      ? { x: (edge === "left" ? rr.x : rr.x + rr.w) - espessura / 2, y: rr.y, width: espessura, height: rr.h }
      : { x: rr.x, y: (edge === "top" ? rr.y : rr.y + rr.h) - espessura / 2, width: rr.w, height: espessura };
    const ponta = { peer_id: d.peer_id, monitor: r.m.ref, edge, from: 0, to: 1 };
    const selecionada = selecao?.tipo === "borda" && mesmaBorda(selecao.ponta, ponta);
    const zona = svg("rect", {
      class: `layout__borda${selecionada ? " layout__borda--selecionada" : ""}`,
      ...geo,
      tabindex: 0,
      role: "button",
      "aria-label": `${d.name}, ${nomeDoMonitor(r.m)}, borda ${BORDAS[edge]}. Enter seleciona um trecho para ligar.`,
    });
    const escolher = (fracaoClicada) => {
      const ligacaoExistente = layout.links.find((l) => [l.a, l.b].some((p) => mesmaBorda(p, ponta) && fracaoClicada >= p.from && fracaoClicada <= p.to));
      if (ligacaoExistente) {
        selecionarLigacao(ligacaoExistente, ponta);
        return;
      }
      const travada = layout.locked_edges.find((e) => mesmaBorda(e, ponta) && fracaoClicada >= e.from && fracaoClicada <= e.to);
      const livres = trechosLivres(ponta);
      if (livres.length === 0 || (travada && !livres.some((t) => fracaoClicada >= t.from && fracaoClicada <= t.to))) {
        avisoPainel = { tom: "aviso", texto: travada ? "Essa borda fica entre dois monitores da mesma máquina e não cruza para outra. Escolha uma borda de fora." : "Essa borda já está toda ligada. Remova ou encurte uma ligação para liberar um trecho." };
        selecao = null;
        pintarCanvas();
        pintarPainel();
        return;
      }
      const trecho = livres.find((t) => fracaoClicada >= t.from && fracaoClicada <= t.to) ?? livres.reduce((a, b) => (b.to - b.from > a.to - a.from ? b : a));
      selecao = { tipo: "borda", ponta: { ...ponta, from: trecho.from, to: trecho.to }, linkId: null, destino: destinoPadrao(ponta) };
      avisoPainel = null;
      resultadoTeste = null;
      focarPopover = true;
      pintarCanvas();
      pintarPainel();
      pintarLigacoes();
    };
    zona.addEventListener("click", (evento) => {
      const caixa = zona.getBoundingClientRect();
      const f = vertical(edge) ? (evento.clientY - caixa.top) / caixa.height : (evento.clientX - caixa.left) / caixa.width;
      escolher(limitar(f, 0, 1));
    });
    zona.addEventListener("keydown", (evento) => {
      if (evento.key === "Enter" || evento.key === " ") {
        evento.preventDefault();
        escolher(0.5);
      }
    });
    return zona;
  }

  function destinoPadrao(ponta) {
    const opcoes = opcoesDestino(ponta);
    const oposta = opcoes.find((o) => o.edge === OPOSTA[ponta.edge]);
    const escolhida = oposta ?? opcoes[0];
    return escolhida ? { ...escolhida, from: 0, to: 1 } : null;
  }

  function opcoesDestino(ponta) {
    return layout.devices
      .filter((d) => d.peer_id !== ponta.peer_id)
      .flatMap((d) => d.monitors.flatMap((m) => Object.keys(BORDAS).map((edge) => ({ peer_id: d.peer_id, monitor: m.ref, rotuloMonitor: nomeDoMonitor(m), edge, nome: d.name }))))
      .filter((o) => !layout.locked_edges.some((e) => mesmaBorda(e, o) && e.from <= 0 && e.to >= 1));
  }

  function ligacao(l, rects) {
    const ra = retanguloDe(l.a, rects);
    const rb = retanguloDe(l.b, rects);
    if (!ra || !rb) return svg("g");
    const a1 = pontoNaBorda(ra, l.a.edge, l.a.from);
    const a2 = pontoNaBorda(ra, l.a.edge, l.a.to);
    const b1 = pontoNaBorda(rb, l.b.edge, l.b.from);
    const b2 = pontoNaBorda(rb, l.b.edge, l.b.to);
    const ma = { x: (a1.x + a2.x) / 2, y: (a1.y + a2.y) / 2 };
    const mb = { x: (b1.x + b2.x) / 2, y: (b1.y + b2.y) / 2 };
    const ativa = selecao?.tipo === "borda" && selecao.linkId === l.id;
    const g = svg("g", { class: `layout__ligacao${l.problem ? " layout__ligacao--problema" : ""}${ativa ? " layout__ligacao--ativa" : ""}` });
    g.append(
      svg("line", { class: "layout__segmento", x1: a1.x, y1: a1.y, x2: a2.x, y2: a2.y }),
      svg("line", { class: "layout__segmento", x1: b1.x, y1: b1.y, x2: b2.x, y2: b2.y }),
      svg("line", { class: "layout__fio", x1: ma.x, y1: ma.y, x2: mb.x, y2: mb.y }),
      svg("circle", { class: "layout__no", cx: ma.x, cy: ma.y, r: 4 }),
      svg("circle", { class: "layout__no", cx: mb.x, cy: mb.y, r: 4 }),
    );
    const titulo = svg("title");
    titulo.textContent = `${descreverPonta(layout, l.a)} para ${descreverPonta(layout, l.b)}`;
    g.append(titulo);
    g.addEventListener("click", () => selecionarLigacao(l, l.a));
    return g;
  }

  function selecaoBorda(rects) {
    const ponta = selecao.ponta;
    const rr = retanguloDe(ponta, rects);
    if (!rr) return svg("g");
    const g = svg("g", { class: "layout__selecao" });
    const p1 = pontoNaBorda(rr, ponta.edge, ponta.from);
    const p2 = pontoNaBorda(rr, ponta.edge, ponta.to);
    const trecho = svg("line", { class: "layout__segmento layout__segmento--selecionado", x1: p1.x, y1: p1.y, x2: p2.x, y2: p2.y });
    const alcaDe = alca(rr, ponta, "from", p1);
    const alcaAte = alca(rr, ponta, "to", p2);
    g.append(trecho, alcaDe, alcaAte);
    g.atualizar = () => {
      const q1 = pontoNaBorda(rr, ponta.edge, ponta.from);
      const q2 = pontoNaBorda(rr, ponta.edge, ponta.to);
      trecho.setAttribute("x1", q1.x);
      trecho.setAttribute("y1", q1.y);
      trecho.setAttribute("x2", q2.x);
      trecho.setAttribute("y2", q2.y);
      alcaDe.setAttribute("cx", q1.x);
      alcaDe.setAttribute("cy", q1.y);
      alcaAte.setAttribute("cx", q2.x);
      alcaAte.setAttribute("cy", q2.y);
    };
    selecaoAtual = g;
    return g;
  }

  let selecaoAtual = null;

  function alca(rr, ponta, lado, p) {
    const circulo = svg("circle", {
      class: "layout__alca",
      cx: p.x,
      cy: p.y,
      r: 7,
      tabindex: 0,
      role: "slider",
      "aria-label": lado === "from" ? "Começo do trecho" : "Fim do trecho",
      "aria-valuemin": 0,
      "aria-valuemax": 100,
      "aria-valuenow": Math.round(ponta[lado] * 100),
      "aria-valuetext": pct(ponta[lado]),
    });
    const aplicar = (f) => {
      const novo = arredondarFracao(f);
      if (lado === "from") ponta.from = Math.min(novo, ponta.to - PASSO_FRACAO);
      else ponta.to = Math.max(novo, ponta.from + PASSO_FRACAO);
      circulo.setAttribute("aria-valuenow", Math.round(ponta[lado] * 100));
      circulo.setAttribute("aria-valuetext", pct(ponta[lado]));
      selecaoAtual?.atualizar();
      pintarPainel();
    };
    circulo.addEventListener("pointerdown", (evento) => {
      evento.preventDefault();
      evento.stopPropagation();
      circulo.setPointerCapture(evento.pointerId);
      const caixaSvg = envoltorio.querySelector("svg").getBoundingClientRect();
      const mover = (e) => {
        const px = e.clientX - caixaSvg.left;
        const py = e.clientY - caixaSvg.top;
        const f = vertical(ponta.edge) ? (py - rr.y) / rr.h : (px - rr.x) / rr.w;
        aplicar(limitar(f, 0, 1));
      };
      const soltar = () => {
        circulo.removeEventListener("pointermove", mover);
        circulo.removeEventListener("pointerup", soltar);
        circulo.removeEventListener("pointercancel", soltar);
      };
      circulo.addEventListener("pointermove", mover);
      circulo.addEventListener("pointerup", soltar);
      circulo.addEventListener("pointercancel", soltar);
    });
    circulo.addEventListener("click", (evento) => evento.stopPropagation());
    circulo.addEventListener("keydown", (evento) => {
      const delta = { ArrowLeft: -1, ArrowDown: -1, ArrowRight: 1, ArrowUp: 1 }[evento.key];
      if (!delta) return;
      evento.preventDefault();
      aplicar(ponta[lado] + delta * PASSO_FRACAO);
    });
    return circulo;
  }

  function selecionarLigacao(l, pontaClicada) {
    const minha = mesmaBorda(l.a, pontaClicada) ? "a" : "b";
    const outra = minha === "a" ? "b" : "a";
    const outroDispositivo = layout.devices.find((d) => d.peer_id === l[outra].peer_id);
    selecao = { tipo: "borda", ponta: { ...l[minha] }, linkId: l.id, destino: { ...l[outra], nome: outroDispositivo?.name ?? "" } };
    avisoPainel = null;
    resultadoTeste = null;
    focarPopover = true;
    pintarCanvas();
    pintarPainel();
    pintarLigacoes();
  }

  function selecionarDispositivo(d) {
    selecao = { tipo: "dispositivo", peerId: d.peer_id };
    avisoPainel = null;
    resultadoTeste = null;
    pintarCanvas();
    pintarPainel();
    pintarLigacoes();
  }

  function limparSelecao() {
    selecao = null;
    avisoPainel = null;
    pintarCanvas();
    pintarPainel();
    pintarLigacoes();
  }

  // ----- painel -----

  function pintarPainel() {
    const naBorda = selecao?.tipo === "borda";
    trocar(popover, naBorda && painelBorda());
    trocar(painel, !naBorda && (selecao ? painelDispositivo() : avisoPainel && aviso(avisoPainel.tom, h("p", null, avisoPainel.texto))));
    if (focarPopover) {
      focarPopover = false;
      popover.querySelector("select:not(:disabled), button")?.focus();
    }
  }

  /** Quadrante do canvas onde a borda escolhida está: o popover vai para o quadrante oposto e não a cobre. */
  function classesDoLado(ponta) {
    const rr = retanguloDe(ponta, retangulos());
    const c = rr ? pontoNaBorda(rr, ponta.edge, (ponta.from + ponta.to) / 2) : { x: 0, y: 0 };
    const horizontal = c.x > envoltorio.clientWidth / 2 ? "esq" : "dir";
    const alto = c.y > envoltorio.clientHeight / 2 ? "cima" : "baixo";
    return `layout__popover--${horizontal} layout__popover--${alto}`;
  }

  /** "Destino da borda": popover sobre o canvas. Dispositivo e borda de destino em dois campos, como no quadro. */
  function painelBorda() {
    const { ponta, linkId } = selecao;
    const opcoes = opcoesDestino(ponta);
    const chave = (o) => `${o.peer_id}|${o.monitor}|${o.edge}`;
    const idTitulo = novoId("layout-popover");
    const idDispositivo = novoId("layout-dispositivo");
    const idBordaDestino = novoId("layout-destino");
    const idDe = novoId("layout-de");
    const idAte = novoId("layout-ate");
    const dispositivos = [...new Map(opcoes.map((o) => [o.peer_id, o.nome])).entries()];

    const seletorDispositivo = h(
      "select",
      { class: "entrada", id: idDispositivo, disabled: dispositivos.length === 0 },
      dispositivos.length === 0 && h("option", { value: "" }, "Nenhuma outra máquina com monitores conhecidos"),
      dispositivos.map(([peerId, nome]) => h("option", { value: peerId, selected: selecao.destino?.peer_id === peerId }, nome)),
    );
    const seletorBorda = h("select", { class: "entrada", id: idBordaDestino, disabled: dispositivos.length === 0 });
    const entradaDe = h("input", { class: "entrada entrada--mono layout__pct", id: idDe, type: "number", min: 0, max: 100, step: 5, value: Math.round((selecao.destino?.from ?? 0) * 100), disabled: !selecao.destino });
    const entradaAte = h("input", { class: "entrada entrada--mono layout__pct", id: idAte, type: "number", min: 0, max: 100, step: 5, value: Math.round((selecao.destino?.to ?? 1) * 100), disabled: !selecao.destino });

    /** O que o campo mostra é o que a gravação manda: se o destino antigo não está mais nas opções, vale a opção mostrada. */
    const sincronizar = () => {
      const o = opcoes.find((x) => chave(x) === seletorBorda.value);
      if (o) selecao.destino = { ...o, from: selecao.destino?.from ?? 0, to: selecao.destino?.to ?? 1 };
    };
    const preencherBordas = () => {
      const daMaquina = opcoes.filter((o) => o.peer_id === seletorDispositivo.value);
      const atual = selecao.destino;
      const preferida = daMaquina.find((o) => atual && chave(o) === chave(atual)) ?? daMaquina.find((o) => o.edge === OPOSTA[ponta.edge]) ?? daMaquina[0];
      trocar(seletorBorda, daMaquina.map((o) => h("option", { value: chave(o), selected: preferida && chave(o) === chave(preferida) }, `${o.rotuloMonitor} · ${BORDAS[o.edge]}`)));
      sincronizar();
    };
    seletorDispositivo.addEventListener("change", preencherBordas);
    seletorBorda.addEventListener("change", sincronizar);
    preencherBordas();

    const lerDestino = () => {
      if (!selecao.destino) return null;
      return { peer_id: selecao.destino.peer_id, monitor: selecao.destino.monitor, edge: selecao.destino.edge, from: limitar(Number(entradaDe.value) / 100, 0, 1), to: limitar(Number(entradaAte.value) / 100, 0, 1) };
    };

    const acoes = h(
      "div",
      { class: "linha layout__popover-acoes" },
      botao({
        rotulo: linkId ? "Salvar ligação" : "Ligar",
        variante: "primario",
        icone: "verificar",
        desabilitado: !selecao.destino || !recurso("layout_links"),
        aoClicar: async () => {
          const b = lerDestino();
          if (!b) return;
          const r = await executar(() => api.layoutSetLink({ id: linkId, a: { peer_id: ponta.peer_id, monitor: ponta.monitor, edge: ponta.edge, from: ponta.from, to: ponta.to }, b, base_rev: layout.rev }), { depois: "nada" });
          if (!r) return;
          if (r.warnings?.length > 0) mostrarAvisoTopo("info", "A ligação foi gravada. O trecho foi encurtado nas pontas pelos cantos mortos.");
          selecao = null;
          await carregar({ silencioso: true });
        },
      }),
      linkId &&
        botao({
          rotulo: "Remover",
          variante: "perigo",
          icone: "descartar",
          aoClicar: async () => {
            const r = await executar(() => api.layoutRemoveLink({ id: linkId, base_rev: layout.rev }), { depois: "nada" });
            if (!r) return;
            selecao = null;
            await carregar({ silencioso: true });
          },
        }),
      botao({ rotulo: "Cancelar", variante: "discreto", icone: "fechar", aoClicar: limparSelecao }),
    );

    const no = h(
      "section",
      { class: `layout__popover ${classesDoLado(ponta)}`, role: "dialog", "aria-modal": "false", "aria-labelledby": idTitulo },
      h("h2", { id: idTitulo }, linkId ? `Destino da borda (ligação ${linkId})` : "Destino da borda"),
      avisoPainel && aviso(avisoPainel.tom, h("p", null, avisoPainel.texto)),
      !recurso("layout_links") && h("p", { class: "nota" }, `Ligações: ${INDISPONIVEL}.`),
      h("div", { class: "layout__campo" }, h("span", { class: "campo__rotulo" }, "Borda"), h("p", { class: "mono layout__borda-texto" }, descreverPonta(layout, ponta)), h("p", { class: "nota" }, "Arraste as alças no canvas para encurtar.")),
      h("div", { class: "layout__campo" }, h("label", { class: "campo__rotulo", for: idDispositivo }, "Dispositivo"), seletorDispositivo),
      h("div", { class: "layout__campo" }, h("label", { class: "campo__rotulo", for: idBordaDestino }, "Borda de destino"), seletorBorda),
      h(
        "div",
        { class: "layout__campo" },
        h("span", { class: "campo__rotulo" }, "Trecho de destino"),
        h("div", { class: "layout__trecho" }, h("label", { for: idDe }, "de"), entradaDe, h("span", null, "%"), h("label", { for: idAte }, "a"), entradaAte, h("span", null, "%")),
      ),
      acoes,
    );
    no.addEventListener("keydown", (evento) => {
      if (evento.key === "Escape") {
        evento.preventDefault();
        limparSelecao();
      }
    });
    return no;
  }

  function painelDispositivo() {
    const d = layout.devices.find((x) => x.peer_id === selecao.peerId);
    if (!d) return null;
    const idAtalho = novoId("layout-atalho");
    const idIrNota = novoId("layout-ir-nota");
    const atalho = h(
      "select",
      { class: "entrada layout__atalho", id: idAtalho, disabled: !recurso("switch_device") },
      h("option", { value: "", selected: d.hotkey === null }, "sem atalho"),
      Array.from({ length: 9 }, (_, i) => i + 1).map((n) => h("option", { value: String(n), selected: d.hotkey === n }, `Win+Alt+${n}`)),
    );
    atalho.addEventListener("change", () => {
      const hotkey = atalho.value === "" ? null : Number(atalho.value);
      executar(() => api.layoutSetDevice({ peer_id: d.peer_id, x: d.x, y: d.y, hotkey, base_rev: layout.rev }));
    });
    const fonte = { live: "monitores lidos agora", cached: "monitores da última conexão", none: "sem monitores conhecidos" }[d.monitors_source] ?? d.monitors_source;
    const areaTeste = h("div", { class: "pilha" });
    if (resultadoTeste) {
      const s = resultadoTeste;
      trocar(
        areaTeste,
        h(
          "dl",
          { class: "kv layout__estatisticas" },
          h("dt", null, "Recebidos"),
          h("dd", { class: "mono" }, String(s.received)),
          h("dt", null, "Perdidos"),
          h("dd", { class: "mono" }, String(s.lost)),
          h("dt", null, "Fora de ordem"),
          h("dd", { class: "mono" }, String(s.out_of_order)),
          h("dt", null, "Latência p50 / p95 / máx"),
          h("dd", { class: "mono" }, `${(s.p50_us / 1000).toFixed(2)} ms / ${(s.p95_us / 1000).toFixed(2)} ms / ${(s.max_us / 1000).toFixed(2)} ms`),
        ),
      );
    }

    return cartao({
      titulo: d.name,
      acoes: h("div", { class: "linha" }, d.is_self && selo("acento", "esta máquina"), seloDispositivo(d.state), layout.active_peer_id === d.peer_id && selo("ok", "ativo")),
      filhos: [
        avisoPainel && aviso(avisoPainel.tom, h("p", null, avisoPainel.texto)),
        h(
          "dl",
          { class: "kv" },
          h("dt", null, "Posição no plano"),
          h("dd", { class: "mono" }, `${d.x}, ${d.y}`),
          h("dt", null, "Monitores"),
          h(
            "dd",
            null,
            d.monitors.length > 0 ? `${d.monitors.length} (${fonte})` : fonte,
            d.monitors.length > 0 &&
              h(
                "ul",
                { class: "layout__monitores" },
                d.monitors.map((m) => h("li", null, h("strong", null, nomeDoMonitor(m)), ` · ${medidasDoMonitor(m)}`)),
              ),
          ),
          h("dt", null, h("label", { for: idAtalho }, "Atalho para ir até ele")),
          h("dd", null, h("div", { class: "linha" }, atalho, !recurso("switch_device") && indisponivel(INDISPONIVEL_NESTE_SISTEMA))),
        ),
        !d.is_self &&
          h(
            "div",
            { class: "linha" },
            botao({
              rotulo: "Ir para este dispositivo",
              icone: "avancar",
              desabilitado: !recurso("switch_device"),
              "aria-describedby": recurso("switch_device") ? null : idIrNota,
              aoClicar: () => executar(() => api.switchToDevice({ peer_id: d.peer_id })),
            }),
            !recurso("switch_device") && h("span", { class: "nota", id: idIrNota }, INDISPONIVEL_NESTE_SISTEMA),
            botao({
              rotulo: "Testar entrada (5 s)",
              icone: "teclado",
              aoClicar: async () => {
                resultadoTeste = null;
                const r = await executar(() => api.inputTest({ peer_id: d.peer_id, seconds: 5 }), { depois: "nada" });
                if (!r) return;
                resultadoTeste = r.stats;
                pintarPainel();
              },
            }),
            botao({ rotulo: "Cancelar seleção", variante: "discreto", icone: "fechar", aoClicar: limparSelecao }),
          ),
        d.is_self && h("div", { class: "linha" }, botao({ rotulo: "Cancelar seleção", variante: "discreto", icone: "fechar", aoClicar: limparSelecao })),
        areaTeste,
      ],
    });
  }

  // ----- lista de ligações -----

  function atalhoDaLigacao(l) {
    const outra = [l.a, l.b].map((p) => layout.devices.find((d) => d.peer_id === p.peer_id)).find((d) => d && !d.is_self);
    return outra?.hotkey ? `Win+Alt+${outra.hotkey}` : null;
  }

  function pintarLigacoes() {
    const vazio = h("div", { class: "estado" }, h("h2", null, "Nenhuma ligação"), h("p", { class: "muted" }, layout.devices.length > 1 ? "Clique numa borda de monitor no canvas e escolha o destino em outra máquina." : "Pareie outra máquina com teclado e mouse em Dispositivos para ligar bordas."));
    const total = layout.links.length;
    const itens = layout.links.map((l) => {
      const selecionada = selecao?.tipo === "borda" && selecao.linkId === l.id;
      const atalho = atalhoDaLigacao(l);
      const corpoLigacao = h(
        "button",
        { class: "layout__lig-corpo", type: "button", "aria-pressed": String(selecionada) },
        h("span", { class: "layout__lig-de mono" }, descreverPonta(layout, l.a)),
        h("span", { class: "layout__lig-para" }, `para ${descreverPonta(layout, l.b)}`),
      );
      corpoLigacao.addEventListener("click", () => selecionarLigacao(l, l.a));
      return h(
        "li",
        { class: `layout__lig${selecionada ? " layout__lig--selecionada" : ""}` },
        corpoLigacao,
        h(
          "div",
          { class: "layout__lig-pe" },
          h("div", { class: "linha" }, estadoDaLigacao(l), atalho ? h("kbd", null, atalho) : h("span", { class: "nota" }, "sem atalho")),
          botao({
            rotulo: "Remover",
            variante: "discreto",
            icone: "descartar",
            "aria-label": `Remover a ligação ${l.id}`,
            aoClicar: async () => {
              const r = await executar(() => api.layoutRemoveLink({ id: l.id, base_rev: layout.rev }), { depois: "nada" });
              if (!r) return;
              if (selecao?.linkId === l.id) selecao = null;
              await carregar({ silencioso: true });
            },
          }),
        ),
      );
    });
    trocar(
      areaLigacoes,
      cartao({
        titulo: "Ligações",
        acoes: total > 0 && selo("acento", `${total} ${total === 1 ? "configurada" : "configuradas"}`),
        filhos:
          total > 0
            ? [
                h("ul", { class: "layout__ligacoes", "aria-label": "Ligações entre bordas" }, itens),
                h("p", { class: "nota" }, "O atalho pula para o dispositivo sem passar pela borda. Arraste as alças para encurtar o trecho da borda que atravessa."),
                h("p", { class: "nota mono" }, `rev ${layout.rev}`),
              ]
            : [vazio],
      }),
    );
  }

  /** Estado da ligação: erro do layout, par fora do ar ou ativa. */
  function estadoDaLigacao(l) {
    if (l.problem === "internal_edge_locked") return selo("erro", "cruza borda interna");
    if (l.problem) return selo("erro", l.problem);
    const fora = [l.a, l.b].map((p) => layout.devices.find((d) => d.peer_id === p.peer_id)).find((d) => d && !d.is_self && d.state !== "connected");
    if (!fora) return selo("ok", "Ativa");
    if (fora.state === "connecting") return selo("acento", "Par conectando");
    if (fora.state === "blocked") return selo("aviso", "Par bloqueado");
    return selo("neutro", "Par offline");
  }

  // ----- oferta pendente -----

  /** A resposta diz qual oferta o usuário viu (par e rev), para não aceitar uma que chegou depois. */
  function respostaOferta(oferta, accept) {
    return { accept, from_peer_id: oferta.from_peer_id, rev: oferta.rev };
  }

  function pintarOferta() {
    const o = layout.pending_offer;
    if (!o) {
      trocar(areaOferta, avisoTopo);
      return;
    }
    trocar(
      areaOferta,
      avisoTopo,
      aviso(
        "info",
        h("p", null, `${o.from_name} ofereceu um layout (rev ${o.rev}) com ${o.links.length} ${o.links.length === 1 ? "ligação" : "ligações"}. Aceitar substitui as ligações que cruzam as mesmas bordas.`),
        botao({ rotulo: "Aceitar layout", variante: "primario", icone: "verificar", aoClicar: () => executar(() => api.layoutOfferAnswer(respostaOferta(o, true))) }),
        botao({ rotulo: "Recusar", icone: "fechar", aoClicar: () => executar(() => api.layoutOfferAnswer(respostaOferta(o, false))) }),
      ),
    );
  }

  // ----- barra de ações (sob o canvas) -----

  function pintarRodape() {
    const travar = botao({
      rotulo: layout.screen_locked ? "Destravar esta tela" : "Travar nesta tela",
      atalho: "Win+Alt+L",
      desabilitado: !recurso("screen_lock"),
      "aria-pressed": String(layout.screen_locked),
      aoClicar: () => executar(() => api.lockToScreen({ locked: !layout.screen_locked })),
    });

    // Cantos mortos: o botão abre um popover com o valor em px; o botão só alterna `hidden`, sem repintar.
    const idCantos = novoId("layout-cantos");
    const idCantosPopover = novoId("layout-cantos-popover");
    const cantos = h("input", { class: "entrada entrada--mono layout__pct", id: idCantos, type: "number", min: 0, max: 32, step: 1, value: layout.options.dead_corner_px, disabled: !recurso("dead_corners") });
    const aplicarCantos = botao({
      rotulo: "Aplicar",
      variante: "primario",
      desabilitado: !recurso("dead_corners"),
      aoClicar: () => executar(() => api.layoutSetOptions({ dead_corner_px: Number(cantos.value), base_rev: layout.rev })),
    });
    const popoverCantos = h(
      "div",
      { class: "layout__cantos", id: idCantosPopover, role: "group", "aria-label": "Cantos mortos", hidden: true },
      h("label", { class: "campo__rotulo", for: idCantos }, "Cantos mortos"),
      h("div", { class: "linha" }, cantos, h("span", null, "px"), aplicarCantos),
      h("p", { class: "nota" }, "Trecho em cada canto onde o cursor não cruza, para o canto do monitor continuar servindo ao sistema."),
    );
    const abrirCantos = h(
      "button",
      { class: "btn btn--secundario", type: "button", "aria-expanded": "false", "aria-controls": idCantosPopover, disabled: !recurso("dead_corners") },
      h("span", { class: "btn__rotulo" }, "Cantos mortos"),
    );
    const alternarCantos = (aberto) => {
      popoverCantos.hidden = !aberto;
      abrirCantos.setAttribute("aria-expanded", String(aberto));
      if (aberto) cantos.focus();
    };
    abrirCantos.addEventListener("click", () => alternarCantos(popoverCantos.hidden));
    popoverCantos.addEventListener("keydown", (evento) => {
      if (evento.key !== "Escape") return;
      evento.preventDefault();
      alternarCantos(false);
      abrirCantos.focus();
    });

    const outros = layout.devices.filter((d) => !d.is_self);
    const irPara = h(
      "div",
      { class: "linha" },
      outros.length === 0 && h("span", { class: "muted" }, "nenhum outro dispositivo"),
      outros.map((d) =>
        botao({
          rotulo: d.name,
          icone: "avancar",
          atalho: d.hotkey ? `Win+Alt+${d.hotkey}` : null,
          desabilitado: !recurso("switch_device") || d.state !== "connected",
          "aria-pressed": String(layout.active_peer_id === d.peer_id),
          aoClicar: () => executar(() => api.switchToDevice({ peer_id: d.peer_id })),
        }),
      ),
    );

    trocar(
      areaRodape,
      h(
        "div",
        { class: "layout__rodape" },
        h(
          "div",
          { class: "layout__barra" },
          h("div", { class: "layout__acao" }, travar, !recurso("screen_lock") && indisponivel()),
          h("div", { class: "layout__acao layout__acao--cantos" }, abrirCantos, popoverCantos, !recurso("dead_corners") && indisponivel()),
          h(
            "div",
            { class: "layout__acao" },
            botao({ rotulo: "Copiar layout para os pares", desabilitado: !recurso("layout_copy"), aoClicar: copiarParaPares }),
            !recurso("layout_copy") && indisponivel(),
          ),
        ),
        layout.screen_locked && h("p", { class: "nota" }, "Com a tela travada, o cursor não sai desta máquina."),
        h("div", { class: "layout__ir" }, h("span", { class: "campo__rotulo" }, "Ir para dispositivo"), irPara, !recurso("switch_device") && indisponivel(INDISPONIVEL_NESTE_SISTEMA)),
      ),
    );
  }

  async function copiarParaPares() {
    let r;
    try {
      r = await api.layoutCopyToPeers({ peer_ids: null });
    } catch (erro) {
      mostrarAvisoTopo("erro", mensagemDe(erro));
      pintarOferta();
      return;
    }
    const ESTADOS = { offered: ["ok", "oferecido"], offline: ["aviso", "offline"], denied: ["erro", "recusado"], unsupported: ["neutro", "sem suporte"] };
    const { promessa } = abrirDialogo({
      titulo: "Layout oferecido aos pares",
      conteudo: h(
        "div",
        { class: "pilha" },
        h("p", { class: "muted" }, "Cada par decide se aceita. O layout dele só muda depois do aceite."),
        r.results.length === 0
          ? h("p", null, "Nenhum par para receber o layout.")
          : h(
              "ul",
              { class: "layout__resultados" },
              r.results.map((x) => {
                const [tom, texto] = ESTADOS[x.status] ?? ["neutro", x.status];
                return h("li", null, h("strong", null, x.name), " ", selo(tom, texto), x.reason && h("span", { class: "muted" }, ` ${x.reason}`));
              }),
            ),
      ),
      acoes: [{ rotulo: "Fechar", valor: "fechar", variante: "primario", foco: true }],
    });
    await promessa;
  }

  const cancelarEventos = [
    assinar(EVENTOS_RECARREGAR, () => carregar({ silencioso: true })),
    assinar("screen_lock_changed", (data) => {
      if (!layout) return;
      layout.screen_locked = data.screen_locked;
      layout.active_peer_id = data.active_peer_id ?? null;
      pintarCanvas();
      pintarRodape();
      if (selecao?.tipo === "dispositivo") pintarPainel();
    }),
  ];

  no.append(
    cabecalho({
      titulo: "Layout de telas",
      descricao: "Ligue um trecho de borda de um monitor à borda de um monitor de outra máquina. Bordas entre monitores da mesma máquina não cruzam.",
    }),
    corpo,
  );
  carregar();

  return {
    no,
    desmontar: () => {
      observador.disconnect();
      for (const fn of cancelarEventos) fn();
    },
  };
}
