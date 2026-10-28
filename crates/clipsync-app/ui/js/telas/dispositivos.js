// Tela 2 · Dispositivos: tabela dos pares, painel do selecionado com permissões por módulo,
// revogar com nome digitado e reparear. Pausar um par só é "indisponível nesta versão" (feature peer_pause).

import { api } from "../api.js";
import { area, aviso, cabecalho, estadoVazio, indisponivel } from "../areas.js";
import { botao, cartao, confirmar, interruptor, tabela } from "../componentes.js";
import { h, trocar } from "../dom.js";
import { indicarParNoLayout, obterApp, observarApp, recurso } from "../estado.js";
import { INDISPONIVEL, formatarMomento, mensagemDe, seloDispositivo } from "../formatar.js";

import { resumoTravessia } from "../travessia-vista.js";

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
  let orientacaoInput = null;
  const travessia = h("div");
  const pintarTravessia = () => trocar(travessia, resumoTravessia(obterApp(), { peerId: selecionado, ir, configurar: () => { indicarParNoLayout(selecionado); ir("layout"); } }));
  const painel = h("div", { class: "dispositivos__painel" });
  // Tabela à esquerda, painel do selecionado à direita; sem dispositivo não há painel e a tabela ocupa tudo.
  const colunas = h("div", { class: "colunas dispositivos__colunas" });
  const botaoParear = () => botao({ rotulo: "Parear novo dispositivo", variante: "primario", icone: "mais", aoClicar: () => ir("pareamento") });

  const lista = area({
    linhas: 4,
    buscar: () => api.devicesList(),
    pintar: (dados) => {
      dispositivos = dados.devices;
      // A primeira linha já vem selecionada.
      const atual = dispositivos.find((d) => d.peer_id === selecionado) ?? dispositivos[0] ?? null;
      selecionado = atual?.peer_id ?? null;
      colunas.classList.toggle("colunas--unica", atual === null);
      // Só repinta o painel se o dispositivo mudou: preserva o foco de quem acabou de mexer num interruptor.
      const chave = JSON.stringify(atual);
      if (chave !== painelChave) {
        painelChave = chave;
        pintarPainel(atual);
      }
      pintarTravessia();
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
        {
          chave: "permissoes",
          rotulo: "Permissões",
          render: (d) =>
            h(
              "span",
              { class: "dispositivos__permissoes-celula" },
              h("span", null, `clipboard: ${rotuloClipboard(d.perms)}`),
              h("span", null, `controle: ${d.perms.input_in ? "controla" : "não controla"}`),
            ),
        },
      ],
      linhas: devices,
      chaveLinha: (d) => d.peer_id,
      selecionada: selecionado,
      aoSelecionar: (d) => {
        selecionado = d.peer_id;
        for (const tr of grade.querySelectorAll("tbody tr")) tr.setAttribute("aria-selected", String(tr.dataset.chave === selecionado));
        painelChave = JSON.stringify(d);
        pintarPainel(d);
        pintarTravessia();
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
        if (mudanca.input_in === true) orientacaoInput = dispositivo.peer_id;
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
        descricao: motivoClipboard,
        marcado: dispositivo.perms.clipboard_in,
        desabilitado: motivoClipboard !== null,
        aoMudar: (valor) => gravarPermissoes({ clipboard_in: valor }),
      }),
      interruptor({
        rotulo: "Pode receber meu clipboard",
        descricao: motivoClipboard,
        marcado: dispositivo.perms.clipboard_out,
        desabilitado: motivoClipboard !== null,
        aoMudar: (valor) => gravarPermissoes({ clipboard_out: valor }),
      }),
      interruptor({
        rotulo: "Pode controlar meu teclado e mouse",
        descricao: motivoInput,
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
      botao({ rotulo: "Pausar par", icone: "pausa", desabilitado: !recurso("peer_pause"), "aria-describedby": "dispositivos-pausa-nota" }),
      !recurso("peer_pause") && notaPausa,
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
      h("p", { class: "nota" }, "Revogar pede o nome da máquina digitado para confirmar e apaga a chave dela daqui."),
    );

    trocar(painel, 
      cartao({
        filhos: [
          h("div", { class: "dispositivos__titulo" }, h("h2", null, dispositivo.name), seloDispositivo(dispositivo.state)),
          dispositivo.state === "blocked" && dispositivo.blocked_reason && aviso("erro", h("p", null, dispositivo.blocked_reason)),
          h(
            "div",
            { class: "dispositivos__impressao" },
            h("p", { class: "nota" }, "Impressão digital"),
            h("p", { class: "mono" }, dispositivo.short),
          ),
          h(
            "details",
            { class: "dispositivos__detalhes" },
            h("summary", null, "Detalhes do par"),
            h(
              "dl",
              { class: "kv" },
              h("dt", null, "Endereço"),
              h("dd", { class: "mono" }, dispositivo.addr ?? "sem endereço"),
              h("dt", null, "Protocolo do par"),
              h("dd", { class: "mono" }, dispositivo.remote_version ?? "desconhecida"),
              h("dt", null, "Pareado em"),
              h("dd", null, formatarMomento(dispositivo.paired_at)),
              h("dt", null, "Módulos do par"),
              h("dd", null, rotuloModulos(dispositivo.remote_modules)),
              h("dt", null, "Última atividade"),
              h("dd", null, indisponivel()),
            ),
          ),
          orientacaoInput === dispositivo.peer_id && aviso("info", h("p", null, "Controle permitido para entrar aqui. Configure a passagem no computador que tem o mouse físico."), botao({ rotulo: "Configurar passagem", aoClicar: () => { indicarParNoLayout(dispositivo.peer_id); ir("layout"); } })),
          h("h3", null, "Permissões"),
          permissoes,
          acoes,
        ],
      }),
    );
  }

  const cancelar = assinar(EVENTOS, () => lista.recarregar({ silencioso: true }));
  const cancelarApp = observarApp(pintarTravessia);

  const no = h(
    "div",
    { class: "dispositivos" },
    cabecalho({
      titulo: "Dispositivos",
      descricao: "Quem pode enviar, quem pode receber e quem pode controlar esta máquina.",
      acoes: [botaoParear()],
    }),
    travessia,
    colunas,
  );
  colunas.append(lista.no, painel);

  return { no, desmontar: () => { cancelar(); cancelarApp(); } };
}
