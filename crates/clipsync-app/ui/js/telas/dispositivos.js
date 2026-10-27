// Tela 2 · Dispositivos: tabela dos pares, painel do selecionado com permissões por módulo,
// revogar com nome digitado e reparear. Pausar um par só é "indisponível nesta versão" (feature peer_pause).

import { api } from "../api.js";
import { area, aviso, cabecalho, estadoVazio, indisponivel } from "../areas.js";
import { botao, cartao, confirmar, interruptor, tabela } from "../componentes.js";
import { h, trocar } from "../dom.js";
import { obterApp, recurso } from "../estado.js";
import { INDISPONIVEL, formatarMomento, mensagemDe, seloDispositivo } from "../formatar.js";

const EVENTOS = ["peer_connected", "peer_disconnected", "peer_blocked", "resync"];

function rotuloClipboard({ clipboard_in, clipboard_out }) {
  if (clipboard_in && clipboard_out) return "envia e recebe";
  if (clipboard_in) return "só envia";
  if (clipboard_out) return "só recebe";
  return "desligado";
}

function rotuloModulos({ clipboard, input }) {
  const partes = [clipboard && "clipboard", input && "teclado e mouse"].filter(Boolean);
  return partes.length > 0 ? partes.join(", ") : "nenhum";
}

function passosVazio() {
  return h(
    "ol",
    { class: "dispositivos__passos" },
    h("li", null, "Instale o ClipSync na outra máquina e abra a tela Dispositivos lá."),
    h("li", null, "Aqui, clique em Parear novo dispositivo e escolha Aceitar."),
    h("li", null, "Na outra máquina, escolha Conectar e digite o endereço mostrado aqui."),
  );
}

export default function montar({ ir, assinar }) {
  let selecionado = null;
  let dispositivos = [];
  let painelChave = null;
  const painel = h("div", { class: "dispositivos__painel" });
  const botaoParear = () => botao({ rotulo: "Parear novo dispositivo", variante: "primario", icone: "mais", aoClicar: () => ir("pareamento") });

  const lista = area({
    linhas: 4,
    buscar: () => api.devicesList(),
    pintar: (dados) => {
      dispositivos = dados.devices;
      const atual = dispositivos.find((d) => d.peer_id === selecionado) ?? null;
      selecionado = atual?.peer_id ?? null;
      // Só repinta o painel se o dispositivo mudou: preserva o foco de quem acabou de mexer num interruptor.
      const chave = JSON.stringify(atual);
      if (chave !== painelChave) {
        painelChave = chave;
        pintarPainel(atual);
      }
      return pintarTabela(dados);
    },
  });

  function pintarTabela({ agent_running, devices }) {
    const vazio = estadoVazio({ titulo: "Nenhum dispositivo pareado", texto: passosVazio(), acao: botaoParear() });
    if (devices.length === 0) return vazio;

    const grade = tabela({
      legenda: "Dispositivos pareados",
      colunas: [
        {
          chave: "name",
          rotulo: "Nome",
          render: (d) => h("span", { class: "dispositivos__nome" }, h("strong", null, d.name), h("span", { class: "mono muted" }, d.short)),
        },
        { chave: "state", rotulo: "Estado", render: (d) => seloDispositivo(d.state) },
        { chave: "addr", rotulo: "Endereço", tipo: "mono", render: (d) => d.addr ?? h("span", { class: "muted" }, "sem endereço") },
        { chave: "clipboard", rotulo: "Clipboard", render: (d) => rotuloClipboard(d.perms) },
        { chave: "controle", rotulo: "Controle", render: (d) => (d.perms.input_in ? "controla" : "não controla") },
      ],
      linhas: devices,
      chaveLinha: (d) => d.peer_id,
      selecionada: selecionado,
      aoSelecionar: (d) => {
        selecionado = d.peer_id;
        for (const tr of grade.querySelectorAll("tbody tr")) tr.setAttribute("aria-selected", String(tr.dataset.chave === selecionado));
        painelChave = JSON.stringify(d);
        pintarPainel(d);
        painel.scrollIntoView({ block: "nearest" });
      },
    });

    return h(
      "div",
      { class: "pilha" },
      !agent_running && aviso("aviso", h("p", null, "O agente está parado: a lista vem do arquivo de pares e todos aparecem offline.")),
      cartao({ plano: true, filhos: [grade] }),
    );
  }

  function pintarPainel(dispositivo) {
    if (!dispositivo) {
      trocar(painel);
      return;
    }
    const app = obterApp();
    const erroPainel = h("p", { class: "campo__erro", role: "alert", hidden: true });
    const mostrarErro = (erro) => {
      erroPainel.textContent = mensagemDe(erro);
      erroPainel.hidden = false;
    };

    async function gravarPermissoes(mudanca, confirmacao = false) {
      erroPainel.hidden = true;
      const perms = { ...dispositivo.perms, ...mudanca };
      try {
        const { device } = await api.deviceSetPerms({ peer_id: dispositivo.peer_id, perms, acknowledge_full_trust: confirmacao });
        dispositivo.perms = device.perms;
        lista.recarregar({ silencioso: true });
      } catch (erro) {
        mostrarErro(erro);
        throw erro;
      }
    }

    const clipboardLocal = app?.modules?.clipboard === true;
    const clipboardRemoto = dispositivo.remote_modules.clipboard === true;
    const inputLocal = recurso("input");
    const inputRemoto = dispositivo.remote_modules.input === true;
    const motivoClipboard = !clipboardLocal ? "O módulo de clipboard está desligado nesta máquina." : !clipboardRemoto ? "O par não tem o módulo de clipboard." : null;
    const motivoInput = !inputLocal ? INDISPONIVEL : !inputRemoto ? "O par não tem o módulo de teclado e mouse." : null;

    const permissoes = h(
      "div",
      { class: "dispositivos__permissoes" },
      interruptor({
        rotulo: "Pode me enviar clipboard",
        descricao: motivoClipboard ?? "O texto que ele copiar chega à caixa desta máquina.",
        marcado: dispositivo.perms.clipboard_in,
        desabilitado: motivoClipboard !== null,
        aoMudar: (valor) => gravarPermissoes({ clipboard_in: valor }),
      }),
      interruptor({
        rotulo: "Pode receber meu clipboard",
        descricao: motivoClipboard ?? "O texto que você copiar aqui vai para ele.",
        marcado: dispositivo.perms.clipboard_out,
        desabilitado: motivoClipboard !== null,
        aoMudar: (valor) => gravarPermissoes({ clipboard_out: valor }),
      }),
      interruptor({
        rotulo: "Pode controlar meu teclado e mouse",
        descricao: motivoInput ?? "Confiança total: ele pode digitar e clicar qualquer coisa aqui enquanto estiver autorizado.",
        marcado: dispositivo.perms.input_in,
        desabilitado: motivoInput !== null,
        aoMudar: async (valor) => {
          if (!valor) return gravarPermissoes({ input_in: false });
          const confirmado = await confirmar({
            titulo: `Deixar ${dispositivo.name} controlar esta máquina?`,
            mensagem: "Confiança total: esse dispositivo poderá digitar e clicar qualquer coisa aqui enquanto estiver autorizado. Você pode desligar quando quiser.",
            rotuloConfirmar: "Conceder controle",
          });
          if (!confirmado) throw new Error("cancelado");
          return gravarPermissoes({ input_in: true }, true);
        },
      }),
      interruptor({
        rotulo: "É controlada por esta máquina",
        descricao: INDISPONIVEL,
        marcado: false,
        desabilitado: true,
      }),
      erroPainel,
    );

    const notaPausa = h("span", { class: "nota", id: "dispositivos-pausa-nota" }, INDISPONIVEL);
    const acoes = h(
      "div",
      { class: "dispositivos__acoes" },
      botao({ rotulo: "Pausar este par", icone: "pausa", desabilitado: !recurso("peer_pause"), "aria-describedby": "dispositivos-pausa-nota" }),
      notaPausa,
      h("span", { class: "dispositivos__espaco" }),
      botao({ rotulo: "Reparear", icone: "atualizar", aoClicar: () => ir("pareamento") }),
      botao({
        rotulo: "Revogar",
        variante: "perigo",
        icone: "descartar",
        aoClicar: async () => {
          const ok = await confirmar({
            titulo: `Revogar ${dispositivo.name}?`,
            mensagem: "A chave deste dispositivo deixa de valer nos dois lados. Para voltar, será preciso parear de novo.",
            rotuloConfirmar: "Revogar",
            perigo: true,
            digitar: dispositivo.name,
            aoConfirmar: () => api.deviceRevoke({ peer_id: dispositivo.peer_id, typed_name: dispositivo.name }),
          });
          if (!ok) return;
          selecionado = null;
          lista.recarregar({ silencioso: true });
        },
      }),
    );

    trocar(painel, 
      cartao({
        filhos: [
          h(
            "div",
            { class: "dispositivos__titulo" },
            h("h2", null, dispositivo.name),
            h("span", { class: "mono muted" }, dispositivo.short),
            seloDispositivo(dispositivo.state),
          ),
          dispositivo.state === "blocked" && dispositivo.blocked_reason && aviso("erro", h("p", null, dispositivo.blocked_reason)),
          h(
            "dl",
            { class: "kv" },
            h("dt", null, "Endereço"),
            h("dd", { class: "mono" }, dispositivo.addr ?? "sem endereço"),
            h("dt", null, "Versão do par"),
            h("dd", { class: "mono" }, dispositivo.remote_version ?? "desconhecida"),
            h("dt", null, "Pareado em"),
            h("dd", null, formatarMomento(dispositivo.paired_at)),
            h("dt", null, "Módulos do par"),
            h("dd", null, rotuloModulos(dispositivo.remote_modules)),
            h("dt", null, "Última atividade"),
            h("dd", null, indisponivel()),
          ),
          h("h3", null, "Permissões que esta máquina concede"),
          permissoes,
          acoes,
        ],
      }),
    );
  }

  const cancelar = assinar(EVENTOS, () => lista.recarregar({ silencioso: true }));

  const no = h(
    "div",
    { class: "dispositivos" },
    cabecalho({
      titulo: "Dispositivos",
      descricao: "Quem pode enviar, quem pode receber e quem pode controlar esta máquina.",
      acoes: [botaoParear()],
    }),
    lista.no,
    painel,
  );

  return { no, desmontar: cancelar };
}
