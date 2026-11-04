// Layout: rascunho por monitor, prévia do agente e aplicação explícita. Bordas no avançado.

import { api } from "../api.js";
import { aviso, cabecalho, estadoErro, indisponivel } from "../areas.js";
import { abrirDialogo, botao, campo, cartao, esqueleto, selo } from "../componentes.js";
import { criarAnunciador, guardarFoco, interacaoAtual, restaurarFoco } from "../acessibilidade.js";
import { h, novoId, trocar } from "../dom.js";
import { atalhoGlobal, consumirParDoLayout, obterApp, observarApp, recurso } from "../estado.js";
import { INDISPONIVEL, INDISPONIVEL_NESTE_SISTEMA, mensagemDe, seloDispositivo } from "../formatar.js";
import { icone, svg } from "../icones.js";

import { controladorMonitores, editorMonitores, ofertaMonitores, recuperacaoMonitores } from "../monitores.js";
import { resumoTravessia } from "../travessia-vista.js";
import { falhaAoLigar, limitesAlca, ligacoesLocais, proporLigacao, posicionarDestino, posicaoRelativa, resultadoDoTeste, trechosLivres as trechosDisponiveis } from "../travessia.js";

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
  const rotaMontagem = no.ownerDocument.defaultView?.location.href;

  if (!recurso("input")) {
    const motivo = app?.input_supported === false ? INDISPONIVEL : app?.modules?.input === false ? "O módulo de teclado e mouse está desligado nesta máquina." : "O agente ainda não informou os recursos de teclado e mouse.";
    no.append(
      cabecalho({ titulo: "Layout de telas", descricao: "Organize cada tela, confira a prévia e aplique as passagens entre os computadores." }),
      resumoTravessia(app, { ir }),
      aviso("aviso", h("p", null, `Layout de telas: ${motivo}`)),
      cartao({ filhos: [h("div", { class: "layout__canvas layout__canvas--inerte", "aria-hidden": "true" }), h("p", { class: "nota" }, "O canvas, as ligações, a trava de tela e os cantos mortos ficam desabilitados até o módulo de teclado e mouse funcionar nesta máquina.")] }),
    );
    return { no };
  }

  let layout = null;
  let monitores = null;
  let ultimoAnuncioMonitores = null;
  let preparando = false;
  let selecao = null; // { tipo: "borda", ponta, linkId, destino } | { tipo: "dispositivo", peerId }
  let escala = 1;
  let origem = { x: 0, y: 0 };
  let avisoPainel = null;
  let resultadoTeste = null;
  let testando = false;
  let salvando = false;
  let saiu = false;
  let geracaoCarga = 0;
  let avancado = false;
  let operacao = false;
  let recargaPendente = false;
  let focoBorda = null;
  let alvoBorda = null;
  const origemFormulario = { peer_id: null, monitor: null, edge: "right" };
  const anuncios = criarAnunciador();
  const chaves = new Map();
  let proximaChave = 0;
  const chave = (tipo, id) => {
    const identidade = JSON.stringify([tipo, id]);
    if (!chaves.has(identidade)) chaves.set(identidade, `layout-${++proximaChave}`);
    return chaves.get(identidade);
  };
  const encontrar = (valor) => [...no.querySelectorAll("[data-focus-key]")].find((e) => e.dataset.focusKey === valor);
  const fallbackCanvas = () => encontrar(alvoBorda ?? focoBorda?.chave) ?? no.querySelector(".layout__dispositivo, .layout__origem select, button");
  const escolha = { peer_id: consumirParDoLayout(), edge: null };
  let idMalha = novoId("layout-malha");
  let focarPopover = false;

  const envoltorio = h("div", { class: "layout__canvas" });
  const painel = h("div", { class: "layout__painel" });
  const popover = h("div", { class: "layout__popover-area" });
  const areaLigacoes = h("div", { class: "layout__area-ligacoes" });
  const areaRodape = h("div");
  const areaOferta = h("div", { class: "layout__oferta" });
  const avisoTopo = h("div");
  const areaTravessia = h("div");
  const areaSimples = h("div", { class: "layout__simples" });
  const ajustes = h("details", { class: "layout__avancado", dataset: { focusKey: "layout-avancado" } }, h("summary", null, "Ajustes avançados"), areaRodape);
  ajustes.addEventListener("toggle", () => {
    avancado = ajustes.open;
    no.toggleAttribute("data-avancado", avancado);
    if (!avancado && selecao?.tipo === "borda") limparSelecao();
    if (layout) { pintarCanvas(); pintarPainel(); }
  });
  function abrirAvancado() { ajustes.open = true; }
  function pintarTravessia() {
    trocar(areaTravessia, resumoTravessia(obterApp(), { ir, acoes: false }));
  }
  function pintarSimples() {
    if (!layout || saiu || !monitores) return;
    const estado = monitores.estado();
    const foco = guardarFoco(areaSimples);
    trocar(areaSimples, editorMonitores({ ...estado, ocupado: estado.ocupado || preparando || operacao,
      aoSelecionar: monitores.selecionar, aoMudar: monitores.alterar,
      aoPrever: monitores.prever, aoAplicar: async () => { await monitores.aplicar(); await carregar({ silencioso: true }); },
      aoAtualizar: () => atualizarMonitores(false), aoPreparar: () => atualizarMonitores(true),
      aoAnunciar: (tipo, texto, origem) => {
        // Repintura não é uma nova ação: não sobrepõe um alerta mais recente.
        // A identidade do resultado permite anunciar uma nova prévia de texto igual.
        if (origem != null) {
          if (origem === ultimoAnuncioMonitores) return;
          ultimoAnuncioMonitores = origem;
        }
        anuncios[tipo](texto);
      },
    }));
    // Aplicar deixa de estar disponível quando a prévia é consumida, inclusive por erro.
    // Só transfere o foco que ainda estava nessa ação no instante da repintura.
    if (foco?.chave === "monitores-aplicar" && !estado.previa && !estado.ocupado &&
        no.ownerDocument.defaultView?.location.href === rotaMontagem) {
      restaurarFoco(foco, areaSimples, { fallback: () => encontrar("monitores-prever") });
    }
  }
  async function atualizarMonitores(preparar) {
    if (preparando || monitores?.estado().ocupado) return;
    preparando = true; pintarSimples();
    try {
      if (preparar) await api.layoutPrepareMonitors();
      const novo = await api.layoutGet();
      if (saiu) return;
      layout = novo; monitores.atualizar(novo, true); pintarTudo();
      if (!preparar) await monitores.prever();
    } catch (erro) { mostrarAvisoTopo("aviso", mensagemDe(erro)); }
    finally { preparando = false; pintarSimples(); }
  }
  async function testarEntrada(peerId) {
    if (testando || salvando) return;
    const nome = layout.devices.find((d) => d.peer_id === peerId)?.name ?? "";
    const foco = guardarFoco(no);
    testando = true;
    resultadoTeste = null;
    pintarSimples();
    pintarPainel();
    restaurarFoco(foco, no);
    try {
      const r = await api.inputTest({ peer_id: peerId, seconds: 5 });
      resultadoTeste = { peerId, nome, stats: r.stats };
      anuncios.status(`${resultadoDoTeste(r.stats).titulo}. ${r.stats.received} recebidos, ${r.stats.lost} perdidos, ${r.stats.out_of_order} fora de ordem.`);
    } catch (erro) {
      mostrarAvisoTopo("aviso", `O teste não foi concluído: ${mensagemDe(erro)} Confira o Diagnóstico.`);
    } finally {
      testando = false;
      if (!saiu) { pintarSimples(); pintarPainel(); restaurarFoco(foco, no, { fallback: () => encontrar("layout-testar") }); if (recargaPendente) await carregar({ silencioso: true }); }
    }
  }
  async function moverDispositivo(d, x, y) {
    const r = await executar(() => api.layoutSetDevice({ peer_id: d.peer_id, x, y, hotkey: d.hotkey, base_rev: layout.rev }));
    if (!r || saiu) return;
    const destino = d.is_self ? escolha.peer_id : d.peer_id;
    escolha.peer_id = destino;
    escolha.edge = posicaoRelativa(layout, destino);
    escolha.posicao = !d.is_self ? { x: r.device.x, y: r.device.y } : null;
    pintarSimples();
  }

  let larguraPintada = 0;
  const observador = new ResizeObserver(() => {
    // Só repinta se a largura mudou: a troca do SVG muda a altura e dispararia o observador de novo.
    if (layout && Math.abs(envoltorio.clientWidth - larguraPintada) > 1) pintarCanvas();
  });
  observador.observe(envoltorio);

  async function carregar({ silencioso = false, duranteOperacao = false } = {}) {
    if (saiu) return;
    if (salvando || testando || (operacao && !duranteOperacao)) { recargaPendente = true; return; }
    recargaPendente = false;
    const foco = guardarFoco(no);
    const minha = ++geracaoCarga;
    if (!silencioso) trocar(corpo, esqueleto({ linhas: 6 }));
    try {
      const proximo = await api.layoutGet();
      if (saiu || minha !== geracaoCarga) return;
      layout = proximo;
      if (!monitores) monitores = controladorMonitores({ layout, api, aoMudar: pintarSimples });
      else monitores.atualizar(layout);
    } catch (erro) {
      if (saiu || minha !== geracaoCarga) return;
      trocar(corpo, estadoErro(erro, () => carregar()));
      return;
    }
    // Seleção de ligação que já não existe cai.
    if (selecao?.tipo === "borda" && selecao.linkId && !layout.links.some((l) => l.id === selecao.linkId)) selecao = null;
    if (selecao?.tipo === "dispositivo" && !layout.devices.some((d) => d.peer_id === selecao.peerId)) selecao = null;
    try {
      pintarTudo();
      restaurarFoco(foco, no, { fallback: fallbackCanvas });
    } catch (erro) {
      // Falha ao desenhar nunca fica muda: a tela mostra o erro em vez de um canvas vazio.
      console.error("falha ao pintar o layout", erro);
      trocar(corpo, estadoErro(erro, () => carregar()));
    }
  }

  function pintarTudo() {
    trocar(ajustes, h("summary", null, "Ajustes avançados"),
      h("p", { class: "nota", id: "layout-canvas-ajuda" }, "Ligações avançadas por borda. Alterar uma ligação gerida a converte em avançada; a próxima organização a preserva ou informa conflito."),
      h("div", { class: "layout__principal" }, cartao({ plano: true, filhos: [h("div", { class: "layout__palco" }, envoltorio, popover)] })),
      h("div", { class: "layout__lateral" }, painel, areaLigacoes), areaRodape,
      recuperacaoMonitores({ layout, aoRestaurar: async (args) => {
        const r = await api.layoutRestoreV1(args);
        mostrarResultadoOrganizacao(r, "Layout v1 restaurado.");
        monitores = null; await carregar({ silencioso: true });
      } }),
    );
    trocar(corpo, avisoTopo, areaTravessia, areaOferta, areaSimples, ajustes);
    pintarTravessia(); pintarSimples(); pintarOferta(); pintarCanvas(); pintarPainel(); pintarLigacoes(); pintarRodape();
  }

  function mostrarResultadoOrganizacao(resultado, confirmado) {
    const ativacao = resultado.activation === "unconfirmed"
      ? "Organização salva, ativação não confirmada"
      : confirmado;
    const durabilidade = resultado.durability === "unconfirmed"
      ? " A revisão foi publicada, mas a persistência após uma queda não foi confirmada."
      : "";
    mostrarAvisoTopo("info", ativacao + durabilidade);
  }

  function mostrarAvisoTopo(tom, texto) {
    trocar(avisoTopo, texto && h("div", { class: `aviso aviso--${tom}` }, h("p", null, texto)));
    if (texto) { if (tom === "erro" || tom === "aviso") anuncios.erro(texto); else anuncios.status(texto); }
  }

  async function executar(fn, { depois = "recarregar" } = {}) {
    if (operacao || salvando || testando || saiu) return undefined;
    const foco = guardarFoco(no);
    operacao = true;
    avisoPainel = null;
    try {
      const r = await fn();
      if (saiu) return undefined;
      if (depois === "recarregar") await carregar({ silencioso: true, duranteOperacao: true });
      return r;
    } catch (erro) {
      if (saiu) return undefined;
      if (erro?.code === "stale_rev") {
        mostrarAvisoTopo("aviso", mensagemDe(erro));
        await carregar({ silencioso: true, duranteOperacao: true });
        return undefined;
      }
      if (ERROS_DE_OFERTA.has(erro?.code)) {
        // A oferta mudou ou sumiu depois que a tela a mostrou: o texto do erro fica no topo e a oferta é lida de novo.
        mostrarAvisoTopo("aviso", mensagemDe(erro));
        await carregar({ silencioso: true, duranteOperacao: true });
        return undefined;
      }
      mostrarAvisoTopo("erro", mensagemDe(erro));
      avisoPainel = { tom: "erro", texto: mensagemDe(erro) };
      pintarPainel();
      return undefined;
    } finally {
      operacao = false;
      if (!saiu) {
        // As closures dos controles também capturam ocupado; limpar só a variável
        // mantém a última renderização bloqueada após a carga durante a operação.
        pintarSimples(); pintarOferta();
        restaurarFoco(foco, no, { fallback: fallbackCanvas });
        if (recargaPendente) await carregar({ silencioso: true });
      }
    }
  }

  // ----- geometria -----

  function retangulos() {
    const lista = [];
    for (const d of layout.devices) {
      const monitores = d.monitors.length > 0 ? d.monitors : [{ ref: null, name: "sem monitores conhecidos", index: 0, x: 0, y: 0, width: 1920, height: 1080, scale: 1 }];
      const pos = posicaoVisual(d);
      for (const m of monitores) lista.push({ d, m, x: (pos?.x ?? d.x) + m.x, y: (pos?.y ?? d.y) + m.y, w: m.width, h: m.height, fantasma: m.ref === null });
    }
    return lista;
  }

  function posicaoVisual(d) {
    return !avancado && d.peer_id === escolha.peer_id && proporLigacao(layout, escolha)
      ? posicionarDestino(layout, escolha) : { x: d.x, y: d.y };
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
    return { largura, altura: Math.max(avancado ? ALTURA_MINIMA_PX : 320, Math.round((maxY - minY + extraCima) * escala)) };
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
    return trechosDisponiveis(layout, ponta, selecao?.linkId).filter((t) => t.to - t.from >= PASSO_FRACAO);
  }

  // ----- canvas -----

  function pintarCanvas() {
    const rects = retangulos();
    const { largura, altura } = calcularEscala(rects);
    larguraPintada = envoltorio.clientWidth;
    const desenho = svg("svg", { class: "layout__svg", width: largura, height: altura, role: "group", "aria-label": "Monitores e prévia da passagem", "aria-describedby": "layout-canvas-ajuda" });
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
    if (!avancado) {
      const proposta = proporLigacao(layout, escolha);
      if (proposta) for (const [ponta, texto] of [[proposta.a, "Saída"], [proposta.b, "Entrada"]]) {
        const r = retanguloDe(ponta, rects);
        if (!r) continue;
        const a = pontoNaBorda(r, ponta.edge, ponta.from);
        const b = pontoNaBorda(r, ponta.edge, ponta.to);
        desenho.append(svg("line", { class: "layout__previa-borda", x1: a.x, y1: a.y, x2: b.x, y2: b.y }));
        const rotulo = svg("text", { class: "layout__texto", x: (a.x + b.x) / 2 + 8, y: (a.y + b.y) / 2 - 8 });
        rotulo.textContent = texto;
        desenho.append(rotulo);
      }
    }

    trocar(envoltorio, desenho);
  }

  function grupoDispositivo(d, rects) {
    const meus = rects.filter((r) => r.d === d);
    const ativo = layout.active_peer_id === d.peer_id;
    const grupo = svg("g", {
      class: `layout__dispositivo${d.is_self ? " layout__dispositivo--eu" : ""}${d.state !== "connected" && !d.is_self ? " layout__dispositivo--fora" : ""}${selecao?.tipo === "dispositivo" && selecao.peerId === d.peer_id ? " layout__dispositivo--selecionado" : ""}`,
      tabindex: 0,
      role: "button",
      "data-focus-key": chave("dispositivo", d.peer_id),
      "aria-pressed": String(selecao?.tipo === "dispositivo" && selecao.peerId === d.peer_id),
      "aria-describedby": "layout-canvas-ajuda",
      "aria-label": `${d.name}${d.is_self ? " (esta máquina)" : ""}, ${ESTADOS[d.state] ?? d.state}${atalhoGlobal(d.hotkey) ? `, atalho ${atalhoGlobal(d.hotkey)}` : ""}.`,
    });

    const primeiroMonitor = [...meus].sort((a, b) => a.y - b.y || a.x - b.x)[0];
    const primeiro = tela(primeiroMonitor.x, primeiroMonitor.y);
    const texto = `${d.name}${d.is_self ? " · esta máquina" : ""}${atalhoGlobal(d.hotkey) ? ` · ${atalhoGlobal(d.hotkey)}` : ""}${ativo ? " · ativo" : ""}${d.state !== "connected" && !d.is_self ? ` · ${ESTADOS[d.state] ?? d.state}` : ""}`;
    // Nome cabe na área da própria máquina, sem cobrir monitores vizinhos.
    const larguraGrupo = primeiroMonitor.w * escala;
    const rotulo = svg("text", { class: "layout__nome", x: primeiro.x + 8, y: primeiro.y + 16 });
    rotulo.textContent = caber(texto, larguraGrupo - 16);
    const nomeIntegral = svg("title"); nomeIntegral.textContent = texto; grupo.append(nomeIntegral);
    for (const r of meus) {
      const p = tela(r.x, r.y);
      const w = r.w * escala;
      const hh = r.h * escala;
      grupo.append(svg("rect", { class: `layout__monitor${r.fantasma ? " layout__monitor--fantasma" : ""}`, x: p.x, y: p.y, width: w, height: hh, rx: 4 }));
      const t1 = svg("text", { class: "layout__texto", x: p.x + 8, y: p.y + 38 });
      t1.textContent = caber(r.fantasma ? r.m.name : nomeDoMonitor(r.m), w - 16);
      grupo.append(t1);
      if (!r.fantasma && w > 72 && hh > 40) {
        const t2 = svg("text", { class: "layout__texto layout__texto--fraco", x: p.x + 8, y: p.y + 54 });
        t2.textContent = caber(medidasDoMonitor(r.m), w - 16);
        grupo.append(t2);
      }
      if (avancado && !r.fantasma) for (const edge of Object.keys(BORDAS)) grupo.append(zonaBorda(d, r, { x: p.x, y: p.y, w, h: hh }, edge));
    }

    grupo.append(rotulo);

    // Bordas internas travadas
    for (const e of (avancado ? layout.locked_edges : []).filter((x) => x.peer_id === d.peer_id)) {
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

    grupo.addEventListener("click", (evento) => { if (evento.target === grupo) selecionarDispositivo(d); });
    grupo.addEventListener("keydown", (evento) => {
      if (evento.target === grupo && ["Enter", " "].includes(evento.key)) { evento.preventDefault(); selecionarDispositivo(d); }
    });
    return grupo;
  }

  function zonaBorda(d, r, rr, edge) {
    const espessura = 10;
    const geo = vertical(edge)
      ? { x: (edge === "left" ? rr.x : rr.x + rr.w) - espessura / 2, y: rr.y, width: espessura, height: rr.h }
      : { x: rr.x, y: (edge === "top" ? rr.y : rr.y + rr.h) - espessura / 2, width: rr.w, height: espessura };
    const ponta = { peer_id: d.peer_id, monitor: r.m.ref, edge, from: 0, to: 1 };
    const selecionada = selecao?.tipo === "borda" && mesmaBorda(selecao.ponta, ponta);
    const zona = svg("rect", {
      class: `layout__borda${selecionada ? " layout__borda--selecionada" : ""}${ligacoesLocais(layout).length === 0 && d.is_self && trechosDisponiveis(layout, ponta).length > 0 ? " layout__borda--disponivel" : ""}`,
      ...geo,
      tabindex: 0,
      role: "button",
      "data-focus-key": chave("borda", [d.peer_id, r.m.ref, edge]),
      "aria-describedby": "layout-canvas-ajuda",
      "aria-label": `${d.name}, ${nomeDoMonitor(r.m)}, borda ${BORDAS[edge]}`,
    });
    const escolher = (fracaoClicada) => {
      focoBorda = guardarFoco(no);
      alvoBorda = chave("borda", [d.peer_id, r.m.ref, edge]);
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
    g.addEventListener("click", () => {
      if (avancado) selecionarLigacao(l, l.a);
      else {
        const d = layout.devices.find((x) => !x.is_self && [l.a.peer_id, l.b.peer_id].includes(x.peer_id));
        if (d) selecionarDispositivo(d);
      }
    });
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
      for (const [alca, lado] of [[alcaDe, "from"], [alcaAte, "to"]]) {
        const limites = limitesAlca(layout, ponta, lado, selecao?.linkId);
        alca.setAttribute("aria-valuenow", ponta[lado] * 100);
        alca.setAttribute("aria-valuetext", pct(ponta[lado]));
        alca.setAttribute("aria-valuemin", (limites?.min ?? ponta[lado]) * 100);
        alca.setAttribute("aria-valuemax", (limites?.max ?? ponta[lado]) * 100);
      }
    };
    selecaoAtual = g;
    return g;
  }

  let selecaoAtual = null;

  function alca(rr, ponta, lado, p) {
    const limites = limitesAlca(layout, ponta, lado, selecao?.linkId) ?? { min: ponta[lado], max: ponta[lado] };
    const circulo = svg("circle", {
      class: "layout__alca",
      cx: p.x,
      cy: p.y,
      r: 7,
      tabindex: 0,
      role: "slider",
      "data-focus-key": chave("alca", [ponta.peer_id, ponta.monitor, ponta.edge, lado]),
      "aria-label": `${lado === "from" ? "Começo" : "Fim"} do trecho, ${descreverPonta(layout, ponta)}`,
      "aria-orientation": vertical(ponta.edge) ? "vertical" : "horizontal",
      "aria-valuemin": limites.min * 100,
      "aria-valuemax": limites.max * 100,
      "aria-valuenow": Math.round(ponta[lado] * 100),
      "aria-valuetext": pct(ponta[lado]),
    });
    const aplicar = (f) => {
      const permitidos = limitesAlca(layout, ponta, lado, selecao?.linkId);
      if (!permitidos) return;
      const novo = f === permitidos.min || f === permitidos.max ? f : limitar(arredondarFracao(f), permitidos.min, permitidos.max);
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
      const inicioTrecho = { from: ponta.from, to: ponta.to };
      const caixaSvg = envoltorio.querySelector("svg").getBoundingClientRect();
      const mover = (e) => {
        const px = e.clientX - caixaSvg.left;
        const py = e.clientY - caixaSvg.top;
        const f = vertical(ponta.edge) ? (py - rr.y) / rr.h : (px - rr.x) / rr.w;
        aplicar(limitar(f, 0, 1));
      };
      const soltar = (eventoFinal) => {
        if (eventoFinal.type === "pointercancel") { Object.assign(ponta, inicioTrecho); selecaoAtual?.atualizar(); pintarPainel(); }
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
      const permitidos = limitesAlca(layout, ponta, lado, selecao?.linkId);
      if (!permitidos || (!delta && evento.key !== "Home" && evento.key !== "End")) return;
      evento.preventDefault();
      aplicar(evento.key === "Home" ? permitidos.min : evento.key === "End" ? permitidos.max : ponta[lado] + delta * PASSO_FRACAO);
    });
    return circulo;
  }

  function selecionarLigacao(l, pontaClicada) {
    focoBorda = guardarFoco(no);
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
    if (!d.is_self) { if (escolha.peer_id !== d.peer_id) escolha.posicao = null; escolha.peer_id = d.peer_id; escolha.edge = posicaoRelativa(layout, d.peer_id); }
    selecao = { tipo: "dispositivo", peerId: d.peer_id };
    avisoPainel = null;
    resultadoTeste = null;
    pintarSimples();
    pintarCanvas();
    pintarPainel();
    pintarLigacoes();
  }

  function limparSelecao() {
    const foco = guardarFoco(popover);
    selecao = null;
    avisoPainel = null;
    pintarCanvas();
    pintarPainel();
    pintarLigacoes();
    restaurarFoco(foco, no, { fallback: fallbackCanvas });
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
      { dataset: { focusKey: "borda-destino-computador" }, class: "entrada", id: idDispositivo, disabled: dispositivos.length === 0 },
      dispositivos.length === 0 && h("option", { value: "" }, "Nenhuma outra máquina com monitores conhecidos"),
      dispositivos.map(([peerId, nome]) => h("option", { value: peerId, selected: selecao.destino?.peer_id === peerId }, nome)),
    );
    const seletorBorda = h("select", { dataset: { focusKey: "borda-destino-monitor" }, class: "entrada", id: idBordaDestino, disabled: dispositivos.length === 0 });
    const entradaDe = h("input", { dataset: { focusKey: "borda-destino-de" }, "aria-label": "Início do trecho de destino (%)", class: "entrada entrada--mono layout__pct", id: idDe, type: "number", min: 0, max: 100, step: 5, value: Math.round((selecao.destino?.from ?? 0) * 100), disabled: !selecao.destino });
    const entradaAte = h("input", { dataset: { focusKey: "borda-destino-ate" }, "aria-label": "Fim do trecho de destino (%)", class: "entrada entrada--mono layout__pct", id: idAte, type: "number", min: 0, max: 100, step: 5, value: Math.round((selecao.destino?.to ?? 1) * 100), disabled: !selecao.destino });

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

    entradaDe.addEventListener("input", () => { if (selecao.destino) selecao.destino.from = Number(entradaDe.value) / 100; });
    entradaAte.addEventListener("input", () => { if (selecao.destino) selecao.destino.to = Number(entradaAte.value) / 100; });
    const lerDestino = () => {
      if (!selecao.destino) return null;
      return { peer_id: selecao.destino.peer_id, monitor: selecao.destino.monitor, edge: selecao.destino.edge, from: limitar(Number(entradaDe.value) / 100, 0, 1), to: limitar(Number(entradaAte.value) / 100, 0, 1) };
    };

    const acoes = h(
      "div",
      { class: "linha layout__popover-acoes" },
      botao({
        rotulo: linkId ? "Salvar ligação" : "Ligar", dataset: { focusKey: "borda-salvar" },
        variante: "primario",
        icone: "verificar",
        desabilitado: !selecao.destino || !recurso("layout_links"),
        aoClicar: async () => {
          const focoRetorno = guardarFoco(no);
          const b = lerDestino();
          if (!b) return;
          const r = await executar(() => api.layoutSetLink({ id: linkId, a: { peer_id: ponta.peer_id, monitor: ponta.monitor, edge: ponta.edge, from: ponta.from, to: ponta.to }, b, base_rev: layout.rev }), { depois: "nada" });
          if (!r) return;
          if (r.warnings?.length > 0) mostrarAvisoTopo("info", "A ligação foi gravada. O trecho foi encurtado nas pontas pelos cantos mortos.");
          const foco = focoRetorno;
          selecao = null;
          await carregar({ silencioso: true });
          restaurarFoco(foco, no, { fallback: fallbackCanvas });
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
      botao({ rotulo: "Cancelar", dataset: { focusKey: "borda-cancelar" }, variante: "discreto", icone: "fechar", aoClicar: limparSelecao }),
    );

    const no = h(
      "section",
      { class: `layout__popover ${classesDoLado(ponta)}`, dataset: { scrollKey: "layout-borda-editor" }, role: "dialog", "aria-modal": "false", "aria-labelledby": idTitulo },
      h("h2", { id: idTitulo }, linkId ? `Destino da borda (ligação ${linkId})` : "Destino da borda"),
      avisoPainel && aviso(avisoPainel.tom, h("p", null, avisoPainel.texto)),
      !recurso("layout_links") && h("p", { class: "nota" }, `Ligações: ${INDISPONIVEL}.`),
      h("div", { class: "layout__campo" }, h("span", { class: "campo__rotulo" }, "Borda"), h("p", { class: "mono layout__borda-texto" }, descreverPonta(layout, ponta)), h("p", { class: "nota" }, "Ajuste os percentuais abaixo ou use as alças no canvas. Home e End usam os limites permitidos.")),
      camposOrigem(ponta),
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
    if (resultadoTeste?.peerId === d.peer_id) {
      const s = resultadoTeste.stats;
      const resumo = resultadoDoTeste(s);
      trocar(
        areaTeste,
        h("div", { class: `aviso aviso--${resumo.tom}` }, h("strong", null, resumo.titulo), h("p", null, resumo.texto)),
        h(
          "dl",
          { class: "kv layout__estatisticas" },
          h("dt", null, "Recebidos"),
          h("dd", { class: "mono" }, String(s.received)),
          h("dt", null, "Perdidos"),
          h("dd", { class: "mono" }, String(s.lost)),
          h("dt", null, "Fora de ordem"),
          h("dd", { class: "mono" }, String(s.out_of_order)),
          h("dt", null, "Tempos do teste p50 / p95 / máx (não são latência de rede)"),
          h("dd", { class: "mono" }, `${(s.p50_us / 1000).toFixed(2)} ms / ${(s.p95_us / 1000).toFixed(2)} ms / ${(s.max_us / 1000).toFixed(2)} ms`),
        ),
      );
    }

    return cartao({
      titulo: d.name,
      acoes: h("div", { class: "linha" }, d.is_self && selo("acento", "esta máquina"), seloDispositivo(d.state), layout.active_peer_id === d.peer_id && selo("ok", "ativo")),
      filhos: [
        avisoPainel && aviso(avisoPainel.tom, h("p", null, avisoPainel.texto)),
        posicaoFina(d),
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
              rotulo: testando ? "Teste em andamento..." : "Testar passagem (5 s)",
              dataset: { focusKey: chave("painel-testar", d.peer_id) },
              "aria-busy": String(testando), "aria-disabled": String(testando),
              icone: "teclado",
              desabilitado: d.state !== "connected",
              aoClicar: () => testarEntrada(d.peer_id),
            }),
            botao({ rotulo: "Cancelar seleção", variante: "discreto", icone: "fechar", aoClicar: limparSelecao }),
          ),
        d.is_self && h("div", { class: "linha" }, botao({ rotulo: "Cancelar seleção", variante: "discreto", icone: "fechar", aoClicar: limparSelecao })),
        !d.is_self && h("p", { class: "nota" }, "Após iniciar, empurre a borda ligada e mova o mouse. O teste pode controlar o destino e pedir permissão do sistema. A volta pela borda precisa ser conferida manualmente."),
        areaTeste,
      ],
    });
  }

  // ----- lista de ligações -----

  function atalhoDaLigacao(l) {
    const outra = [l.a, l.b].map((p) => layout.devices.find((d) => d.peer_id === p.peer_id)).find((d) => d && !d.is_self);
    return atalhoGlobal(outra?.hotkey);
  }

  function pintarLigacoes() {
    const vazio = h("div", { class: "estado" }, h("h2", null, "Nenhuma passagem configurada"), h("p", { class: "muted" }, "O mouse desta máquina não sai pelas bordas. Configure a posição no computador que tem o mouse físico."));
    const total = layout.links.length;
    const itens = layout.links.map((l) => {
      const selecionada = selecao?.tipo === "borda" && selecao.linkId === l.id;
      const atalho = atalhoDaLigacao(l);
      const corpoLigacao = h(
        "button",
        { dataset: { focusKey: chave("ligacao", l.id) }, class: "layout__lig-corpo", type: "button", "aria-pressed": String(selecionada) },
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
    if (!fora) return selo("neutro", "Configurada");
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
    if (layout.pending_monitor_offer) {
      trocar(areaOferta, ofertaMonitores({ layout, ocupado: operacao, aoResponder: (args) => executar(async () => {
        const r = await api.layoutMonitorOfferAnswer(args);
        if (!saiu && args.accept) mostrarResultadoOrganizacao(r, "Organização aceita. Confira a travessia nas máquinas.");
        return r;
      }) }));
      return;
    }
    const o = layout.pending_offer;
    if (!o) {
      trocar(areaOferta);
      return;
    }
    trocar(
      areaOferta,
      aviso(
        "info",
        h("p", null, `${o.from_name} ofereceu um layout (rev ${o.rev}) com ${o.links.length} ${o.links.length === 1 ? "ligação" : "ligações"}. Aceitar substitui as ligações que cruzam as mesmas bordas.`),
        botao({ rotulo: "Aceitar layout", variante: "primario", icone: "verificar", aoClicar: () => executar(() => api.layoutOfferAnswer(respostaOferta(o, true))) }),
        botao({ rotulo: "Recusar", icone: "fechar", aoClicar: () => executar(() => api.layoutOfferAnswer(respostaOferta(o, false))) }),
      ),
    );
  }

  // ----- barra de ações (sob o canvas) -----

  function camposOrigem(ponta) {
    const campos = ["from", "to"].map((lado) => {
      const permitidos = limitesAlca(layout, ponta, lado, selecao?.linkId);
      const id = novoId("origem-percentual");
      const entrada = h("input", {
        class: "entrada entrada--mono layout__pct", id, type: "number", step: "any",
        min: (permitidos?.min ?? ponta[lado]) * 100, max: (permitidos?.max ?? ponta[lado]) * 100,
        value: Math.round(ponta[lado] * 10000) / 100,
        dataset: { focusKey: `borda-origem:${lado}` },
      });
      entrada.addEventListener("change", () => {
        const limites = limitesAlca(layout, ponta, lado, selecao?.linkId);
        const valor = Number(entrada.value) / 100;
        if (!limites || !Number.isFinite(valor)) return;
        ponta[lado] = limitar(valor, limites.min, limites.max);
        entrada.value = Math.round(ponta[lado] * 10000) / 100;
        selecaoAtual?.atualizar();
      });
      return h("div", { class: "campo" }, h("label", { for: id }, lado === "from" ? "Início do trecho de origem (%)" : "Fim do trecho de origem (%)"), entrada);
    });
    return h("div", { class: "layout__trecho" }, campos);
  }

  function formularioOrigem() {
    const computadores = layout.devices.filter((d) => d.monitors.length > 0);
    const d = computadores.find((d) => d.peer_id === origemFormulario.peer_id) ?? computadores.find((d) => d.is_self) ?? computadores[0];
    if (!d) return h("p", { class: "nota" }, "Sem monitores conhecidos. Conecte os computadores e confira o módulo de teclado e mouse para editar uma borda.");
    origemFormulario.peer_id = d.peer_id;
    const m = d.monitors.find((m) => m.ref === origemFormulario.monitor) ?? d.monitors[0];
    origemFormulario.monitor = m.ref;
    const ponta = { peer_id: d.peer_id, monitor: m.ref, edge: origemFormulario.edge };
    const seletor = (rotulo, valor, opcoes, focusKey, aoMudar) => campo({ rotulo, tipo: "select", valor, opcoes, dataset: { focusKey }, aoMudar });
    const opcoesTrecho = [
      ...trechosDisponiveis(layout, ponta).map((trecho, i) => ({ valor: `livre:${i}`, rotulo: `Criar no trecho livre de ${pct(trecho.from)} a ${pct(trecho.to)}`, trecho })),
      ...layout.links.filter((l) => [l.a, l.b].some((p) => mesmaBorda(p, ponta))).map((ligacao, i) => {
        const trecho = mesmaBorda(ligacao.a, ponta) ? ligacao.a : ligacao.b;
        return { valor: `ligacao:${i}`, rotulo: `Editar ligação ${ligacao.id}, de ${pct(trecho.from)} a ${pct(trecho.to)}`, ligacao };
      }),
    ];
    const opcao = opcoesTrecho.find((o) => o.valor === origemFormulario.trecho) ?? opcoesTrecho[0];
    origemFormulario.trecho = opcao?.valor ?? null;
    const atualizar = (nome, valor) => {
      origemFormulario[nome] = valor;
      if (nome !== "trecho") origemFormulario.trecho = null;
      pintarRodape();
    };
    return cartao({ titulo: "Editar sem arrastar", filhos: [
      h("p", { class: "nota" }, "Escolha a origem. O editor permite escolher destino e início/fim dos dois trechos. Para uma ligação existente, use também a lista Ligações."),
      h("div", { class: "layout__origem" },
        seletor("Computador de origem", d.peer_id, computadores.map((c) => ({ valor: c.peer_id, rotulo: c.name })), "origem-computador", (valor) => atualizar("peer_id", valor)),
        seletor("Monitor de origem", m.ref, d.monitors.map((m) => ({ valor: m.ref, rotulo: nomeDoMonitor(m) })), "origem-monitor", (valor) => atualizar("monitor", valor)),
        seletor("Borda de origem", ponta.edge, Object.entries(BORDAS).map(([valor, rotulo]) => ({ valor, rotulo })), "origem-borda", (valor) => atualizar("edge", valor)),
        seletor("Trecho da borda", origemFormulario.trecho, opcoesTrecho.length ? opcoesTrecho : [{ valor: "", rotulo: "Nenhum trecho disponível nesta borda" }], "origem-trecho", (valor) => atualizar("trecho", valor)),
      ),
      h("div", { class: "linha" },
        botao({ rotulo: "Abrir trecho selecionado", desabilitado: !opcao, dataset: { focusKey: "origem-editar" }, aoClicar: () => {
          focoBorda = guardarFoco(no); alvoBorda = "origem-editar";
          if (opcao?.ligacao) { selecionarLigacao(opcao.ligacao, ponta); return; }
          const trecho = opcao?.trecho;
          if (!trecho) { mostrarAvisoTopo("aviso", "Esta borda não tem trecho livre. Escolha outra borda ou edite uma ligação existente."); return; }
          selecao = { tipo: "borda", ponta: { ...ponta, ...trecho }, linkId: null, destino: destinoPadrao(ponta) };
          focarPopover = true; pintarCanvas(); pintarPainel(); pintarLigacoes();
        } }),
        botao({ rotulo: "Ajustar posição deste computador", dataset: { focusKey: "origem-posicao" }, aoClicar: () => { selecionarDispositivo(d); painel.querySelector("input")?.focus(); } }),
      ),
    ] });
  }

  function posicaoFina(d) {
    const coordenada = (eixo) => {
      const id = novoId("posicao");
      const entrada = h("input", { class: "entrada entrada--mono", id, type: "number", step: layout.snap_px || MALHA_PX, value: d[eixo], dataset: { focusKey: chave("posicao", [d.peer_id, eixo]) } });
      return { entrada, no: h("div", { class: "campo" }, h("label", { for: id }, `Posição ${eixo.toUpperCase()} (px)`), entrada) };
    };
    const x = coordenada("x"); const y = coordenada("y");
    return h("div", { class: "pilha" }, h("div", { class: "layout__origem" }, x.no, y.no), botao({
      rotulo: "Aplicar posição do computador", dataset: { focusKey: chave("posicao-aplicar", d.peer_id) },
      aoClicar: () => {
        const px = Number(x.entrada.value); const py = Number(y.entrada.value);
        if (!Number.isFinite(px) || !Number.isFinite(py) || !x.entrada.value || !y.entrada.value) { mostrarAvisoTopo("erro", "Informe coordenadas válidas em pixels."); return; }
        return moverDispositivo(d, px, py);
      },
    }));
  }

  function pintarRodape() {
    const travar = botao({
      rotulo: layout.screen_locked ? "Destravar esta tela" : "Travar nesta tela",
      atalho: recurso("screen_lock") ? atalhoGlobal("L") : null,
      desabilitado: !recurso("screen_lock"),
      dataset: { focusKey: "layout-travar" },
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
          atalho: atalhoGlobal(d.hotkey),
          desabilitado: !recurso("switch_device") || d.state !== "connected",
          "aria-pressed": String(layout.active_peer_id === d.peer_id),
          aoClicar: () => executar(() => api.switchToDevice({ peer_id: d.peer_id })),
        }),
      ),
    );

    trocar(
      areaRodape,
      formularioOrigem(),
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
            botao({ rotulo: "Copiar layout para os pares", dataset: { focusKey: "layout-copiar" }, desabilitado: !recurso("layout_copy"), aoClicar: copiarParaPares }),
            !recurso("layout_copy") && indisponivel(),
          ),
        ),
        layout.screen_locked && h("p", { class: "nota" }, "Com a tela travada, o cursor não sai desta máquina."),
        h("div", { class: "layout__ir" }, h("span", { class: "campo__rotulo" }, "Ir para dispositivo"), irPara, !recurso("switch_device") && indisponivel(INDISPONIVEL_NESTE_SISTEMA)),
      ),
    );
  }

  async function copiarParaPares() {
    const foco = guardarFoco(no);
    let r;
    try {
      r = layout.organization ? await api.layoutCopyMonitors({ to_peer_ids: layout.devices.filter(d => !d.is_self).map(d => d.peer_id) }) : await api.layoutCopyToPeers({ peer_ids: null });
    } catch (erro) {
      mostrarAvisoTopo("erro", mensagemDe(erro));
      pintarOferta();
      return;
    }
    if (saiu) return;
    if (!interacaoAtual(foco, no)) {
      trocar(avisoTopo, h("p", null, "A cópia foi processada. Abra o resultado para conferir cada computador."), botao({ rotulo: "Ver resultado da oferta", dataset: { focusKey: "layout-resultado-oferta" }, aoClicar: () => exibirResultadoOferta(r) }));
      anuncios.status("Resultado da cópia disponível nesta página.");
      return;
    }
    await exibirResultadoOferta(r);
  }

  async function exibirResultadoOferta(r) {
    const ESTADOS = { offered: ["ok", "oferecido"], offline: ["aviso", "offline"], denied: ["erro", "recusado"], unsupported: ["neutro", "sem suporte"] };
    const { promessa } = abrirDialogo({
      titulo: "Layout oferecido aos pares", raizRetorno: no,
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
    observarApp(pintarTravessia),
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
      descricao: "Organize cada tela, confira a prévia e aplique as passagens entre os computadores.",
    }),
    anuncios.no,
    corpo,
  );
  carregar();

  return {
    no,
    desmontar: () => {
      saiu = true;
      observador.disconnect();
      for (const fn of cancelarEventos) fn();
    },
  };
}
