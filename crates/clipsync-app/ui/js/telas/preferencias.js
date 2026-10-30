// Preferências: módulos em rascunho com aplicação explícita; demais escolhas salvas ao mudar.
// Atalhos descrevem condições observadas, sem presumir que o sistema registrou a combinação.

import { criarAnunciador, guardarFoco, restaurarFoco, ocupar } from "../acessibilidade.js";
import { api } from "../api.js";
import { area, cabecalho, indisponivel } from "../areas.js";
import { botao, cartao, interruptor, segmentado, tabela, tecla } from "../componentes.js";
import { h, trocar } from "../dom.js";
import { atualizarApp, obterApp, observarApp, recurso, pausaObservada } from "../estado.js";
import { INDISPONIVEL, mensagemDe } from "../formatar.js";

const EVENTOS = ["prefs_changed", "resync"];

const NOTIFICACOES = [
  { chave: "item_received", rotulo: "Item recebido", descricao: "Mostra só a origem. O conteúdo copiado nunca aparece na notificação." },
  { chave: "device_connected", rotulo: "Dispositivo conectado", descricao: "Avisa quando um par pareado entra na malha." },
  { chave: "block", rotulo: "Bloqueio", descricao: "Avisa quando o agente recusa algo, com o motivo e sem conteúdo." },
];

const TEMAS = [
  { valor: "system", rotulo: "Sistema" },
  { valor: "light", rotulo: "Claro" },
  { valor: "dark", rotulo: "Escuro" },
];

const iguais = (a, b) => a?.input === b?.input && a?.clipboard === b?.clipboard;
const completo = (m) => typeof m?.input === "boolean" && typeof m?.clipboard === "boolean";

/** Rascunho local; somente a resposta e a reconsulta confirmam a configuração. */
export function criarEdicaoModulos({ modules, aplicar, reconsultar }) {
  let expected = { ...modules };
  let rascunho = { ...modules };
  let ocupado = false;
  let confirmado = completo(modules);
  return {
    get rascunho() { return { ...rascunho }; },
    get ocupado() { return ocupado; },
    get confirmado() { return confirmado; },
    get alterado() { return !iguais(expected, rascunho); },
    editar(chave, valor) {
      if (ocupado || !["input", "clipboard"].includes(chave) || typeof valor !== "boolean") return false;
      rascunho[chave] = valor;
      return true;
    },
    sincronizar(modulos) {
      if (!completo(modulos)) return;
      if (!confirmado || (!ocupado && iguais(expected, rascunho))) {
        expected = { ...modulos };
        rascunho = { ...modulos };
        confirmado = true;
      }
    },
    async aplicar() {
      if (ocupado) return null;
      if (!confirmado) throw { code: "state_unknown", message: "Atualize o estado dos módulos antes de aplicar novamente." };
      if (iguais(expected, rascunho)) return { changed: false };
      if (!rascunho.input && !rascunho.clipboard) throw { code: "no_usable_module", message: "Ligue pelo menos um módulo." };
      ocupado = true;
      let resultado; let falha;
      try { resultado = await aplicar({ modules: { ...rascunho }, expected: { ...expected } }); }
      catch (erro) { falha = erro; }
      try {
        const atual = await reconsultar();
        if (!completo(atual?.modules)) throw Error("Estado dos módulos não confirmado.");
        expected = { ...atual.modules };
        rascunho = { ...atual.modules };
        confirmado = true;
      } catch (erro) {
        confirmado = false;
        falha ??= { code: "state_unknown", message: "A alteração terminou, mas não foi possível confirmar o estado. Atualize antes de aplicar novamente.", details: resultado };
      } finally { ocupado = false; }
      if (falha) throw falha;
      return resultado;
    },
  };
}

/** Disponibilidade por ação, sem presumir registro ou atividade dos hooks. */
export function condicaoDoAtalho(acao, app) {
  const nao = (motivo) => ({ disponivel: false, motivo });
  if (["open", "send_once"].includes(acao) || app?.platform !== "windows") return nao(INDISPONIVEL);
  if (!app?.modules?.input) return nao("Módulo de teclado e mouse desligado.");
  if (!["in_app", "external"].includes(app.agent)) return nao("Agente sem atividade confirmada.");
  if (pausaObservada(app) === null) return nao("Estado da pausa não confirmado. Atualize o estado do agente.");
  if (acao === "pause_all" && app.paused && app.features?.resume_hotkey_refused) return nao("Retomada pelo atalho recusada. Use Retomar tudo na barra lateral ou na bandeja.");
  if (app.paused && acao !== "pause_all") return nao("Malha pausada. Retome antes de usar esta ação.");
  if (!(app.traversal?.local_links > 0) && !(acao === "pause_all" && app.paused)) return nao("Sem ligação de saída local. Configure no Layout de telas.");
  if (acao === "go_to_device" && !app.features?.switch_device) return nao("A troca direta não está disponível nesta sessão.");
  if (!app.features?.global_shortcuts) return nao("Disponibilidade do atalho não confirmada nesta sessão.");
  return { disponivel: true, motivo: "Previsto no Windows nestas condições. O registro e a atividade do atalho não foram confirmados." };
}

/** Relata só efeitos confirmados pelo backend, inclusive em falha parcial. */
export function resumoEfeitos(relato) {
  const e = relato?.effects; const o = relato?.observed;
  if (!e) return "";
  const partes = [];
  if (e.config_saved === true) partes.push("Configuração salva.");
  else if (e.config_saved === false) partes.push("Esta operação não salvou a configuração.");
  else partes.push("Não foi possível confirmar se a configuração foi salva.");
  if (o?.agent === "external") partes.push("Há um agente externo; a aplicação nele não foi confirmada.");
  else if (o?.agent === "unknown") partes.push("O estado do agente é desconhecido.");
  else if (o?.agent === "stopped") partes.push("O agente está parado. Inicie quando quiser.");
  else if (o?.agent === "failed") partes.push("A inicialização do agente não foi confirmada.");
  if (e.pause_persisted === false) partes.push("A pausa pode ter sido aplicada em memória, mas não foi persistida.");
  else if (o?.paused === true) partes.push("A malha está pausada. Use Retomar tudo quando quiser.");
  if (e.stop === "requested") partes.push("A parada ainda não foi confirmada.");
  if (["pending", "failed"].includes(e.lock_restore)) partes.push("A trava nesta tela não foi confirmada.");
  return partes.join(" ");
}

function atalhos(prefs, app) {
  const s = prefs.shortcuts;
  return [
    { chave: "open", acao: "Abrir o ClipSync", tecla: s.open },
    { chave: "pause_all", acao: app?.paused ? "Retomar tudo" : "Pausar tudo", tecla: s.pause_all },
    { chave: "send_once", acao: "Enviar o clipboard uma vez", tecla: s.send_once },
    { chave: "lock_screen", acao: app?.screen_locked ? "Destravar desta tela" : "Travar nesta tela", tecla: s.lock_screen },
    ...s.go_to_device.map((tecla, i) => ({ chave: "go_to_device", acao: `Ir para o dispositivo ${i + 1}`, tecla })),
  ].map((a) => ({ ...a, ...condicaoDoAtalho(a.chave, app) }));
}

export default function montar({ ir, assinar }) {
  const avisoTopo = h("div");
  let atual = null;
  let edicao = null;
  let estadoAtual = obterApp();
  const anuncios = criarAnunciador();
  const mostrar = (tom, texto) => {
    trocar(avisoTopo, texto && h("div", { class: `aviso aviso--${tom}` }, h("p", null, texto)));
    if (tom === "erro") anuncios.erro(texto);
    else anuncios.status(texto);
  };

  const regiao = area({
    linhas: 6,
    buscar: async () => {
      const [prefs, app] = await Promise.all([api.prefsGet(), api.appState()]);
      return { prefs, app };
    },
    pintar: ({ prefs, app }) => {
      // Publicar somente a consulta cuja pintura a área aceitou.
      atual = prefs;
      estadoAtual = app;
      if (!edicao) edicao = criarEdicaoModulos({ modules: app.modules, aplicar: api.modulesSet, reconsultar: atualizarApp });
      else edicao.sincronizar(app.modules);
      return h(
        "div",
        { class: "preferencias__corpo" },
        h(
          "div",
          { class: "preferencias__grade" },
          h("div", { class: "preferencias__coluna" }, cartaoModulos(), cartaoAtalhos(prefs), cartaoInicio(prefs)),
          h("div", { class: "preferencias__coluna" }, cartaoNotificacoes(prefs), cartaoAparencia(prefs), cartaoIdioma()),
        ),
        rodape(prefs),
      );
    },
  });

  /** Grava e guarda o que o agente devolveu. Em erro, mostra a mensagem e relança para o controle voltar. */
  function salvar(campos) { return regiao.proteger(() => salvarAgora(campos)); }
  async function salvarAgora(campos) {
    trocar(avisoTopo); anuncios.limpar();
    try {
      const r = await api.prefsSet(campos);
      atual = r.prefs;
    } catch (erro) {
      mostrar("erro", `Não foi possível salvar: ${mensagemDe(erro)}`);
      throw erro;
    }
  }

  function cartaoModulos() {
    const rascunho = edicao.rascunho;
    const controles = [];
    const aplicar = botao({ rotulo: "Aplicar módulos", variante: "primario", dataset: { focusKey: "modulos-aplicar" },
      aoClicar: async () => {
        const foco = guardarFoco(no);
        await regiao.proteger(async () => {
          const liberacoes = controles.map(([controle]) => ocupar(controle)).filter(Boolean);
          const tarefa = edicao.aplicar();
          atualizar();
          try {
            const resposta = await tarefa;
            mostrar("info", resposta?.changed === false ? "Os módulos já estavam configurados assim." : resumoEfeitos(resposta));
          } catch (erro) {
            mostrar("erro", `${mensagemDe(erro)} ${resumoEfeitos(erro.details)} Revise o estado antes de aplicar novamente.`);
          } finally {
            for (const liberar of liberacoes) liberar();
            atualizar();
            await regiao.recarregar({ silencioso: true });
          }
        });
        restaurarFoco(foco, no, { fallback: () => [...no.querySelectorAll("[data-focus-key]")].find((alvo) => alvo.dataset.focusKey === "preferencias-modulo-clipboard") });
      },
    });
    const nenhum = h("p", { class: "nota" });
    function atualizar() {
      const r = edicao.rascunho;
      aplicar.disabled = !edicao.confirmado || !edicao.alterado || (!r.input && !r.clipboard);
      nenhum.textContent = !edicao.confirmado ? "Estado não confirmado. Atualize para revisar os módulos." : !r.input && !r.clipboard ? "Ligue pelo menos um módulo antes de aplicar." : edicao.alterado ? "As alterações ainda não foram salvas. Use Aplicar módulos para confirmar." : "Estes são os módulos configurados. Altere e aplique quando quiser.";
      for (const [controle, chave] of controles) controle.disabled = (chave === "input" && estadoAtual?.input_supported !== true);
    }
    const modulo = (chave, rotulo, descricao) => {
      const controle = interruptor({ rotulo, descricao, marcado: rascunho[chave], chaveFoco: `preferencias-modulo-${chave}`,
        desabilitado: chave === "input" && estadoAtual?.input_supported !== true,
        aoMudar: (valor) => {
          if (!edicao.editar(chave, valor)) throw { code: "busy", message: "Aguarde a aplicação dos módulos." };
          atualizar();
        },
      });
      controles.push([controle.querySelector("input"), chave]);
      return controle;
    };
    const filhos = [
      modulo("input", "Teclado e mouse", estadoAtual?.input_supported === true ? "Compartilha teclado e mouse pelas ligações configuradas no Layout de telas." : INDISPONIVEL),
      modulo("clipboard", "Área de transferência", "Envia e recebe texto dos pares autorizados."),
      nenhum,
      h("p", { class: "nota" }, "A identidade, os pares e as permissões são preservados. Se o agente deste app reiniciar, a malha ficará pausada. Retome explicitamente quando quiser."),
      h("div", { class: "linha" }, aplicar, botao({ rotulo: "Atualizar módulos", variante: "discreto", dataset: { focusKey: "modulos-atualizar" },
        aoClicar: () => regiao.recarregar(), })),
    ];
    atualizar();
    return cartao({ titulo: "Módulos", filhos });
  }

  function cartaoAtalhos(prefs) {
    const lista = atalhos(prefs, estadoAtual);
    return cartao({
      titulo: "Atalhos",
      filhos: [
        tabela({ legenda: "Atalhos e condições de uso", colunas: [
          { chave: "acao", rotulo: "Ação", render: (a) => h("span", { class: "preferencias__acao" }, a.acao, h("span", { class: "nota" }, a.motivo)) },
          { chave: "tecla", rotulo: "Atalho", render: (a) => a.disponivel ? tecla(a.tecla) : h("span", { class: "muted" }, "Indisponível") },
        ], linhas: lista, chaveLinha: (a) => `${a.chave}:${a.acao}` }),
        h("div", { class: "preferencias__alternativa" },
          h("p", { class: "nota" }, "Pausar ou Retomar tudo também está na barra lateral e na bandeja. Travar e Ir para dispositivo ficam no Layout de telas. A disponibilidade depende do estado atual."),
          botao({ rotulo: "Abrir o Layout de telas", variante: "discreto", icone: "avancar", aoClicar: () => ir("layout") })),
      ],
    });
  }

  function cartaoInicio(prefs) {
    return cartao({
      titulo: "Inicialização",
      filhos: [
        interruptor({
          rotulo: "Iniciar o ClipSync ao entrar na sessão", chaveFoco: "preferencias-iniciar",
          descricao: "Abre direto na bandeja, sem mostrar a janela. Fica desligado até você ligar.",
          marcado: prefs.autostart,
          aoMudar: (valor) => salvar({ autostart: valor }),
        }),
      ],
    });
  }

  function cartaoNotificacoes(prefs) {
    const ligado = recurso("os_notifications");
    return cartao({
      titulo: "Notificações",
      acoes: !ligado && indisponivel(),
      filhos: [
        h("p", { class: "nota" }, "Notificação do sistema nunca leva o conteúdo do clipboard."),
        h(
          "div",
          { class: "preferencias__lista" },
          NOTIFICACOES.map((n) =>
            interruptor({
              rotulo: n.rotulo, chaveFoco: `preferencias-notificacao-${n.chave}`,
              descricao: n.descricao,
              marcado: prefs.notifications[n.chave] === true,
              desabilitado: !ligado,
              aoMudar: (valor) => salvar({ notifications: { ...atual.notifications, [n.chave]: valor } }),
            }),
          ),
        ),
      ],
    });
  }

  function cartaoAparencia(prefs) {
    const tema = segmentado({
      rotulo: "Tema",
      chaveFoco: "preferencias-tema",
      valor: prefs.appearance.theme,
      opcoes: TEMAS,
      aoMudar: async (valor) => {
        tema.setAttribute("aria-busy", "true");
        try {
          await salvar({ appearance: { ...atual.appearance, theme: valor } });
        } catch (erro) {
          await regiao.recarregar({ silencioso: true });
          throw erro;
        } finally {
          tema.removeAttribute("aria-busy");
        }
      },
    });
    return cartao({
      titulo: "Aparência",
      filhos: [
        h("div", { class: "preferencias__campo" }, h("span", { class: "campo__rotulo" }, "Tema"), tema),
        interruptor({
          rotulo: "Reduzir movimento", chaveFoco: "preferencias-movimento",
          descricao: "Tira transições e animações da interface.",
          marcado: prefs.appearance.reduce_motion,
          aoMudar: (valor) => salvar({ appearance: { ...atual.appearance, reduce_motion: valor } }),
        }),
      ],
    });
  }

  function cartaoIdioma() {
    return cartao({
      titulo: "Idioma",
      filhos: [h("div", { class: "preferencias__idioma" }, h("p", null, "Português (Brasil)"), h("p", { class: "nota" }, `Outros idiomas: ${INDISPONIVEL}.`))],
    });
  }

  function rodape(prefs) {
    return h("footer", { class: "nota preferencias__rodape" }, `ClipSync ${prefs.version} · ${prefs.license} · feito por montesouza.dev`);
  }

  // O agente liga resume_hotkey_refused ao pausar e desliga ao retomar; o app_state traz o valor novo.
  let estadoAnterior = JSON.stringify(obterApp());
  const pararApp = observarApp(() => {
    const proximo = JSON.stringify(obterApp());
    if (proximo === estadoAnterior) return;
    estadoAnterior = proximo;
    regiao.recarregar({ silencioso: true });
  });
  const cancelarEventos = assinar(EVENTOS, () => regiao.recarregar({ silencioso: true }));
  const cancelar = () => {
    pararApp();
    cancelarEventos();
    regiao.desmontar();
  };

  const no = h(
    "div",
    { class: "preferencias" },
    cabecalho({ titulo: "Preferências", descricao: "Revise os módulos e aplique quando estiver pronto. As demais preferências são salvas ao mudar." }),
    anuncios.no,
    avisoTopo,
    regiao.no,
  );
  return { no, desmontar: cancelar };
}
