// Tela 3 · Pareamento: Aceitar (endereço, porta e 2 minutos) ou Conectar (endereço:porta); depois o código
// de 6 dígitos nas duas máquinas, as permissões iniciais (tudo desligado) e o resultado pelo evento pair_result.

import { api } from "../api.js";
import { aviso, cabecalho, estadoErro } from "../areas.js";
import { abrirDialogo, botao, campo, cartao, esqueleto, interruptor, segmentado, selo } from "../componentes.js";
import { criarAnunciador, guardarFoco, interacaoAtual, restaurarFoco } from "../acessibilidade.js";
import { h, trocar } from "../dom.js";
import { indicarParNoLayout, recurso } from "../estado.js";
import { INDISPONIVEL, mensagemDe } from "../formatar.js";

const JANELA_S = 120;

// Cada reinício encerra a tentativa antiga antes de criar outra; nunca reutiliza código/grants.
export async function iniciarTentativa({ comandos, modo, endereco = "", anterior = null }) {
  if (anterior) await comandos.pairCancel({ attempt_id: anterior });
  if (modo === "aceitar") {
    const r = await comandos.pairHostStart({ port: 0, window_secs: JANELA_S });
    return { step: "listening", attempt_id: r.attempt_id, port: r.port, addresses: r.addresses, expires_unix_ms: r.expires_unix_ms };
  }
  const r = await comandos.pairJoinStart({ addr: endereco.trim() });
  return { step: "connecting", attempt_id: r.attempt_id, expires_unix_ms: Date.now() + JANELA_S * 1000 };
}

export function criarAvisosPrazo() {
  let avisou30 = false;
  let expirou = false;
  return (prazo, agora) => {
    if (expirou) return null;
    if (agora >= prazo) { expirou = true; return "expirou"; }
    if (prazo - agora <= 30000 && !avisou30) { avisou30 = true; return "30s"; }
    return null;
  };
}

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
  let ocupado = false;
  let corrigindoEndereco = false;
  let resultadoDuranteCorrecao = null;
  let tentativaDescartada = null;
  let geracaoCarga = 0;
  let consultaPendente = false;
  let focoTentativa = null;
  let avisosPrazo = criarAvisosPrazo();
  let expirou = false;
  const anuncios = criarAnunciador();

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
    anuncios.no,
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
      const avisoPrazo = avisosPrazo(expiresUnixMs, Date.now());
      if (avisoPrazo === "30s") {
        anuncios.status("Faltam 30 segundos para o pareamento expirar.");
      }
      if (avisoPrazo === "expirou" && !expirou) {
        expirou = true;
        pararRelogio();
        anuncios.erro("O tempo de pareamento acabou. Recomece para receber um código novo.");
        pintar();
      }
    }, 1000);
  }

  async function chamar(fn) {
    if (ocupado || saiu || (expirou && estado?.step !== "idle")) return;
    geracaoCarga++;
    ocupado = true;
    erroGeral = null;
    const foco = guardarFoco(no);
    const passo = estado?.step;
    try {
      await fn();
    } catch (erro) {
      erroGeral = mensagemDe(erro);
      anuncios.erro(erroGeral);
    } finally {
      geracaoCarga++;
      ocupado = false;
    }
    if (saiu) return;
    const preservaInteracao = interacaoAtual(foco, no);
    const avancar = estado?.step !== passo && preservaInteracao;
    pintar({ foco, avancar });
    if (erroEndereco) { anuncios.erro(erroEndereco); if (preservaInteracao) [...corpo.querySelectorAll("[data-focus-key]")].find((e) => e.dataset.focusKey === "pareamento-endereco")?.focus({ preventScroll: true }); }
    if (consultaPendente) { consultaPendente = false; await carregar(); }
  }

  function novaTentativa() {
    geracaoCarga++;
    grants = { clipboard_in: false, clipboard_out: false, input_in: false };
    confiancaTotal = false;
    avisosPrazo = criarAvisosPrazo();
    expirou = false;
    resultado = null;
    erroEndereco = null;
    focoTentativa = null;
    anuncios.limpar();
  }

  async function recomecar() {
    if (ocupado || saiu) return;
    geracaoCarga++;
    ocupado = true;
    const foco = guardarFoco(no);
    try {
      const proximo = await iniciarTentativa({ comandos: api, modo, endereco: enderecoDigitado, anterior: estado?.attempt_id });
      if (saiu) return abandonar(proximo.attempt_id);
      novaTentativa();
      estado = proximo;
      pintar({ foco, avancar: interacaoAtual(foco, no) });
    } catch (erro) { erroGeral = mensagemDe(erro); anuncios.erro(erroGeral); pintar(); }
    finally {
      geracaoCarga++;
      ocupado = false;
      if (consultaPendente && !saiu) { consultaPendente = false; await carregar(); }
    }
  }

  async function corrigirEndereco() {
    if (ocupado || saiu || modo !== "conectar") return;
    const minha = ++geracaoCarga;
    const foco = guardarFoco(no);
    ocupado = true;
    corrigindoEndereco = true;
    try {
      const tentativa = estado?.attempt_id;
      if (tentativa != null) await api.pairCancel({ attempt_id: tentativa });
      if (saiu || minha !== geracaoCarga) return;
      // Cancelar confirma a decisão, mas o watcher ainda pode publicar essa tentativa.
      // Só há um slot no backend; basta reter o último descarte, inclusive ao iniciar outra.
      if (tentativa != null) tentativaDescartada = tentativa;
      novaTentativa();
      erroGeral = null;
      estado = { step: "idle", attempt_id: null };
      pintar({ foco, avancar: interacaoAtual(foco, no) });
    } catch (erro) {
      if (saiu || minha !== geracaoCarga) return;
      // Se o cancelamento foi recusado, o resultado recebido continua sendo válido.
      if (resultadoDuranteCorrecao) {
        erroGeral = null;
        corrigindoEndereco = false;
        receberResultado(resultadoDuranteCorrecao);
        return;
      }
      erroGeral = mensagemDe(erro);
      anuncios.erro(erroGeral);
      pintar({ foco });
    } finally {
      geracaoCarga++;
      ocupado = false;
      corrigindoEndereco = false;
      resultadoDuranteCorrecao = null;
      if (consultaPendente && !saiu) { consultaPendente = false; await carregar(); }
    }
  }

  function botaoCorrigirEndereco() {
    return modo === "conectar" && botao({
      rotulo: "Corrigir endereço", dataset: { focusKey: "pareamento-corrigir-endereco" },
      aoClicar: corrigirEndereco,
    });
  }

  async function carregar() {
    if (saiu) return;
    if (ocupado) { consultaPendente = true; return; }
    const minha = ++geracaoCarga;
    const foco = guardarFoco(no);
    if (!estado) { corpo.classList.add("colunas--unica"); trocar(corpo, esqueleto({ linhas: 4 })); }
    try {
      const proximo = await api.pairState();
      if (saiu || minha !== geracaoCarga) return;
      if (proximo.attempt_id != null && proximo.attempt_id === tentativaDescartada) return;
      estado = proximo;
      if (estado.step === "permissions") disponiveis = disponiveisDoEstado();
    } catch (erro) {
      if (saiu || minha !== geracaoCarga) return;
      anuncios.erro(mensagemDe(erro));
      trocar(corpo, estadoErro(erro, carregar, { anunciar: false }));
      return;
    }
    pintar({ foco });
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
      rotulo: "Modo de pareamento", chaveFoco: "pareamento-modo",
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
            rotulo: "Aceitar pareamento", dataset: { focusKey: "pareamento-aceitar" },
            variante: "primario",
            icone: "avancar",
            aoClicar: () =>
              chamar(async () => {
                novaTentativa();
                const proximo = await iniciarTentativa({ comandos: api, modo: "aceitar" });
                if (saiu) return abandonar(proximo.attempt_id);
                estado = proximo;
              }),
          }),
        ),
      );
    } else {
      const entrada = campo({
        rotulo: "Endereço e porta da outra máquina", dataset: { focusKey: "pareamento-endereco" },
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
          novaTentativa();
          try {
            const proximo = await iniciarTentativa({ comandos: api, modo: "conectar", endereco: enderecoDigitado });
            if (saiu) return abandonar(proximo.attempt_id);
            estado = proximo;
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
        h("div", { class: "linha" }, botao({ rotulo: "Conectar", dataset: { focusKey: "pareamento-conectar" }, variante: "primario", icone: "avancar", aoClicar: conectar })),
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
      h("p", null, "Os dois computadores precisam se alcançar pela rede. Digite na outra máquina um destes endereços:"),
      h("ul", { class: "pareamento__enderecos" }, (estado.addresses ?? []).map(enderecoCopiavel)),
      h("p", { class: "pareamento__restante" }, h("span", { class: "nota" }, "Tempo restante para conectar"), linhaTempo()),
    );
  }

  function cartaoConectando() {
    return cartao({
      titulo: "Conectando à outra máquina",
      filhos: [linhaTempo(), h("p", { class: "pareamento__passo" }, h("span", { class: "girador", "aria-hidden": "true" }), `Conectando a ${enderecoDigitado.trim() || "outra máquina"}.`)],
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
      h("p", { class: "pareamento__codigo" }, estado.code),
      h("p", { class: "muted" }, "Confirme que a outra máquina mostra o mesmo código. Se for diferente, alguém pode estar no meio da conexão."),
      h(
        "div",
        { class: "linha pareamento__decisao" },
        botao({
          rotulo: "São iguais", dataset: { focusKey: "pareamento-confirmar-codigo" },
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
      filhos: [linhaTempo(), h("p", null, `Escolha, no painel de permissões, o que ${(estado.peer ?? { name: "a outra máquina" }).name} pode fazer com esta máquina. Depois, clique em Concluir.`)],
    });
  }

  function cartaoAguardandoPar() {
    return cartao({
      titulo: "Aguardando a outra máquina",
      filhos: [
        linhaTempo(),
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
          rotulo: "Pode me enviar clipboard", chaveFoco: "pareamento-clipboard-in",
          descricao: motivoClipboard ?? "O texto que ele copiar chega à caixa desta máquina.",
          marcado: grants.clipboard_in,
          desabilitado: !ativo || motivoClipboard !== null,
          aoMudar: (valor) => {
            grants.clipboard_in = valor;
          },
        }),
        interruptor({
          rotulo: "Pode receber meu clipboard", chaveFoco: "pareamento-clipboard-out",
          descricao: motivoClipboard ?? "O texto que você copiar aqui vai para ele.",
          marcado: grants.clipboard_out,
          desabilitado: !ativo || motivoClipboard !== null,
          aoMudar: (valor) => {
            grants.clipboard_out = valor;
          },
        }),
        h("h3", { class: "pareamento__grupo pareamento__grupo--separado" }, "Teclado e mouse"),
        interruptor({
          rotulo: `${par.name} pode controlar o teclado e mouse deste computador`, chaveFoco: "pareamento-input-in",
          descricao: motivoInput ?? "Confiança total: ele pode digitar e clicar qualquer coisa aqui enquanto estiver autorizado.",
          marcado: grants.input_in,
          desabilitado: !ativo || motivoInput !== null,
          aoMudar: async (valor) => {
            if (!valor) {
              grants.input_in = false;
              confiancaTotal = false;
              return;
            }
            const tentativa = estado.attempt_id;
            const { promessa } = abrirDialogo({
              titulo: `Deixar ${par.name} controlar este computador?`,
              conteudo: h("p", null, "Confiança total: esse computador poderá digitar e clicar qualquer coisa aqui enquanto estiver autorizado. Conceda no computador que receberá o controle."),
              raizRetorno: no,
              acoes: [
                { rotulo: "Cancelar", valor: "cancelar", foco: true },
                { rotulo: "Conceder controle", valor: "conceder", variante: "primario" },
              ],
            });
            const ok = (await promessa) === "conceder" && !expirou && estado.attempt_id === tentativa && estado.step === "permissions";
            if (!ok) throw new Error("cancelado");
            grants.input_in = true;
            confiancaTotal = true;
          },
        }),
        interruptor({ rotulo: `Controlar ${par.name} a partir deste computador`, descricao: `Autorize a entrada em ${par.name}, na tela Dispositivos dele. ${INDISPONIVEL}`, marcado: false, desabilitado: true }),
      ),
      h(
        "div",
        { class: "pareamento__concluir" },
        botao({
          rotulo: "Concluir", dataset: { focusKey: "pareamento-concluir" },
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
        h("div", { class: "aviso aviso--info" }, selo("ok", "pareado"), h("p", null, d ? `${d.name} agora faz parte da malha.` : "Dispositivo pareado.")),
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
        d?.remote_modules?.input && recurso("input") && aviso("info",
          h("p", null, d?.perms?.input_in ? `Você permitiu que ${d.name} controle este computador. Em ${d.name}, configure a saída do computador que tem o mouse físico. Você também pode configurar uma saída daqui quando usar o mouse deste computador.` : "Para passar o mouse, configure a saída no computador que tem o mouse físico. Parear não cria a passagem automaticamente."),
          botao({ rotulo: "Configurar saída deste computador", variante: "primario", aoClicar: () => { indicarParNoLayout(d.peer_id); ir("layout"); } }),
        ),
        h(
          "div",
          { class: "linha" },
          botao({ rotulo: "Ir para Dispositivos", variante: "primario", icone: "dispositivos", aoClicar: () => ir("dispositivos") }),
          botao({
            rotulo: "Parear outro",
            icone: "mais",
            aoClicar: () => {
              novaTentativa();
              pintar();
            },
          }),
        ),
      ];
    }
    return [
      h("div", { class: "aviso aviso--erro" }, h("p", null, fraseDoResultado(resultado))),
      h(
        "div",
        { class: "linha" },
        botao({
          rotulo: "Recomeçar com novo código", dataset: { focusKey: "pareamento-recomecar" },
          variante: "primario",
          icone: "atualizar",
          aoClicar: recomecar,
        }),
        botaoCorrigirEndereco(),
      ),
    ];
  }

  function rotuloGrants({ clipboard_in, clipboard_out, input_in }) {
    const partes = [clipboard_in && "envia clipboard", clipboard_out && "recebe clipboard", input_in && "controla esta máquina"].filter(Boolean);
    return partes.length > 0 ? partes.join(", ") : "nenhuma";
  }

  function pintar({ foco = guardarFoco(no), avancar = false } = {}) {
    if (saiu || !estado) return;
    const acompanhaTentativa = interacaoAtual(focoTentativa, no);
    pararRelogio();
    const aviso_ = erroGeral && h("div", { class: "aviso aviso--erro" }, h("p", null, erroGeral));
    botaoCabecalho.hidden = estado.step === "idle" || !!resultado;
    if (resultado) {
      corpo.classList.add("colunas--unica");
      trocar(corpo, h("div", { class: "pilha" }, aviso_, cartao({ filhos: painelResultado() })));
    } else if (expirou) {
      corpo.classList.add("colunas--unica");
      trocar(corpo, h("div", { class: "pilha" }, aviso_, cartao({
        titulo: "Tempo de pareamento esgotado",
        filhos: [
          h("p", null, "Recomece para criar outra tentativa e outro código. O endereço digitado será preservado; as permissões anteriores serão descartadas."),
          h("div", { class: "linha" },
            botao({ rotulo: "Recomeçar com novo código", variante: "primario", dataset: { focusKey: "pareamento-recomecar" }, aoClicar: recomecar }),
            botaoCorrigirEndereco(),
          ),
        ],
      })));
    } else {
      const passos = { listening: cartaoEscutando, connecting: cartaoConectando, code: cartaoCodigo, permissions: cartaoConfirmado, waiting_peer: cartaoAguardandoPar };
      const permissoesVisiveis = ["permissions", "waiting_peer"].includes(estado.step);
      corpo.classList.toggle("colunas--unica", !permissoesVisiveis);
      trocar(corpo, h("div", { class: "pilha" }, aviso_, abas(), (passos[estado.step] ?? cartaoOcioso)()), permissoesVisiveis && painelPermissoes());
    }
    for (const titulo of corpo.querySelectorAll("h2")) titulo.tabIndex = -1;
    const primeiro = () => erroEndereco || (modo === "conectar" && estado.step === "idle" && !resultado) ? [...corpo.querySelectorAll("[data-focus-key]")].find((e) => e.dataset.focusKey === "pareamento-endereco") : corpo.querySelector("h2, input:not(:disabled), button:not(:disabled)");
    if (avancar && interacaoAtual(foco, no)) {
      primeiro()?.focus({ preventScroll: true });
      focoTentativa = guardarFoco(no);
    } else if (restaurarFoco(foco, no, { fallback: primeiro }) && acompanhaTentativa) {
      // A própria repintura da espera não encerra a posse da ação sobre o foco.
      focoTentativa = guardarFoco(no);
    }
  }

  function receberResultado(data) {
    if (!estado) return;
    if (data.attempt_id && data.attempt_id !== estado.attempt_id) return;
    if (corrigindoEndereco) { resultadoDuranteCorrecao = data; return; }
    geracaoCarga++;
    const foco = focoTentativa;
    resultado = data;
    estado = { ...estado, step: "idle", attempt_id: null, code: null, peer: null };
    if (data.outcome === "paired") anuncios.status("Pareamento concluído.");
    else anuncios.erro(fraseDoResultado(data));
    pintar({ foco, avancar: interacaoAtual(foco, no) });
  }

  const cancelarEventos = [
    assinar("pair_code_ready", (data) => {
      if (!estado || corrigindoEndereco) return;
      if (data.attempt_id !== estado.attempt_id || expirou) return;
      geracaoCarga++;
      const foco = focoTentativa;
      estado = { ...estado, step: "code", attempt_id: data.attempt_id, code: data.code, peer: data.peer };
      resultado = null;
      anuncios.status(`Código de confirmação: ${String(data.code).split("").join(" ")}. Confira na outra máquina.`);
      pintar({ foco, avancar: interacaoAtual(foco, no) });
    }),
    assinar("pair_result", receberResultado),
    assinar("resync", () => carregar()),
  ];

  carregar();

  return {
    no,
    desmontar: () => {
      saiu = true;
      geracaoCarga++;
      pararRelogio();
      // Depois de "Concluir" (waiting_peer) a decisão já é sua e fica com a outra máquina; antes disso, sair cancela.
      if (["listening", "connecting", "code", "permissions"].includes(estado?.step)) abandonar(estado.attempt_id);
      for (const fn of cancelarEventos) fn();
    },
  };
}
