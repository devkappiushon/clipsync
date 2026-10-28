// Tela 3 · Pareamento: Aceitar (endereço, porta e 2 minutos) ou Conectar (endereço:porta); depois o código
// de 6 dígitos nas duas máquinas, as permissões iniciais (tudo desligado) e o resultado pelo evento pair_result.

import { api } from "../api.js";
import { aviso, cabecalho, estadoErro } from "../areas.js";
import { botao, campo, cartao, confirmar, esqueleto, interruptor, segmentado, selo } from "../componentes.js";
import { h, trocar } from "../dom.js";
import { recurso } from "../estado.js";
import { INDISPONIVEL, mensagemDe } from "../formatar.js";

const JANELA_S = 120;

const RESULTADOS = {
  paired: null,
  declined_local: "Você cancelou: os códigos eram diferentes.",
  declined_peer: "A outra máquina recusou o pareamento.",
  failed: "O pareamento falhou.",
  expired: "O tempo acabou antes da confirmação.",
  cancelled: "Pareamento cancelado.",
};

// Uma frase só por desfecho. Recusa e cancelamento têm texto próprio, porque o `reason` do agente só repete o mesmo
// fato; falha e tempo esgotado trazem a causa no `reason`, que vale mais que o texto fixo.
const COM_CAUSA = new Set(["failed", "expired"]);

function fraseDoResultado({ outcome, reason }) {
  const causa = typeof reason === "string" ? reason.trim() : "";
  const frase = COM_CAUSA.has(outcome) && causa ? causa : (RESULTADOS[outcome] ?? "O pareamento terminou sem sucesso.");
  const maiuscula = frase.charAt(0).toLocaleUpperCase("pt-BR") + frase.slice(1);
  return /[.!?]$/.test(maiuscula) ? maiuscula : `${maiuscula}.`;
}

function rotuloModulos({ clipboard, input }) {
  const partes = [clipboard && "clipboard", input && "teclado e mouse"].filter(Boolean);
  return partes.length > 0 ? partes.join(", ") : "nenhum";
}

function formatarRestante(expiresUnixMs) {
  const restante = Math.max(0, Math.round((expiresUnixMs - Date.now()) / 1000));
  return `${Math.floor(restante / 60)}:${String(restante % 60).padStart(2, "0")}`;
}

export default function montar({ ir, assinar }) {
  let estado = null; // pair_state do agente
  let modo = "aceitar";
  let enderecoDigitado = "";
  let erroEndereco = null;
  let erroGeral = null;
  let resultado = null; // último pair_result, enquanto não começa outro
  let grants = { clipboard_in: false, clipboard_out: false, input_in: false };
  let confiancaTotal = false;
  let disponiveis = { clipboard: true, input: true };
  let relogio = null;
  let saiu = false;

  // Duas colunas: à esquerda o passo atual, à direita as permissões iniciais (liberadas depois do código).
  const corpo = h("div", { class: "colunas pareamento__colunas" });
  const botaoCabecalho = botao({ rotulo: "Cancelar pareamento", icone: "fechar", hidden: true, aoClicar: () => cancelar() });
  const no = h(
    "div",
    { class: "pareamento" },
    cabecalho({
      titulo: "Pareamento",
      descricao: "Confirme o mesmo código nas duas máquinas. O código vem da conexão, não da rede.",
      acoes: [botaoCabecalho],
    }),
    corpo,
  );

  function pararRelogio() {
    clearInterval(relogio);
    relogio = null;
  }

  function ligarRelogio(elemento, expiresUnixMs) {
    pararRelogio();
    elemento.textContent = formatarRestante(expiresUnixMs);
    relogio = setInterval(() => {
      elemento.textContent = formatarRestante(expiresUnixMs);
    }, 1000);
  }

  async function chamar(fn) {
    erroGeral = null;
    try {
      await fn();
    } catch (erro) {
      erroGeral = mensagemDe(erro);
    }
    pintar();
  }

  async function carregar() {
    corpo.classList.add("colunas--unica");
    trocar(corpo, esqueleto({ linhas: 4 }));
    try {
      estado = await api.pairState();
      if (estado.step === "permissions") disponiveis = disponiveisDoEstado();
    } catch (erro) {
      trocar(corpo, estadoErro(erro, carregar));
      return;
    }
    pintar();
  }

  function disponiveisDoEstado() {
    return {
      clipboard: estado.peer?.modules?.clipboard !== false,
      input: recurso("input") && estado.peer?.modules?.input !== false,
    };
  }

  /** Sair da tela no meio do pareamento fecha a porta: a tentativa não fica aberta sem ninguém olhando. */
  function abandonar(attemptId) {
    if (attemptId == null) return;
    api.pairCancel({ attempt_id: attemptId }).catch((erro) => console.warn("não foi possível cancelar o pareamento ao sair", erro));
  }

  const cancelar = () =>
    chamar(async () => {
      await api.pairCancel({ attempt_id: estado.attempt_id });
    });

  function linhaTempo() {
    const tempo = h("span", { class: "pareamento__tempo", role: "timer", "aria-label": "Tempo restante" });
    ligarRelogio(tempo, estado.expires_unix_ms);
    return tempo;
  }

  /** Abas Aceitar e Conectar: só se escolhe com o pareamento parado; depois ficam visíveis e inertes. */
  function abas() {
    const ocioso = estado.step === "idle";
    return segmentado({
      rotulo: "Modo de pareamento",
      opcoes: [
        { valor: "aceitar", rotulo: "Aceitar", desabilitada: !ocioso },
        { valor: "conectar", rotulo: "Conectar", desabilitada: !ocioso },
      ],
      valor: modo,
      aoMudar: (valor) => {
        modo = valor;
        pintar();
      },
    });
  }

  function cartaoOcioso() {
    let conteudo;
    if (modo === "aceitar") {
      conteudo = h(
        "div",
        { class: "pilha" },
        h("p", null, `Esta máquina abre uma porta e espera a outra por ${JANELA_S / 60} minutos. Na outra, escolha Conectar e digite o endereço que vai aparecer aqui.`),
        h(
          "div",
          { class: "linha" },
          botao({
            rotulo: "Aceitar pareamento",
            variante: "primario",
            icone: "avancar",
            aoClicar: () =>
              chamar(async () => {
                resultado = null;
                const r = await api.pairHostStart({ port: 0, window_secs: JANELA_S });
                if (saiu) return abandonar(r.attempt_id);
                estado = { ...estado, step: "listening", attempt_id: r.attempt_id, port: r.port, addresses: r.addresses, expires_unix_ms: r.expires_unix_ms };
              }),
          }),
        ),
      );
    } else {
      const entrada = campo({
        rotulo: "Endereço e porta da outra máquina",
        tipo: "mono",
        valor: enderecoDigitado,
        placeholder: "192.168.0.10:53122",
        ajuda: "O endereço aparece na outra máquina depois de ela escolher Aceitar.",
        erro: erroEndereco,
        aoMudar: (valor) => {
          enderecoDigitado = valor;
        },
      });
      const conectar = () =>
        chamar(async () => {
          erroEndereco = null;
          resultado = null;
          try {
            const r = await api.pairJoinStart({ addr: enderecoDigitado.trim() });
            if (saiu) return abandonar(r.attempt_id);
            estado = { ...estado, step: "connecting", attempt_id: r.attempt_id, expires_unix_ms: Date.now() + JANELA_S * 1000 };
          } catch (erro) {
            if (erro?.code === "bad_addr") {
              erroEndereco = mensagemDe(erro);
              return;
            }
            throw erro;
          }
        });
      entrada.controle.addEventListener("keydown", (evento) => {
        if (evento.key === "Enter") {
          evento.preventDefault();
          conectar();
        }
      });
      conteudo = h(
        "div",
        { class: "pilha" },
        entrada,
        h("div", { class: "linha" }, botao({ rotulo: "Conectar", variante: "primario", icone: "avancar", aoClicar: conectar })),
      );
    }
    return cartao({ filhos: [conteudo] });
  }

  /** Endereço desta máquina com o botão Copiar; o anúncio para leitor de tela vai numa região de status. */
  function enderecoCopiavel(endereco) {
    const anuncio = h("span", { class: "so-leitor", role: "status" });
    const copiar = botao({
      rotulo: "Copiar",
      variante: "discreto",
      icone: "copiar",
      "aria-label": `Copiar o endereço ${endereco}`,
      aoClicar: async () => {
        const rotulo = copiar.querySelector(".btn__rotulo");
        try {
          await navigator.clipboard.writeText(endereco);
          rotulo.textContent = "Copiado";
          anuncio.textContent = "Endereço copiado";
        } catch {
          rotulo.textContent = "Não copiou";
          anuncio.textContent = "Não foi possível copiar o endereço";
        }
        setTimeout(() => {
          rotulo.textContent = "Copiar";
          anuncio.textContent = "";
        }, 2000);
      },
    });
    return h("li", { class: "pareamento__endereco" }, h("span", { class: "mono" }, endereco), copiar, anuncio);
  }

  function cartaoEscutando() {
    return h(
      "section",
      { class: "cartao" },
      h("div", { class: "cartao__cabecalho" }, h("h2", null, "Aguardando a outra máquina"), selo("acento", "escutando")),
      h("p", null, "Na outra máquina, escolha Conectar e digite este endereço:"),
      h("ul", { class: "pareamento__enderecos" }, (estado.addresses ?? []).map(enderecoCopiavel)),
      h("p", { class: "pareamento__restante" }, h("span", { class: "nota" }, "Tempo restante para conectar"), linhaTempo()),
    );
  }

  function cartaoConectando() {
    return cartao({
      filhos: [h("p", { class: "pareamento__passo" }, h("span", { class: "girador", "aria-hidden": "true" }), `Conectando a ${enderecoDigitado.trim() || "outra máquina"}.`)],
    });
  }

  function cartaoCodigo() {
    const par = estado.peer;
    return h(
      "section",
      { class: "cartao pareamento__confirmacao" },
      h("h2", null, "Código de confirmação"),
      par &&
        h(
          "p",
          { class: "pareamento__par" },
          "Par: ",
          h("strong", null, par.name),
          " ",
          h("span", { class: "mono muted" }, par.short),
          h("span", { class: "muted" }, ` · módulos: ${rotuloModulos(par.modules ?? {})}`),
        ),
      h("p", { class: "pareamento__codigo", "aria-live": "polite", "aria-label": `Código ${estado.code}` }, estado.code),
      h("p", { class: "muted" }, "Confirme que a outra máquina mostra o mesmo código. Se for diferente, alguém pode estar no meio da conexão."),
      h(
        "div",
        { class: "linha pareamento__decisao" },
        botao({
          rotulo: "São iguais",
          variante: "primario",
          icone: "verificar",
          aoClicar: () =>
            chamar(async () => {
              const r = await api.pairCodeMatches({ attempt_id: estado.attempt_id });
              estado = { ...estado, step: r.step };
              disponiveis = r.available;
              grants = { ...r.defaults };
              confiancaTotal = false;
            }),
        }),
        botao({ rotulo: "São diferentes, cancelar", variante: "perigo", icone: "fechar", aoClicar: () => cancelar() }),
      ),
      h("p", { class: "nota" }, "Tempo restante: ", linhaTempo()),
    );
  }

  function cartaoConfirmado() {
    return cartao({
      titulo: "Código confirmado",
      filhos: [h("p", null, `Escolha, no painel de permissões, o que ${(estado.peer ?? { name: "a outra máquina" }).name} pode fazer com esta máquina. Depois, clique em Concluir.`)],
    });
  }

  function cartaoAguardandoPar() {
    return cartao({
      titulo: "Aguardando a outra máquina",
      filhos: [
        h("p", { class: "pareamento__passo" }, h("span", { class: "girador", "aria-hidden": "true" }), "Esperando a outra máquina confirmar as permissões dela."),
        h("p", { class: "muted" }, "Sem decisão em 2 minutos, o pareamento é recusado nos dois lados."),
      ],
    });
  }

  /** Painel da direita: sempre visível, mas só se mexe nele depois que o código foi confirmado. */
  function painelPermissoes() {
    const ativo = estado.step === "permissions";
    const par = estado.peer ?? { name: "a outra máquina" };
    const motivoClipboard = disponiveis.clipboard ? null : "Um dos lados não tem o módulo de clipboard.";
    const motivoInput = !recurso("input") ? INDISPONIVEL : disponiveis.input ? null : "O par não tem o módulo de teclado e mouse.";
    const nota = ativo ? null : estado.step === "waiting_peer" ? "Suas escolhas foram enviadas." : "Libera depois de confirmar o código.";
    return h(
      "section",
      { class: "cartao pareamento__permissoes" },
      h("h2", null, "Permissões iniciais"),
      h("p", { class: "muted" }, `Tudo desligado por padrão. Ligue só o que ${par.name} pode fazer aqui. Dá para mudar depois em Dispositivos.`),
      nota && h("p", { class: "nota", role: "status" }, nota),
      h(
        "div",
        { class: "pilha" },
        h("h3", { class: "pareamento__grupo" }, "Área de transferência"),
        interruptor({
          rotulo: "Pode me enviar clipboard",
          descricao: motivoClipboard ?? "O texto que ele copiar chega à caixa desta máquina.",
          marcado: grants.clipboard_in,
          desabilitado: !ativo || motivoClipboard !== null,
          aoMudar: (valor) => {
            grants.clipboard_in = valor;
          },
        }),
        interruptor({
          rotulo: "Pode receber meu clipboard",
          descricao: motivoClipboard ?? "O texto que você copiar aqui vai para ele.",
          marcado: grants.clipboard_out,
          desabilitado: !ativo || motivoClipboard !== null,
          aoMudar: (valor) => {
            grants.clipboard_out = valor;
          },
        }),
        h("h3", { class: "pareamento__grupo pareamento__grupo--separado" }, "Teclado e mouse"),
        interruptor({
          rotulo: "Pode controlar meu teclado e mouse",
          descricao: motivoInput ?? "Confiança total: ele pode digitar e clicar qualquer coisa aqui enquanto estiver autorizado.",
          marcado: grants.input_in,
          desabilitado: !ativo || motivoInput !== null,
          aoMudar: async (valor) => {
            if (!valor) {
              grants.input_in = false;
              confiancaTotal = false;
              return;
            }
            const ok = await confirmar({
              titulo: `Deixar ${par.name} controlar esta máquina?`,
              mensagem: "Confiança total: esse dispositivo poderá digitar e clicar qualquer coisa aqui enquanto estiver autorizado. Você pode desligar quando quiser em Dispositivos.",
              rotuloConfirmar: "Conceder controle",
            });
            if (!ok) throw new Error("cancelado");
            grants.input_in = true;
            confiancaTotal = true;
          },
        }),
        interruptor({ rotulo: "É controlada por esta máquina", descricao: INDISPONIVEL, marcado: false, desabilitado: true }),
      ),
      h(
        "div",
        { class: "pareamento__concluir" },
        botao({
          rotulo: "Concluir",
          variante: "primario",
          icone: "verificar",
          desabilitado: !ativo,
          aoClicar: () =>
            chamar(async () => {
              const r = await api.pairFinish({ attempt_id: estado.attempt_id, grants: { ...grants }, acknowledge_full_trust: confiancaTotal });
              estado = { ...estado, step: r.step };
            }),
        }),
      ),
    );
  }

  function painelResultado() {
    if (resultado.outcome === "paired") {
      const d = resultado.device;
      return [
        aviso("info", selo("ok", "pareado"), h("p", null, d ? `${d.name} agora faz parte da malha.` : "Dispositivo pareado.")),
        d &&
          h(
            "dl",
            { class: "kv" },
            h("dt", null, "Nome"),
            h("dd", null, d.name, " ", h("span", { class: "mono muted" }, d.short)),
            h("dt", null, "Endereço"),
            h("dd", { class: "mono" }, d.addr ?? "sem endereço"),
            h("dt", null, "Permissões concedidas"),
            h("dd", null, rotuloGrants(d.perms)),
          ),
        h(
          "div",
          { class: "linha" },
          botao({ rotulo: "Ir para Dispositivos", variante: "primario", icone: "dispositivos", aoClicar: () => ir("dispositivos") }),
          botao({
            rotulo: "Parear outro",
            icone: "mais",
            aoClicar: () => {
              resultado = null;
              pintar();
            },
          }),
        ),
      ];
    }
    return [
      aviso("erro", h("p", null, fraseDoResultado(resultado))),
      h(
        "div",
        { class: "linha" },
        botao({
          rotulo: "Tentar de novo",
          variante: "primario",
          icone: "atualizar",
          aoClicar: () => {
            resultado = null;
            pintar();
          },
        }),
      ),
    ];
  }

  function rotuloGrants({ clipboard_in, clipboard_out, input_in }) {
    const partes = [clipboard_in && "envia clipboard", clipboard_out && "recebe clipboard", input_in && "controla esta máquina"].filter(Boolean);
    return partes.length > 0 ? partes.join(", ") : "nenhuma";
  }

  function pintar() {
    pararRelogio();
    const aviso_ = erroGeral && aviso("erro", h("p", null, erroGeral));
    if (resultado) {
      botaoCabecalho.hidden = true;
      corpo.classList.add("colunas--unica");
      trocar(corpo, h("div", { class: "pilha" }, aviso_, cartao({ filhos: painelResultado() })));
      return;
    }
    botaoCabecalho.hidden = estado.step === "idle";
    corpo.classList.remove("colunas--unica");
    const passos = {
      listening: cartaoEscutando,
      connecting: cartaoConectando,
      code: cartaoCodigo,
      permissions: cartaoConfirmado,
      waiting_peer: cartaoAguardandoPar,
    };
    trocar(corpo, h("div", { class: "pilha" }, aviso_, abas(), (passos[estado.step] ?? cartaoOcioso)()), painelPermissoes());
  }

  const cancelarEventos = [
    assinar("pair_code_ready", (data) => {
      if (!estado) return;
      estado = { ...estado, step: "code", attempt_id: data.attempt_id, code: data.code, peer: data.peer };
      resultado = null;
      pintar();
    }),
    assinar("pair_result", (data) => {
      if (!estado) return;
      resultado = data;
      estado = { ...estado, step: "idle", attempt_id: null, code: null, peer: null };
      pintar();
    }),
    assinar("resync", () => carregar()),
  ];

  carregar();

  return {
    no,
    desmontar: () => {
      saiu = true;
      pararRelogio();
      // Depois de "Concluir" (waiting_peer) a decisão já é sua e fica com a outra máquina; antes disso, sair cancela.
      if (["listening", "connecting", "code", "permissions"].includes(estado?.step)) abandonar(estado.attempt_id);
      for (const fn of cancelarEventos) fn();
    },
  };
}
