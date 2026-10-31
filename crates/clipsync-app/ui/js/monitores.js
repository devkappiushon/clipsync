// Organização virtual: somente a prévia recebida do agente pode ser aplicada.
import { criarRascunho, moverMonitor, encaixarMonitor } from "./monitores-geometria.js";
import { h, novoId } from "./dom.js";
import { svg } from "./icones.js";
import { botao, abrirDialogo } from "./componentes.js";
const keyId = (key) => `${key.peer}:${key.uid}`;
const mesmo = (a, b) => keyId(a) === keyId(b);
const request = (draft) => JSON.parse(JSON.stringify(draft));
const nomePeer = (layout, peer) => layout.devices?.find((d) => d.peer_id === peer)?.name ?? peer.slice(0, 8);
export function candidatosMonitores(layout) {
  const local = layout.inventories?.find((v) => v.stamp.peer === layout.self_id);
  if (
    !local ||
    local.inventory.space !== "Physical" ||
    local.inventory.monitors.length < 1 ||
    local.inventory.monitors.length > 2
  )
    return [];
  return layout.inventories
    .filter(
      (v) =>
        v.stamp.peer !== layout.self_id &&
        v.inventory.monitors.length === 1 &&
        v.inventory.space === "Logical"
    )
    .map((v) => v.stamp.peer);
}
function projetar(layout, destino) {
  const peers = new Set([layout.self_id, destino]);
  return {
    ...layout,
    inventories: (layout.inventories ?? []).filter((v) => peers.has(v.stamp.peer)),
    organization: layout.organization
      ? {
          positions: layout.organization.positions.filter((p) => peers.has(p.key.peer)),
          links: layout.organization.links.filter((l) => peers.has(l.a.key.peer) && peers.has(l.b.key.peer))
        }
      : null
  };
}
export function controladorMonitores({ layout, api, aoMudar = () => {} }) {
  let rascunho = null,
    previa = null,
    erro = null,
    resultado = null,
    ocupado = false,
    destino = null,
    geracao = 0,
    visto = null;
  const rotulos = new Map();
  const lembrar = () => {
    for (const v of layout.inventories ?? [])
      for (const m of v.inventory.monitors) rotulos.set(keyId({ peer: v.stamp.peer, uid: m.uid }), m.label);
  };
  lembrar();
  const candidatos = candidatosMonitores(layout);
  if (candidatos.length === 1) destino = candidatos[0];
  function criar(anteriores) {
    try {
      const snapshot = destino ? projetar(layout, destino) : null;
      if (snapshot && anteriores) {
        const keys = new Set(anteriores.map((p) => keyId(p.key)));
        const salvas = snapshot.organization ?? { positions: [], links: [] };
        snapshot.organization = {
          ...salvas,
          positions: [...salvas.positions.filter((p) => !keys.has(keyId(p.key))), ...anteriores]
        };
      }
      rascunho = snapshot ? criarRascunho(snapshot) : null;
      erro = null;
    } catch (e) {
      erro = e;
      rascunho = null;
    }
  }
  criar();
  const estado = () => ({
    layout,
    rascunho,
    previa,
    erro,
    resultado,
    ocupado,
    destino,
    candidatos: candidatosMonitores(layout),
    rotulos
  });
  const notificar = () => aoMudar(estado());
  function alterar(draft) {
    rascunho = draft;
    previa = null;
    visto = null;
    erro = null;
    resultado = null;
    geracao++;
    notificar();
  }
  function selecionar(peer) {
    if (ocupado || !candidatosMonitores(layout).includes(peer)) return;
    destino = peer;
    geracao++;
    previa = null;
    visto = null;
    criar();
    notificar();
  }
  function atualizar(novo, renovar = false) {
    layout = novo;
    lembrar();
    if (renovar) {
      const antigas = rascunho?.positions;
      criar(antigas);
      geracao++;
      previa = null;
      visto = null;
    } else if (!rascunho) {
      const c = candidatosMonitores(layout);
      if (!destino && c.length === 1) destino = c[0];
      criar();
    } else if (
      previa &&
      (novo.rev !== rascunho.base_rev ||
        JSON.stringify(projetar(novo, destino).inventories.map((v) => v.stamp)) !==
          JSON.stringify(rascunho.inventories))
    ) {
      previa = null;
      visto = null;
      geracao++;
    }
    notificar();
  }
  async function prever() {
    if (ocupado || !rascunho) return;
    const atual = ++geracao,
      pedido = request(rascunho);
    ocupado = true;
    erro = null;
    notificar();
    try {
      const resposta = await api.layoutPreviewOrganization(pedido);
      if (atual !== geracao) return;
      previa = resposta;
      visto = pedido;
    } catch (e) {
      if (atual === geracao) {
        erro = e;
        previa = null;
        visto = null;
      }
    } finally {
      ocupado = false;
      notificar();
    }
  }
  async function aplicar() {
    if (ocupado || !previa || !visto || JSON.stringify(visto) !== JSON.stringify(request(rascunho))) return;
    const pedido = visto,
      confirmada = previa;
    ocupado = true;
    erro = null;
    notificar();
    try {
      resultado = await api.layoutApplyOrganization(pedido);
      const antigas = layout.organization ?? { positions: [], links: [] };
      const keys = new Set(pedido.positions.map((p) => keyId(p.key)));
      const substituidos = new Set(pedido.replace_link_ids);
      layout = {
        ...layout,
        rev: resultado.layout_rev,
        organization: {
          positions: [...antigas.positions.filter((p) => !keys.has(keyId(p.key))), ...pedido.positions],
          links: [...antigas.links.filter((l) => !substituidos.has(l.id)), ...confirmada.effective_links]
        }
      };
      previa = null;
      visto = null;
      geracao++;
    } catch (e) {
      erro = e;
      previa = null;
      visto = null;
    } finally {
      ocupado = false;
      notificar();
    }
  }
  return {
    estado,
    selecionar,
    alterar,
    atualizar,
    prever,
    aplicar,
    mover: (key, pos) => alterar(moverMonitor(rascunho, key, pos)),
    encaixar: (key, alvo, lado) => alterar(encaixarMonitor(rascunho, key, alvo, lado))
  };
}
export function aproximarMonitor(draft, key, pos) {
  const metadata = draft.monitores.find((m) => mesmo(m.key, key));
  if (!metadata) return moverMonitor(draft, key, pos);
  let x = pos.x,
    y = pos.y;
  for (const eixo of ["x", "y"]) {
    const tamanho = eixo === "x" ? "width" : "height",
      opcoes = [];
    for (const p of draft.positions) {
      if (mesmo(p.key, key)) continue;
      const m = draft.monitores.find((v) => mesmo(v.key, p.key));
      if (!m) continue;
      for (const n of [
        p[eixo] - metadata[tamanho],
        p[eixo] + m[tamanho],
        p[eixo],
        p[eixo] + m[tamanho] - metadata[tamanho]
      ])
        if (Math.abs(pos[eixo] - n) <= 16) opcoes.push({ n, d: Math.abs(pos[eixo] - n), key: keyId(p.key) });
    }
    opcoes.sort((a, b) => a.d - b.d || a.key.localeCompare(b.key) || a.n - b.n);
    if (opcoes.length) {
      if (eixo === "x") x = opcoes[0].n;
      else y = opcoes[0].n;
    }
  }
  return moverMonitor(draft, key, { x, y });
}
export function gestosMonitor(
  no,
  { key, obter, aoMudar, aoSelecionar = () => {}, aoErro = () => {}, escala = () => 1, ocupado = () => false }
) {
  const aplicar = (fn) => {
    try {
      aoMudar(fn());
    } catch (erro) {
      aoErro(erro);
    }
  };
  let gesto = null;
  no.addEventListener("pointerdown", (e) => {
    if (e.button !== 0 || ocupado()) return;
    const draft = obter(),
      p = draft.positions.find((p) => mesmo(p.key, key));
    if (!p) return;
    e.preventDefault();
    gesto = { draft, p, x: e.clientX, y: e.clientY, id: e.pointerId, dx: 0, dy: 0 };
    no.setPointerCapture(e.pointerId);
  });
  no.addEventListener("pointermove", (e) => {
    if (!gesto || e.pointerId !== gesto.id) return;
    gesto.dx = (e.clientX - gesto.x) / escala();
    gesto.dy = (e.clientY - gesto.y) / escala();
    no.setAttribute?.("transform", `translate(${gesto.dx} ${gesto.dy})`);
  });
  const fim = (e, cancelar) => {
    if (!gesto || e.pointerId !== gesto.id) return;
    const g = gesto;
    gesto = null;
    no.removeAttribute?.("transform");
    no.releasePointerCapture?.(e.pointerId);
    if (!cancelar && !ocupado())
      aplicar(() => aproximarMonitor(g.draft, key, { x: g.p.x + g.dx, y: g.p.y + g.dy }));
  };
  no.addEventListener("pointerup", (e) => fim(e, false));
  no.addEventListener("pointercancel", (e) => fim(e, true));
  no.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      if (!ocupado()) aoSelecionar();
      return;
    }
    const delta = { ArrowLeft: [-16, 0], ArrowRight: [16, 0], ArrowUp: [0, -16], ArrowDown: [0, 16] }[e.key];
    if (!delta || ocupado()) return;
    e.preventDefault();
    const d = obter(),
      p = d.positions.find((p) => mesmo(p.key, key));
    aplicar(() => moverMonitor(d, key, { x: p.x + delta[0], y: p.y + delta[1] }));
  });
}
const erroTexto = (e) =>
  ({
    identity_ambiguous: "Tela não identificada com segurança",
    stale_inventory: "As telas mudaram. Atualize a prévia conservando suas posições.",
    stale_rev: "O layout mudou. Atualize a prévia antes de aplicar.",
    inventory_required: "Prepare as telas nesta máquina e no notebook.",
    unsupported_origin: "A origem desta jornada precisa ser o desktop Windows.",
    invalid_position: "Informe posições finitas entre -1.000.000 e 1.000.000.",
    monitor_missing: "Tela ausente. Prepare as telas e atualize a prévia."
  })[e?.code] ??
  e?.message ??
  "Não foi possível preparar o rascunho.";
export function editorMonitores({
  layout,
  rascunho,
  ocupado = false,
  previa = null,
  erro = null,
  resultado = null,
  destino = null,
  candidatos = [],
  rotulos = new Map(),
  aoSelecionar,
  aoMudar,
  aoPreparar,
  aoPrever,
  aoAplicar,
  aoAtualizar,
  aoAnunciar = () => {}
}) {
  const root = h("section", { class: "monitores", "aria-label": "Organização por monitor" });
  const erroLocal = h("p", { class: "nota" });
  const informarErro = (e) => {
    erroLocal.textContent = erroTexto(e);
    aoAnunciar("erro", erroLocal.textContent);
  };
  const tentar = (fn) => {
    try {
      fn();
      erroLocal.textContent = "";
    } catch (e) {
      informarErro(e);
    }
  };
  const acao = (rotulo, fn, key, disabled = false) =>
    botao({
      rotulo,
      desabilitado: disabled,
      dataset: { focusKey: key },
      "aria-disabled": String(ocupado || disabled),
      "aria-busy": String(ocupado),
      aoClicar: () => {
        if (!ocupado && !disabled) return fn?.();
      }
    });
  root.append(
    h("h2", null, "Organizar cada tela"),
    h(
      "p",
      { class: "nota" },
      "Desktop Windows como origem, notebook como destino. O mapa é virtual e não altera a disposição das telas no sistema. Cada computador mantém uma única conexão e permissão."
    ),
    h(
      "p",
      { class: "nota" },
      "Prepare as telas também no notebook. Preparar telas atua somente nesta máquina; confirme pessoalmente qualquer diálogo do sistema."
    )
  );
  const escolher = (label, options, value, onChange, id) => {
    const select = h(
      "select",
      { class: "entrada", id, dataset: { focusKey: id }, "aria-disabled": String(ocupado) },
      options.map((o) => h("option", { value: o.value, selected: o.value === value }, o.label))
    );
    select.addEventListener("change", () => {
      if (!ocupado) onChange(select.value);
    });
    return h("label", { class: "campo" }, label, select);
  };
  if (candidatos.length > 1)
    root.append(
      escolher(
        "Notebook de destino",
        [
          { value: "", label: "Escolha o destino" },
          ...candidatos.map((p) => ({ value: p, label: nomePeer(layout, p) }))
        ],
        destino ?? "",
        aoSelecionar,
        "monitor-destino"
      )
    );
  root.append(
    h(
      "div",
      { class: "linha" },
      acao("Preparar telas", aoPreparar, "monitores-preparar"),
      acao("Atualizar prévia", aoAtualizar, "monitores-atualizar")
    )
  );
  if (erro) root.append(h("p", { class: "aviso aviso--erro" }, erroTexto(erro)));
  if (layout.inventory_problem === "identity_ambiguous")
    root.append(h("p", null, "Tela não identificada com segurança"));
  if (resultado)
    root.append(
      h(
        "p",
        { class: "aviso" },
        resultado.activation === "unconfirmed"
          ? "Organização salva, ativação não confirmada"
          : "Organização salva e ativação confirmada pelo agente. A travessia física ainda precisa ser testada.",
        resultado.durability === "unconfirmed" ? " A durabilidade da gravação também não foi confirmada." : ""
      )
    );
  const titulo = (key) =>
    `${nomePeer(layout, key.peer)} · ${rascunho?.monitores.find((m) => mesmo(m.key, key))?.label ?? rotulos.get(keyId(key)) ?? `tela ausente, UID ${key.uid}`}`;
  const preview = h("div", { class: "monitores__previa" });
  if (previa) {
    preview.append(
      h("h3", null, "Prévia das passagens"),
      h(
        "ul",
        null,
        previa.effective_links.map((l) => h("li", null, `${titulo(l.a.key)} ↔ ${titulo(l.b.key)}`))
      ),
      h(
        "p",
        null,
        `${previa.preserved_link_ids.length} ligações preservadas; ${previa.replaced_link_ids.length} substituídas; ${previa.unresolved_link_ids.length} sem identidade resolvida, mantidas inativas.`
      )
    );
  } else
    preview.append(
      h("p", null, "Mova as telas e peça uma prévia. Somente Aplicar organização grava as passagens.")
    );
  preview.append(
    h(
      "div",
      { class: "linha" },
      acao("Conferir prévia", aoPrever, "monitores-prever", !rascunho),
      acao("Aplicar organização", aoAplicar, "monitores-aplicar", !previa)
    )
  );
  root.append(preview);
  if (!rascunho) {
    root.append(
      h(
        "p",
        { class: "nota" },
        candidatos.length > 1
          ? "Escolha o notebook para editar esse par."
          : "Sem um par compatível observado. Prepare as telas, confira a conexão, o protocolo v4 e a permissão de receber controle no destino."
      )
    );
  } else {
    const cards = rascunho.positions.map((p) => ({
      ...p,
      m: rascunho.monitores.find((m) => mesmo(m.key, p.key))
    }));
    const active = cards.filter((p) => p.m);
    const minX = Math.min(0, ...active.map((p) => p.x)) - 50,
      minY = Math.min(0, ...active.map((p) => p.y)) - 50;
    const width = Math.max(400, ...active.map((p) => p.x + p.m.width)) - minX + 50,
      height = Math.max(250, ...active.map((p) => p.y + p.m.height)) - minY + 50;
    const canvas = svg("svg", {
      viewBox: `${minX} ${minY} ${width} ${height}`,
      class: "monitores__canvas",
      role: "group",
      "aria-label": "Posições virtuais. Setas movem; Enter ou Espaço abre a edição da tela numerada."
    });
    // Número curto no mapa; identificação integral permanece visível nos campos abaixo.
    // Nenhuma coordenada ou dimensão operacional é ajustada para acomodar texto.
    const numero = (p) => cards.indexOf(p) + 1;
    for (const p of active) {
      const group = svg(
        "g",
        {
          class: "monitores__cartao",
          tabindex: 0,
          role: "button",
          "data-focus-key": `monitor:${keyId(p.key)}`,
          "aria-label": `Tela ${numero(p)}: ${titulo(p.key)}, X ${p.x}, Y ${p.y}. Setas movem o rascunho; Enter ou Espaço edita as coordenadas.`
        },
        svg("rect", { x: p.x, y: p.y, width: p.m.width, height: p.m.height, rx: 20 }),
        svg("text", {
          x: p.x + p.m.width / 2, y: p.y + p.m.height / 2,
          "text-anchor": "middle", "dominant-baseline": "central",
          "font-size": Math.min(160, p.m.width / (String(numero(p)).length + 2), p.m.height / 3),
          "aria-hidden": "true"
        }, document.createTextNode(String(numero(p))))
      );
      gestosMonitor(group, {
        key: p.key,
        obter: () => rascunho,
        aoMudar,
        aoErro: informarErro,
        aoSelecionar: () => {
          const key = `monitor:${keyId(p.key)}:x`;
          [...root.querySelectorAll("[data-focus-key]")].find(n => n.dataset.focusKey === key)?.focus();
        },
        escala: () => {
          const rect = canvas.getBoundingClientRect();
          return Math.min(rect.width / width, rect.height / height);
        },
        ocupado: () => ocupado
      });
      canvas.append(group);
    }
    root.append(canvas);
    const controls = h("div", { class: "monitores__controles" });
    for (const p of cards) {
      const fieldset = h("fieldset", { class: "monitores__ajuste" }, h("legend", null, `Tela ${numero(p)}: ${titulo(p.key)}`));
      if (!p.m)
        fieldset.append(
          h("p", null, `Tela ausente e inativa. UID ${p.key.uid}. Não há dimensão observada disponível.`)
        );
      else if (p.m.source !== "Observed")
        fieldset.append(h("p", null, "Geometria estimada. Não permite aplicar a organização."));
      const inputs = {};
      for (const axis of ["x", "y"]) {
        const id = novoId("monitor-pos");
        inputs[axis] = h("input", {
          id,
          class: "entrada",
          type: "number",
          step: "any",
          min: -1000000,
          max: 1000000,
          value: p[axis],
          dataset: { focusKey: `monitor:${keyId(p.key)}:${axis}` }
        });
        fieldset.append(
          h("label", { class: "campo", for: id }, `Posição ${axis.toUpperCase()}`, inputs[axis])
        );
      }
      fieldset.append(
        acao(
          "Mover no rascunho",
          () => {
            if (
              !inputs.x.value ||
              !inputs.y.value ||
              !Number.isFinite(Number(inputs.x.value)) ||
              !Number.isFinite(Number(inputs.y.value))
            ) {
              informarErro({ code: "invalid_position" });
              return;
            }
            tentar(() =>
              aoMudar(moverMonitor(rascunho, p.key, { x: Number(inputs.x.value), y: Number(inputs.y.value) }))
            );
          },
          `monitor:${keyId(p.key)}:mover`
        )
      );
      const targets = active.filter((t) => !mesmo(t.key, p.key));
      let target = targets[0],
        side = "direita";
      if (p.m && targets.length) {
        fieldset.append(
          escolher(
            "Encostar em",
            targets.map((t) => ({ value: keyId(t.key), label: titulo(t.key) })),
            keyId(target.key),
            (v) => {
              target = targets.find((t) => keyId(t.key) === v);
            },
            `monitor:${keyId(p.key)}:alvo`
          ),
          escolher(
            "Posição relativa",
            ["esquerda", "direita", "acima", "abaixo"].map((v) => ({ value: v, label: v })),
            side,
            (v) => {
              side = v;
            },
            `monitor:${keyId(p.key)}:lado`
          ),
          acao(
            "Encostar no rascunho",
            () => tentar(() => aoMudar(encaixarMonitor(rascunho, p.key, target.key, side))),
            `monitor:${keyId(p.key)}:encaixar`
          )
        );
      }
      controls.append(fieldset);
    }
    root.append(controls);
  }
  const outside = (layout.organization?.positions ?? []).filter(
    (p) => !rascunho?.positions.some((q) => mesmo(q.key, p.key))
  );
  if (outside.length)
    root.append(
      h("h3", null, "Telas salvas fora do par selecionado"),
      h(
        "ul",
        null,
        outside.map((p) =>
          h(
            "li",
            null,
            `${nomePeer(layout, p.key.peer)} · UID ${p.key.uid}, X ${p.x}, Y ${p.y}. Não selecionada; preservada.`
          )
        )
      )
    );
  root.append(erroLocal);
  // A tela hospedeira mantém as regiões vivas montadas, fora deste editor repintável.
  if (erro) aoAnunciar("erro", erroTexto(erro), erro);
  else if (!ocupado && previa) aoAnunciar("status", `Prévia disponível: ${previa.effective_links.length} passagens; ${previa.preserved_link_ids.length} ligações preservadas; ${previa.replaced_link_ids.length} substituídas; ${previa.unresolved_link_ids.length} sem identidade resolvida.`, previa);
  else if (!ocupado && resultado) aoAnunciar("status",
    (resultado.activation === "unconfirmed" ? "Organização salva, ativação não confirmada" : "Organização salva e ativação confirmada pelo agente. A travessia física ainda precisa ser testada.") +
    (resultado.durability === "unconfirmed" ? " A durabilidade da gravação também não foi confirmada." : ""), resultado);
  else if (layout.inventory_problem === "identity_ambiguous") aoAnunciar("status", "Tela não identificada com segurança", layout.inventory_problem);
  return root;
}
export function ofertaMonitores({ layout, ocupado = false, aoResponder }) {
  const offer = layout.pending_monitor_offer;
  if (!offer) return null;
  const rotulo = (key) =>
    layout.inventories
      ?.find((v) => v.stamp.peer === key.peer)
      ?.inventory.monitors.find((m) => m.uid === key.uid)?.label ?? `UID ${key.uid}`;
  const positions = offer.positions.map((p) =>
    h("li", null, `${nomePeer(layout, p.key.peer)} · ${rotulo(p.key)}: X ${p.x}, Y ${p.y}`)
  );
  const respostaBase = {
    from_peer_id: offer.from_peer_id,
    connection: offer.connection,
    rev: offer.rev,
    base_rev: layout.rev
  };
  const responder = (accept) => {
    if (!ocupado) return aoResponder({ ...respostaBase, accept });
  };
  return h(
    "section",
    { class: "aviso", "aria-label": "Oferta de organização" },
    h("h2", null, `${offer.from_name} ofereceu uma organização`),
    h("ul", null, positions),
    h(
      "ul",
      null,
      offer.links.map((l) => h("li", null, `${rotulo(l.a.key)} ↔ ${rotulo(l.b.key)}`))
    ),
    h(
      "p",
      null,
      `${new Set([...(layout.links ?? []), ...(layout.organization?.links ?? [])].map(l => l.id)).size} ligações atuais serão substituídas. ${offer.links.length} passagens por monitor e ${offer.legacy_links.length} avançadas. Aceitar substitui as posições e ligações pelo mapa oferecido; preserva opções e atalhos locais. Ligações sem observação válida continuam inativas.`
    ),
    h(
      "div",
      { class: "linha" },
      botao({
        rotulo: "Recusar organização",
        dataset: { focusKey: "monitores-oferta-recusar" },
        "aria-disabled": String(ocupado),
        aoClicar: () => responder(false)
      }),
      botao({
        rotulo: "Aceitar organização",
        dataset: { focusKey: "monitores-oferta-aceitar" },
        "aria-disabled": String(ocupado),
        aoClicar: () => responder(true)
      })
    )
  );
}
export function recuperacaoMonitores({ layout, aoRestaurar }) {
  return h(
    "section",
    { class: "monitores__recuperacao" },
    h("h3", null, "Recuperar layout anterior"),
    h(
      "p",
      null,
      "A versão v0.4.0 não lê layouts v2. Restaurar usa o backup anterior e substitui o mapa, sem copiar identidade, pares ou permissões."
    ),
    h(
      "p",
      null,
      layout.v1_backup_available
        ? "Backup v1 disponível nesta consulta. Será conferido novamente ao restaurar."
        : "Sem backup v1 válido disponível."
    ),
    botao({
      rotulo: "Restaurar layout v1",
      desabilitado: !layout.organization || !layout.v1_backup_available,
      dataset: { focusKey: "monitores-restaurar" },
      aoClicar: () => {
        const { promessa } = abrirDialogo({
          titulo: "Restaurar o layout anterior?",
          conteudo: h(
            "p",
            null,
            "As posições e passagens atuais serão substituídas pelo backup. Esta ação não altera identidade nem permissões."
          ),
          acoes: [
            { rotulo: "Cancelar", valor: "cancelar", foco: true },
            {
              rotulo: "Restaurar layout v1",
              valor: "restaurar",
              variante: "perigo",
              aoClicar: () => aoRestaurar({ base_rev: layout.rev })
            }
          ]
        });
        return promessa;
      }
    })
  );
}
