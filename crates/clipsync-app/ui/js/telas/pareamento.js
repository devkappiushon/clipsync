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

  const corpo = h("div", { class: "pareamento__corpo" });
  const no = h(
    "div",
    { class: "pareamento" },
    cabecalho({ titulo: "Pareamento", descricao: "Confirme o mesmo código nas duas máquinas. O código vem da conexão, não da rede." }),
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

  const cancelar = () =>
    chamar(async () => {
      await api.pairCancel({ attempt_id: estado.attempt_id });
    });

  function botaoCancelar(rotulo = "Cancelar") {
    return botao({ rotulo, icone: "fechar", aoClicar: cancelar });
  }

  function linhaTempo() {
    const tempo = h("span", { class: "pareamento__tempo", role: "timer", "aria-label": "Tempo restante" });
    ligarRelogio(tempo, estado.expires_unix_ms);
    return tempo;
  }

  function painelOcioso() {
    const seletor = segmentado({
      rotulo: "Modo de pareamento",
      opcoes: [
        { valor: "aceitar", rotulo: "Aceitar" },
        { valor: "conectar", rotulo: "Conectar" },
      ],
      valor: modo,
      aoMudar: (valor) => {
        modo = valor;
        pintar();
      },
    });

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
    return [seletor, conteudo];
  }

  function painelEsperando() {
    return [
      h(
        "dl",
        { class: "kv" },
        h("dt", null, "Endereços desta máquina"),
        h("dd", null, h("ul", { class: "pareamento__enderecos mono" }, (estado.addresses ?? []).map((a) => h("li", null, a)))),
        h("dt", null, "Porta"),
        h("dd", { class: "mono" }, String(estado.port ?? "")),
        h("dt", null, "Tempo restante"),
        h("dd", null, linhaTempo()),
      ),
      h("p", { class: "pareamento__passo" }, h("span", { class: "girador", "aria-hidden": "true" }), "Esperando a outra máquina conectar."),
      h("div", { class: "linha" }, botaoCancelar()),
    ];
  }

  function painelConectando() {
    return [
      h("p", { class: "pareamento__passo" }, h("span", { class: "girador", "aria-hidden": "true" }), `Conectando a ${enderecoDigitado.trim() || "outra máquina"}.`),
      h("div", { class: "linha" }, botaoCancelar()),
    ];
  }

  function painelCodigo() {
    const par = estado.peer;
    return [
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
      h("p", null, "Confirme que a outra máquina mostra o mesmo código."),
      h(
        "div",
        { class: "linha" },
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
        botao({ rotulo: "São diferentes, cancelar", variante: "perigo", icone: "fechar", aoClicar: cancelar }),
        h("span", { class: "nota" }, "Tempo restante: ", linhaTempo()),
      ),
    ];
  }

  function painelPermissoes() {
    const par = estado.peer ?? { name: "a outra máquina" };
    const motivoClipboard = disponiveis.clipboard ? null : "Um dos lados não tem o módulo de clipboard.";
    const motivoInput = !recurso("input") ? INDISPONIVEL : disponiveis.input ? null : "O par não tem o módulo de teclado e mouse.";
    return [
      h("h2", null, "Permissões iniciais"),
      h("p", { class: "muted" }, `Tudo desligado por padrão. Ligue só o que ${par.name} pode fazer com esta máquina; dá para mudar depois em Dispositivos.`),
      h(
        "div",
        { class: "pilha" },
        interruptor({
          rotulo: "Pode me enviar clipboard",
          descricao: motivoClipboard ?? "O texto que ele copiar chega à caixa desta máquina.",
          marcado: grants.clipboard_in,
          desabilitado: motivoClipboard !== null,
          aoMudar: (valor) => {
            grants.clipboard_in = valor;
          },
        }),
        interruptor({
          rotulo: "Pode receber meu clipboard",
          descricao: motivoClipboard ?? "O texto que você copiar aqui vai para ele.",
          marcado: grants.clipboard_out,
          desabilitado: motivoClipboard !== null,
          aoMudar: (valor) => {
            grants.clipboard_out = valor;
          },
        }),
        interruptor({
          rotulo: "Pode controlar meu teclado e mouse",
          descricao: motivoInput ?? "Confiança total: ele pode digitar e clicar qualquer coisa aqui enquanto estiver autorizado.",
          marcado: grants.input_in,
          desabilitado: motivoInput !== null,
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
      ),
      h(
        "div",
        { class: "linha" },
        botao({
          rotulo: "Concluir",
          variante: "primario",
          icone: "verificar",
          aoClicar: () =>
            chamar(async () => {
              const r = await api.pairFinish({ attempt_id: estado.attempt_id, grants: { ...grants }, acknowledge_full_trust: confiancaTotal });
              estado = { ...estado, step: r.step };
            }),
        }),
        botaoCancelar(),
      ),
    ];
  }

  function painelAguardandoPar() {
    return [
      h("p", { class: "pareamento__passo" }, h("span", { class: "girador", "aria-hidden": "true" }), "Aguardando a outra máquina confirmar as permissões dela."),
      h("p", { class: "muted" }, "Sem decisão em 2 minutos, o pareamento é recusado nos dois lados."),
      h("div", { class: "linha" }, botaoCancelar()),
    ];
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
    const texto = RESULTADOS[resultado.outcome] ?? "O pareamento terminou sem sucesso.";
    return [
      aviso("erro", h("p", null, texto, resultado.reason ? ` ${resultado.reason}` : "")),
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
    let filhos;
    if (resultado) filhos = painelResultado();
    else if (estado.step === "listening") filhos = painelEsperando();
    else if (estado.step === "connecting") filhos = painelConectando();
    else if (estado.step === "code") filhos = painelCodigo();
    else if (estado.step === "permissions") filhos = painelPermissoes();
    else if (estado.step === "waiting_peer") filhos = painelAguardandoPar();
    else filhos = painelOcioso();
    trocar(corpo, cartao({ filhos: [erroGeral && aviso("erro", h("p", null, erroGeral)), ...filhos] }));
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
      pararRelogio();
      for (const fn of cancelarEventos) fn();
    },
  };
}
