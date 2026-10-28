// Tela 1 · Onboarding em 3 passos na mesma janela: esta máquina (nome e impressão digital),
// o que sincronizar (dois módulos) e teste com conteúdo sintético. O perfil é criado ao sair do passo 2.

import { api } from "../api.js";
import { aviso, cabecalho, estadoErro } from "../areas.js";
import { botao, campo, cartao, confirmar, esqueleto, interruptor } from "../componentes.js";
import { h, trocar } from "../dom.js";
import { atualizarApp, obterApp } from "../estado.js";
import { INDISPONIVEL, mensagemDe, seloTeste } from "../formatar.js";

const PASSOS = [
  { n: 1, titulo: "Esta máquina", descricao: "Como os outros dispositivos vão ver esta máquina." },
  { n: 2, titulo: "O que sincronizar", descricao: "Ligue só o que quiser. Dá para mudar depois." },
  { n: 3, titulo: "Teste", descricao: "Confira os módulos antes de parear a primeira máquina." },
];

const PORTA_PADRAO = 47800;

// Estado compartilhado entre os passos enquanto a janela está aberta.
const dados = { inicio: null, nome: null, modulos: null, resultados: null };

const MODULOS = { clipboard: "Área de transferência", input: "Teclado e mouse" };

function indicador(atual) {
  return h(
    "ol",
    { class: "onboarding__passos", "aria-label": "Passos da configuração" },
    PASSOS.map((p) =>
      h(
        "li",
        { "aria-current": p.n === atual ? "step" : null, class: p.n < atual ? "onboarding__passo--feito" : "" },
        h("span", { class: "onboarding__numero", "aria-hidden": "true" }, String(p.n)),
        p.titulo,
      ),
    ),
  );
}

async function carregarInicio() {
  dados.inicio ??= await api.onboardingStart();
  return dados.inicio;
}

export default async function montar({ rota, ir }) {
  const passo = PASSOS.find((p) => p.n === rota.passo) ?? PASSOS[0];
  const app = obterApp();

  // Sem nome escolhido não há passo 2; sem perfil não há passo 3.
  if (passo.n === 2 && dados.nome === null) {
    ir("onboarding/1");
    return { no: h("div") };
  }
  if (passo.n === 3 && !app?.initialized) {
    ir(dados.nome === null ? "onboarding/1" : "onboarding/2");
    return { no: h("div") };
  }

  const corpo = h("div", { class: "pilha" });
  const no = h(
    "div",
    { class: "onboarding" },
    h("p", { class: "onboarding__marca" }, "Primeiro uso"),
    indicador(passo.n),
    cabecalho({ titulo: passo.titulo, descricao: passo.descricao }),
    corpo,
  );

  const pintores = { 1: passo1, 2: passo2, 3: passo3 };
  await pintores[passo.n]();
  return { no };

  async function passo1() {
    trocar(corpo, esqueleto({ linhas: 3 }));
    let inicio;
    try {
      inicio = await carregarInicio();
    } catch (erro) {
      trocar(corpo, estadoErro(erro, passo1));
      return;
    }
    dados.nome ??= inicio.default_name;
    dados.modulos ??= { ...inicio.defaults };

    let nome = dados.nome;
    const areaCampo = h("div");
    const pintarCampo = (erro = null) => {
      const c = campo({
        rotulo: "Nome desta máquina",
        valor: nome,
        erro,
        ajuda: "Pré-preenchido com o nome do sistema. Só para você reconhecer a máquina: a identidade é a chave abaixo.",
        maxlength: "32",
        aoMudar: (valor) => {
          nome = valor;
        },
      });
      c.controle.addEventListener("keydown", (evento) => {
        if (evento.key === "Enter") {
          evento.preventDefault();
          continuar.click();
        }
      });
      trocar(areaCampo, c);
      if (erro) c.controle.focus();
    };
    pintarCampo();

    const continuar = botao({
      rotulo: "Continuar",
      variante: "primario",
      icone: "avancar",
      aoClicar: async () => {
        try {
          await api.onboardingCheckName({ name: nome.trim() });
        } catch (erro) {
          pintarCampo(mensagemDe(erro));
          return;
        }
        dados.nome = nome.trim();
        ir("onboarding/2");
      },
    });

    trocar(corpo, 
      inicio.already_initialized &&
        aviso(
          "aviso",
          h("p", null, "Esta máquina já tem um perfil. Continuar refaz a identidade, e os pares terão de parear de novo."),
          botao({ rotulo: "Ir para Dispositivos", variante: "discreto", aoClicar: () => ir("dispositivos") }),
        ),
      cartao({
        filhos: [
          areaCampo,
          h(
            "dl",
            { class: "kv" },
            h("dt", null, "Impressão digital"),
            h("dd", null, h("span", { class: "mono onboarding__impressao" }, inicio.fingerprint_short)),
          ),
          h("p", { class: "nota" }, "A impressão digital identifica a chave desta máquina. Ela aparece na outra máquina ao parear, para você conferir."),
        ],
      }),
      h("div", { class: "linha" }, continuar),
    );
  }

  async function passo2() {
    const inicio = dados.inicio ?? { input_supported: app?.input_supported ?? true, defaults: { input: true, clipboard: false } };
    dados.modulos ??= { ...inicio.defaults };
    const modulos = dados.modulos;
    const inputSuportado = inicio.input_supported === true;
    if (!inputSuportado) modulos.input = false;

    const avisoVazio = h("div");
    const erroCriar = h("div");
    const criar = botao({
      rotulo: "Criar perfil e continuar",
      variante: "primario",
      icone: "avancar",
      aoClicar: async () => {
        trocar(erroCriar);
        const pedido = { name: dados.nome, modules: { ...modulos }, port: PORTA_PADRAO, recreate: false };
        try {
          try {
            await api.onboardingCreate(pedido);
          } catch (erro) {
            if (erro?.code !== "needs_confirmation") throw erro;
            const ok = await confirmar({
              titulo: "Refazer o perfil desta máquina?",
              mensagem: `${mensagemDe(erro)} Os dispositivos pareados hoje deixam de confiar nesta máquina.`,
              rotuloConfirmar: "Refazer perfil",
              perigo: true,
              aoConfirmar: () => api.onboardingCreate({ ...pedido, recreate: true }),
            });
            if (!ok) return;
          }
        } catch (erro) {
          trocar(erroCriar, aviso("erro", h("p", null, mensagemDe(erro))));
          return;
        }
        dados.resultados = null;
        await atualizarApp();
        ir("onboarding/3");
      },
    });

    const atualizar = () => {
      const nenhum = !modulos.input && !modulos.clipboard;
      criar.disabled = nenhum;
      trocar(avisoVazio, nenhum ? aviso("aviso", h("p", null, "Com os dois desligados, esta máquina não envia nem recebe nada. Ligue pelo menos um.")) : "");
    };

    const cartaoModulo = ({ rotulo, descricao, permissao, chave, desabilitado = false, motivo }) =>
      cartao({
        filhos: [
          interruptor({
            rotulo,
            descricao: motivo ?? descricao,
            marcado: modulos[chave],
            desabilitado,
            aoMudar: (valor) => {
              modulos[chave] = valor;
              atualizar();
            },
          }),
          !motivo && h("p", { class: "nota" }, permissao),
        ],
      });

    trocar(corpo, 
      h(
        "div",
        { class: "onboarding__modulos" },
        cartaoModulo({
          chave: "input",
          rotulo: MODULOS.input,
          descricao: "Leva o cursor e o teclado para outra máquina pelas bordas que você ligar em Layout de telas. Ligado por padrão.",
          permissao: "Pede ao sistema a permissão de captura de entrada (portal InputCapture) na primeira vez.",
          desabilitado: !inputSuportado,
          motivo: inputSuportado ? null : INDISPONIVEL,
        }),
        cartaoModulo({
          chave: "clipboard",
          rotulo: MODULOS.clipboard,
          descricao: "Envia o texto que você copiar aos pares autorizados e recebe deles. Só texto nesta versão. Desligado por padrão.",
          permissao: "Não pede permissão extra: usa o clipboard da sessão gráfica, por X11 ou pelo portal.",
        }),
      ),
      avisoVazio,
      erroCriar,
      h("div", { class: "linha" }, botao({ rotulo: "Voltar", icone: "voltar", aoClicar: () => ir("onboarding/1") }), criar),
    );
    atualizar();
  }

  async function passo3() {
    const areaResultados = h("div", { class: "pilha" });
    const pintarResultados = () => {
      if (!dados.resultados) {
        trocar(areaResultados);
        return;
      }
      trocar(areaResultados, 
        h(
          "ul",
          { class: "onboarding__resultados", "aria-label": "Resultado por módulo" },
          dados.resultados.map((r) =>
            h(
              "li",
              null,
              h("div", { class: "onboarding__resultado-topo" }, h("strong", null, MODULOS[r.module] ?? r.module), seloTeste(r.status)),
              r.detail && h("p", { class: "muted" }, r.detail),
            ),
          ),
        ),
      );
    };
    pintarResultados();

    const erroTeste = h("div");
    trocar(corpo, 
      cartao({
        filhos: [
          h("p", null, "O teste confere cada módulo ligado e, com teclado e mouse ligados, move o cursor em loopback nesta máquina."),
          dados.modulos?.clipboard !== false &&
            aviso("aviso", h("p", { id: "aviso-teste-clipboard" }, "O teste escreve um texto de teste na área de transferência e devolve o texto anterior. Conteúdo que não é texto (imagem, arquivo) não volta.")),
          h(
            "div",
            { class: "linha" },
            botao({
              rotulo: "Rodar teste com conteúdo sintético",
              variante: "primario",
              "aria-describedby": dados.modulos?.clipboard !== false ? "aviso-teste-clipboard" : null,
              icone: "verificar",
              aoClicar: async () => {
                trocar(erroTeste);
                try {
                  dados.resultados = (await api.onboardingSelfTest()).results;
                } catch (erro) {
                  trocar(erroTeste, aviso("erro", h("p", null, mensagemDe(erro))));
                  return;
                }
                pintarResultados();
              },
            }),
          ),
          erroTeste,
          areaResultados,
        ],
      }),
      h(
        "div",
        { class: "linha" },
        botao({ rotulo: "Voltar", icone: "voltar", aoClicar: () => ir("onboarding/2") }),
        botao({
          rotulo: "Concluir",
          variante: "primario",
          icone: "avancar",
          aoClicar: async () => {
            await atualizarApp();
            ir("dispositivos");
          },
        }),
      ),
    );
  }
}
