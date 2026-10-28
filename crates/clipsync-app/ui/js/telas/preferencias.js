// Tela 7 · Preferências: atalhos (texto, indisponíveis nesta versão), início com a sessão, notificações por tipo,
// aparência, idioma e Sobre. Toda mudança grava na hora pelo prefs_set; se falhar, o controle volta e o erro aparece.

import { api } from "../api.js";
import { area, aviso, cabecalho, indisponivel } from "../areas.js";
import { botao, cartao, interruptor, segmentado, tabela, tecla } from "../componentes.js";
import { h, trocar } from "../dom.js";
import { recurso } from "../estado.js";
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

function atalhos(prefs) {
  const s = prefs.shortcuts;
  return [
    { acao: "Abrir o ClipSync", tecla: s.open },
    { acao: "Pausar tudo", tecla: s.pause_all },
    { acao: "Enviar o clipboard uma vez", tecla: s.send_once },
    { acao: "Travar nesta tela", tecla: s.lock_screen },
    ...s.go_to_device.map((tecla, i) => ({ acao: `Ir para o dispositivo ${i + 1}`, tecla })),
  ];
}

export default function montar({ ir, assinar }) {
  const avisoTopo = h("div");
  let atual = null;

  const regiao = area({
    linhas: 6,
    buscar: async () => {
      atual = await api.prefsGet();
      return atual;
    },
    pintar: (prefs) => h("div", { class: "pilha" }, cartaoAtalhos(prefs), cartaoInicio(prefs), cartaoNotificacoes(prefs), cartaoAparencia(prefs), cartaoSobre(prefs)),
  });

  /** Grava e guarda o que o agente devolveu. Em erro, mostra a mensagem e relança para o controle voltar. */
  async function salvar(campos) {
    trocar(avisoTopo);
    try {
      const r = await api.prefsSet(campos);
      atual = r.prefs;
    } catch (erro) {
      trocar(avisoTopo, aviso("erro", h("p", null, `Não foi possível salvar: ${mensagemDe(erro)}`)));
      throw erro;
    }
  }

  function cartaoAtalhos(prefs) {
    const lista = atalhos(prefs);
    const ligado = recurso("global_shortcuts");
    return cartao({
      titulo: "Atalhos",
      acoes: !ligado && indisponivel(),
      filhos: [
        lista.length === 0
          ? h("p", { class: "muted" }, "Nenhum atalho definido.")
          : tabela({
              legenda: "Atalhos globais",
              colunas: [
                { chave: "acao", rotulo: "Ação" },
                { chave: "tecla", rotulo: "Atalho", render: (a) => tecla(a.tecla) },
                { chave: "estado", rotulo: "Estado", render: () => (ligado ? "ativo" : h("span", { class: "muted" }, INDISPONIVEL)) },
              ],
              linhas: lista,
              chaveLinha: (a) => a.acao,
            }),
        h(
          "div",
          { class: "preferencias__alternativa" },
          h("p", { class: "nota" }, "Enquanto o atalho global não existe, as mesmas ações estão em botões: Pausar tudo na barra lateral e no menu da bandeja; Travar nesta tela e Ir para dispositivo no Layout de telas."),
          botao({ rotulo: "Abrir o Layout de telas", variante: "discreto", icone: "avancar", aoClicar: () => ir("layout") }),
        ),
      ],
    });
  }

  function cartaoInicio(prefs) {
    return cartao({
      titulo: "Início com a sessão",
      filhos: [
        interruptor({
          rotulo: "Iniciar o ClipSync ao entrar na sessão",
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
              rotulo: n.rotulo,
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
      valor: prefs.appearance.theme,
      opcoes: TEMAS,
      aoMudar: async (valor) => {
        tema.setAttribute("aria-busy", "true");
        try {
          await salvar({ appearance: { ...atual.appearance, theme: valor } });
        } catch {
          await regiao.recarregar({ silencioso: true });
        } finally {
          tema.removeAttribute("aria-busy");
        }
      },
    });
    return cartao({
      titulo: "Aparência e idioma",
      filhos: [
        h("div", { class: "preferencias__campo" }, h("span", { class: "campo__rotulo" }, "Tema"), tema),
        interruptor({
          rotulo: "Reduzir movimento",
          descricao: "Tira transições e animações da interface.",
          marcado: prefs.appearance.reduce_motion,
          aoMudar: (valor) => salvar({ appearance: { ...atual.appearance, reduce_motion: valor } }),
        }),
        h(
          "dl",
          { class: "kv" },
          h("dt", null, "Idioma"),
          h("dd", null, "Português (Brasil)", h("span", { class: "nota" }, " Outros idiomas: " + INDISPONIVEL + ".")),
        ),
      ],
    });
  }

  function cartaoSobre(prefs) {
    return cartao({
      titulo: "Sobre",
      filhos: [
        h(
          "dl",
          { class: "kv" },
          h("dt", null, "Versão"),
          h("dd", { class: "mono" }, prefs.version),
          h("dt", null, "Licença"),
          h("dd", null, prefs.license),
        ),
        h("p", { class: "nota preferencias__credito" }, "feito por montesouza.dev"),
      ],
    });
  }

  const cancelar = assinar(EVENTOS, () => regiao.recarregar({ silencioso: true }));

  const no = h(
    "div",
    { class: "preferencias" },
    cabecalho({ titulo: "Preferências", descricao: "Atalhos, início com a sessão, notificações e aparência. Cada mudança vale na hora." }),
    avisoTopo,
    regiao.no,
  );
  return { no, desmontar: cancelar };
}
