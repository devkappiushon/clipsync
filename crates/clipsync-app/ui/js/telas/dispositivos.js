// Tela 2 · Dispositivos: tabela dos pares, painel do selecionado com permissões por módulo,
// revogar com nome digitado e reparear. Pausar um par só é "indisponível nesta versão" (feature peer_pause).

import { api } from "../api.js";
import { area, aviso, cabecalho, estadoVazio, indisponivel } from "../areas.js";
import { abrirDialogo, botao, cartao, confirmar, interruptor, tabela } from "../componentes.js";
import { criarAnunciador, guardarFoco, restaurarFoco } from "../acessibilidade.js";
import { h, trocar } from "../dom.js";
import { indicarParNoLayout, obterApp, observarApp, recurso } from "../estado.js";
import { INDISPONIVEL, formatarMomento, mensagemDe, seloDispositivo } from "../formatar.js";
import { motivoInputIndisponivel } from "../disponibilidade.js";

import { resumoTravessia } from "../travessia-vista.js";

const EVENTOS = ["agent_state", "peer_connected", "peer_disconnected", "peer_blocked", "resync"];

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
  const anuncios = criarAnunciador();
  const travessia = h("div");
  const pintarTravessia = () => trocar(travessia, resumoTravessia(obterApp(), { peerId: selecionado, peerNome: dispositivos.find((d) => d.peer_id === selecionado)?.name, ir, configurar: () => { indicarParNoLayout(selecionado); ir("layout"); } }));
  const painel = h("div", { class: "dispositivos__painel" });
  // Tabela à esquerda, painel do selecionado à direita; sem dispositivo não há painel e a tabela ocupa tudo.
  const colunas = h("div", { class: "colunas dispositivos__colunas" });
  const botaoParear = () => botao({ rotulo: "Parear novo dispositivo", dataset: { focusKey: "dispositivos-parear" }, variante: "primario", icone: "mais", aoClicar: () => ir("pareamento") });

  const lista = area({
    linhas: 4,
    rotulo: "Computadores pareados",
    fallback: () => no.querySelector("tbody tr") ?? no.querySelector("button"),
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

  function pintarTabela({ devices }) {
    const vazio = estadoVazio({ titulo: "Nenhum dispositivo pareado", texto: passosVazio(), acao: botaoParear() });
    if (devices.length === 0) return vazio;

    const grade = tabela({
      legenda: "Dispositivos pareados",
      colunas: [
        {
          chave: "name",
          rotulo: "Nome",
          render: (d) => h("span", { class: "dispositivos__nome" }, h("strong", null, d.name), h("span", { class: "mono muted" }, d.short), obterApp()?.traversal?.peers?.find((p) => p.peer_id === d.peer_id)?.protocol_warning && h("span", { class: "selo selo--aviso" }, "protocolo antigo")),
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
              h("span", null, `controle deste computador: ${d.perms.input_in ? "permitido" : "não permitido"}`),
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

    for (const tr of grade.querySelectorAll("tbody tr")) tr.dataset.focusKey = `dispositivo:${tr.dataset.chave}`;
    return h(
      "div",
      { class: "pilha" },
      cartao({ plano: true, filhos: [grade] }),
    );
  }

  function pintarPainel(dispositivo) {
    if (!dispositivo) {
      trocar(painel);
      return;
    }
    const app = obterApp();
    const erroPainel = h("p", { class: "campo__erro", hidden: true });
    const mostrarErro = (erro) => {
      erroPainel.textContent = mensagemDe(erro);
      erroPainel.hidden = false;
      anuncios.erro(erroPainel.textContent);
    };

    async function gravarPermissoes(mudanca, confirmacao = false) {
      if (dispositivos.find((d) => d.peer_id === dispositivo.peer_id)?.state === "blocked") throw new Error("Repareie o computador para editar permissões.");
      const foco = guardarFoco(no);
      return lista.proteger(async () => {
        erroPainel.hidden = true;
        const perms = { ...dispositivo.perms, ...mudanca };
      try {
        const { device } = await api.deviceSetPerms({ peer_id: dispositivo.peer_id, perms, acknowledge_full_trust: confirmacao });
        dispositivo.perms = device.perms;
        if (mudanca.input_in === true) orientacaoInput = dispositivo.peer_id;
        anuncios.status("Permissões atualizadas.");
        await lista.recarregar({ silencioso: true });
      } catch (erro) {
        mostrarErro(erro);
        throw erro;
      } finally {
        queueMicrotask(() => restaurarFoco(foco, no));
      }
      });
    }

    const clipboardLocal = app?.modules?.clipboard === true;
    const clipboardRemoto = dispositivo.remote_modules.clipboard === true;
    const inputLocal = recurso("input");
    const inputRemoto = dispositivo.remote_modules.input === true;
    const bloqueado = dispositivo.state === "blocked" ? "Computador bloqueado. Use Reparear para renovar a confiança." : null;
    const motivoClipboard = bloqueado ?? (!clipboardLocal ? "O módulo de clipboard está desligado nesta máquina." : !clipboardRemoto ? `Não foi possível confirmar os recursos de ${dispositivo.name}.` : null);
    const motivoInput = bloqueado ?? (!inputLocal ? motivoInputIndisponivel(app) : !inputRemoto ? `Não foi possível confirmar os recursos de ${dispositivo.name}.` : null);

    const permissoes = h(
      "div",
      { class: "dispositivos__permissoes" },
      interruptor({
        rotulo: "Pode me enviar clipboard", chaveFoco: `permissao:${dispositivo.peer_id}:clipboard-in`,
        descricao: motivoClipboard,
        marcado: dispositivo.perms.clipboard_in,
        desabilitado: motivoClipboard !== null,
        aoMudar: (valor) => gravarPermissoes({ clipboard_in: valor }),
      }),
      interruptor({
        rotulo: "Pode receber meu clipboard", chaveFoco: `permissao:${dispositivo.peer_id}:clipboard-out`,
        descricao: motivoClipboard,
        marcado: dispositivo.perms.clipboard_out,
        desabilitado: motivoClipboard !== null,
        aoMudar: (valor) => gravarPermissoes({ clipboard_out: valor }),
      }),
      interruptor({
        rotulo: `${dispositivo.name} pode controlar o teclado e mouse deste computador`, chaveFoco: `permissao:${dispositivo.peer_id}:input-in`,
        descricao: motivoInput,
        marcado: dispositivo.perms.input_in,
        desabilitado: motivoInput !== null,
        aoMudar: async (valor) => {
          if (!valor) return gravarPermissoes({ input_in: false });
          const { promessa } = abrirDialogo({
            titulo: `Deixar ${dispositivo.name} controlar este computador?`,
            conteudo: h("p", null, "Confiança total: esse computador poderá digitar e clicar qualquer coisa aqui enquanto estiver autorizado. Conceda no computador que receberá o controle."),
            raizRetorno: no,
            acoes: [
              { rotulo: "Cancelar", valor: "cancelar", foco: true },
              { rotulo: "Conceder controle", valor: "conceder", variante: "primario" },
            ],
          });
          const confirmado = (await promessa) === "conceder";
          if (!confirmado) throw new Error("cancelado");
          return gravarPermissoes({ input_in: true }, true);
        },
      }),
      h("div", { class: "opcao" }, h("div", { class: "opcao__texto" },
        h("p", { class: "opcao__rotulo" }, `Controlar ${dispositivo.name} a partir deste computador`),
        h("p", { class: "opcao__descricao" }, `Para controlar ${dispositivo.name} a partir deste computador, autorize este computador na tela Dispositivos de ${dispositivo.name}. O ClipSync não informa aqui se essa autorização já foi concedida.`),
      )),
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
        rotulo: "Revogar", dataset: { focusKey: `revogar:${dispositivo.peer_id}` },
        variante: "perigo",
        icone: "descartar",
        aoClicar: async () => {
          const ok = await confirmar({
            titulo: `Revogar ${dispositivo.name}?`,
            mensagem: "A chave deste dispositivo deixa de valer nos dois lados. Para voltar, será preciso parear de novo.",
            rotuloConfirmar: "Revogar",
            perigo: true,
            digitar: dispositivo.name,
            raizRetorno: no,
            fallback: () => no.querySelector("tbody tr") ?? no.querySelector("button"),
            aoConfirmar: async () => {
              await api.deviceRevoke({ peer_id: dispositivo.peer_id, typed_name: dispositivo.name });
              selecionado = null;
              await lista.recarregar({ silencioso: true });
              anuncios.status("Computador revogado.");
            },
          });
          if (!ok) return;
        },
      }),
      h("p", { class: "nota" }, "Revogar pede o nome da máquina digitado para confirmar e apaga a chave dela daqui."),
    );

    const focoPainel = guardarFoco(painel);
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
            { class: "dispositivos__detalhes", dataset: { focusKey: `detalhes:${dispositivo.peer_id}` } },
            h("summary", null, "Detalhes do par"),
            h(
              "dl",
              { class: "kv" },
              h("dt", null, "Endereço"),
              h("dd", { class: "mono" }, dispositivo.addr ?? "sem endereço"),
              h("dt", null, "Protocolo do par"),
              h("dd", { class: "mono" }, dispositivo.remote_version ?? "desconhecida"),
              obterApp()?.traversal?.peers?.find((p) => p.peer_id === dispositivo.peer_id)?.protocol_warning && h("dt", null, "Compatibilidade"),
              obterApp()?.traversal?.peers?.find((p) => p.peer_id === dispositivo.peer_id)?.protocol_warning && h("dd", null, obterApp().traversal.peers.find((p) => p.peer_id === dispositivo.peer_id).protocol_warning),
              h("dt", null, "Pareado em"),
              h("dd", null, formatarMomento(dispositivo.paired_at)),
              h("dt", null, "Módulos do par"),
              h("dd", null, rotuloModulos(dispositivo.remote_modules)),
              h("dt", null, "Última atividade"),
              h("dd", null, indisponivel()),
            ),
          ),
          orientacaoInput === dispositivo.peer_id && aviso("info", h("p", null, `Controle permitido para entrar aqui. Em ${dispositivo.name}, configure a saída do computador que tem o mouse físico.`), botao({ rotulo: "Configurar saída deste computador", aoClicar: () => { indicarParNoLayout(dispositivo.peer_id); ir("layout"); } })),
          h("h3", null, "Permissões"),
          permissoes,
          acoes,
        ],
      }),
    );
    restaurarFoco(focoPainel, no);
  }

  const cancelar = assinar(EVENTOS, () => lista.recarregar({ silencioso: true }));
  const cancelarApp = observarApp(() => {
    pintarTravessia();
    painelChave = null;
    lista.recarregar({ silencioso: true });
  });

  const no = h(
    "div",
    { class: "dispositivos" },
    cabecalho({
      titulo: "Dispositivos",
      descricao: "Quem pode enviar, quem pode receber e quem pode controlar esta máquina.",
      acoes: [botaoParear()],
    }),
    anuncios.no,
    travessia,
    colunas,
  );
  colunas.append(lista.no, painel);

  return { no, desmontar: () => { lista.desmontar(); cancelar(); cancelarApp(); } };
}
