// Tela 1 · Onboarding em 3 passos na mesma janela: esta máquina (nome e impressão digital),
// o que sincronizar (dois módulos) e teste com conteúdo sintético. O perfil é criado ao sair do passo 2.

import { criarAnunciador, guardarFoco, interacaoAtual, restaurarFoco, ocupar } from "../acessibilidade.js";
import { criarEdicaoModulos, resumoEfeitos } from "./preferencias.js";
import { api } from "../api.js";
import { aviso, cabecalho, estadoErro } from "../areas.js";
import { botao, campo, esqueleto, interruptor } from "../componentes.js";
import { h, trocar } from "../dom.js";
import { atualizarApp, obterApp, pausaObservada } from "../estado.js";
import { INDISPONIVEL, mensagemDe, seloTeste } from "../formatar.js";
import { icone } from "../icones.js";

const PASSOS = [
  { n: 1, titulo: "Esta máquina", descricao: "Como os outros dispositivos vão ver esta máquina." },
  { n: 2, titulo: "O que sincronizar", descricao: "Ligue só o que quiser. Dá para mudar depois." },
  {
    n: 3,
    titulo: "Teste",
    descricao:
      "Teste local opcional. Confere os módulos ligados e pode mover o cursor nesta máquina. Não comprova a conexão nem o controle de outro computador.",
  },
];

const PORTA_PADRAO = 47800;

export function salvarEscolhaModulos({ perfilExiste, edicao, nome, modulos }) {
  if (perfilExiste) return edicao.aplicar();
  return api.onboardingCreate({ name: nome, modules: { ...modulos }, port: PORTA_PADRAO, recreate: false });
}

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
        h("span", { class: "onboarding__numero", "aria-hidden": "true" }, p.n < atual ? icone("verificar") : String(p.n)),
        p.titulo,
        p.n < atual && h("span", { class: "so-leitor" }, ", concluído"),
      ),
    ),
  );
}

export async function carregarInicio(app = obterApp()) {
  if (app?.initialized) return {
    default_name: app.device?.name ?? "Esta máquina",
    fingerprint_short: app.device?.short ?? "Não confirmada",
    input_supported: app.input_supported,
    already_initialized: true,
    defaults: { ...app.modules },
  };
  dados.inicio ??= await api.onboardingStart();
  return dados.inicio;
}

export default async function montar({ rota, ir }) {
  const passo = PASSOS.find((p) => p.n === rota.passo) ?? PASSOS[0];
  const app = obterApp();
  let saiu = false;
  const anuncios = criarAnunciador();
  function mostrarErro(destino, erro) {
    const texto = `${mensagemDe(erro)} ${resumoEfeitos(erro.details)}`.trim();
    trocar(destino, h("div", { class: "aviso aviso--erro" }, h("p", null, texto)));
    anuncios.erro(texto);
  }

  // Sem nome escolhido não há passo 2; sem perfil não há passo 3.
  if (passo.n === 2 && !app?.initialized && dados.nome === null) {
    ir("onboarding/1");
    return { no: h("div") };
  }
  if (passo.n === 3 && !app?.initialized) {
    ir(dados.nome === null ? "onboarding/1" : "onboarding/2");
    return { no: h("div") };
  }

  // Um cartão central: marca e passos no topo, conteúdo no meio e, no rodapé, Voltar à esquerda e a ação principal à direita.
  const corpo = h("div", { class: "pilha" });
  const rodape = h("div", { class: "onboarding__rodape" });
  const no = h(
    "section",
    { class: "cartao onboarding__cartao" },
    h("div", { class: "onboarding__topo" }, h("p", { class: "onboarding__marca" }, "ClipSync"), indicador(passo.n)),
    cabecalho({ titulo: passo.titulo, descricao: passo.descricao }),
    anuncios.no,
    corpo,
    rodape,
  );

  const pintores = { 1: passo1, 2: passo2, 3: passo3 };
  await pintores[passo.n]();
  return { no, desmontar: () => { saiu = true; } };

  async function passo1() {
    trocar(corpo, esqueleto({ linhas: 3 }));
    let inicio;
    try {
      inicio = await carregarInicio();
    } catch (erro) {
      trocar(corpo, estadoErro(erro, passo1));
      return;
    }
    dados.nome = app?.initialized ? app.device?.name ?? inicio.default_name : dados.nome ?? inicio.default_name;
    dados.modulos = app?.initialized ? { ...app.modules } : dados.modulos ?? { ...inicio.defaults };

    let nome = dados.nome;
    const areaCampo = h("div");
    const pintarCampo = (erro = null, focar = true) => {
      const c = campo({
        rotulo: "Nome desta máquina",
        valor: nome,
        erro,
        ajuda: "Pré-preenchido com o nome do sistema. Só para você reconhecer a máquina: a identidade é a chave abaixo.",
        maxlength: "32",
        dataset: { focusKey: "onboarding-nome" },
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
      if (erro && focar) c.controle.focus();
    };
    if (app?.initialized) trocar(areaCampo, h("p", { class: "campo__rotulo" }, "Nome desta máquina"), h("p", null, dados.nome), h("p", { class: "nota" }, "O nome e a identidade existentes são preservados nesta revisão."));
    else pintarCampo();

    const continuar = botao({
      rotulo: app?.initialized ? "Revisar módulos" : "Continuar",
      dataset: { focusKey: "onboarding-continuar" },
      variante: "primario",
      icone: "avancar",
      aoClicar: async () => {
        if (app?.initialized) { ir("onboarding/2"); return; }
        const foco = guardarFoco(no);
        try {
          await api.onboardingCheckName({ name: nome.trim() });
        } catch (erro) {
          if (saiu) return;
          pintarCampo(mensagemDe(erro), interacaoAtual(foco, no));
          anuncios.erro(mensagemDe(erro));
          return;
        }
        if (saiu) return;
        dados.nome = nome.trim();
        ir("onboarding/2");
      },
    });

    trocar(corpo, 
      app?.initialized &&
        aviso(
          "aviso",
          h("p", null, "Esta máquina já tem um perfil. Revise os módulos sem alterar a identidade, os pares ou as permissões."),
          botao({ rotulo: "Ir para Dispositivos", variante: "discreto", aoClicar: () => ir("dispositivos") }),
        ),
      areaCampo,
      h(
        "div",
        { class: "pilha onboarding__impressao-bloco" },
        h("p", { class: "campo__rotulo" }, "Impressão digital"),
        h("p", { class: "mono onboarding__impressao" }, inicio.fingerprint_short),
        h("p", { class: "nota" }, "A impressão digital identifica a chave desta máquina. Ela aparece na outra máquina ao parear, para você conferir."),
      ),
    );
    trocar(rodape, continuar);
  }

  async function passo2(erroInicial = null) {
    const corrente = obterApp();
    const perfilExiste = corrente?.initialized === true;
    const inicio = dados.inicio ?? { input_supported: corrente?.input_supported ?? false, defaults: { input: true, clipboard: false } };
    if (perfilExiste) dados.modulos = { ...corrente.modules };
    else dados.modulos ??= { ...inicio.defaults };
    const modulos = dados.modulos;
    const inputSuportado = (corrente?.input_supported ?? inicio.input_supported) === true;
    if (!inputSuportado && !perfilExiste) modulos.input = false;
    const edicao = perfilExiste ? criarEdicaoModulos({ modules: corrente.modules, aplicar: api.modulesSet, reconsultar: atualizarApp }) : null;
    const controles = [];
    let ocupado = false;
    const avisoNenhum = h("div");
    const erroCriar = h("div");
    const voltar = botao({ rotulo: "Voltar", icone: "voltar", dataset: { focusKey: "onboarding-voltar" }, aoClicar: () => { if (!ocupado) ir("onboarding/1"); } });
    const criar = botao({
      rotulo: perfilExiste ? "Aplicar módulos e continuar" : "Criar perfil e continuar",
      variante: "primario", icone: "avancar", dataset: { focusKey: "onboarding-aplicar" },
      aoClicar: async () => {
        const foco = guardarFoco(no);
        trocar(erroCriar); anuncios.limpar();
        const liberacoes = [voltar, ...controles.map(([controle]) => controle)].map(ocupar).filter(Boolean);
        ocupado = true; atualizar();
        try {
          const resultado = await salvarEscolhaModulos({ perfilExiste, edicao, nome: dados.nome, modulos });
          dados.resultados = null;
          // A reconsulta não retoma o agente. Retomar permanece uma ação explícita.
          await atualizarApp();
          if (saiu) return;
          dados.modulos = { ...obterApp().modules };
          if (resultado?.changed && resultado.effects) anuncios.status(resumoEfeitos(resultado));
          ir("onboarding/3");
        } catch (erro) {
          if (saiu) return;
          if (edicao?.confirmado) {
            // Reconstrói os controles com o valor reconsultado, inclusive sua escolha confirmada interna.
            await passo2(erro);
          } else mostrarErro(erroCriar, erro);
        } finally {
          for (const liberar of liberacoes) liberar();
          ocupado = false; atualizar();
          if (!saiu) restaurarFoco(foco, no);
        }
      },
    });
    const atualizar = () => {
      const nenhum = !modulos.input && !modulos.clipboard;
      criar.disabled = nenhum || (edicao && !edicao.confirmado);
      for (const [controle, chave] of controles) controle.disabled = (chave === "input" && !inputSuportado);
      trocar(avisoNenhum, h("p", { class: "nota" },
        edicao && !edicao.confirmado ? "O estado não foi confirmado. Volte e revise os módulos novamente antes de aplicar."
          : nenhum ? "Ligue pelo menos um módulo para continuar."
          : perfilExiste ? "Aplique para salvar o rascunho. Se o agente reiniciar, a malha ficará pausada. Use Retomar tudo depois, quando quiser."
          : "Ligue só o que quiser. Você pode revisar os módulos depois em Preferências."));
    };

    const cartaoModulo = ({ rotulo, descricao, permissao, chave, desabilitado = false, motivo }) => {
      const no = h(
        "section",
        { class: "cartao onboarding__modulo" },
        interruptor({
          rotulo,
          chaveFoco: `onboarding-modulo-${chave}`,
          descricao: motivo ?? descricao,
          marcado: modulos[chave],
          desabilitado,
          aoMudar: (valor) => {
            if (ocupado) throw { code: "busy", message: "Aguarde a aplicação dos módulos." };
            modulos[chave] = valor;
            edicao?.editar(chave, valor);
            no.classList.toggle("onboarding__modulo--ligado", valor);
            atualizar();
          },
        }),
        !motivo && h("p", { class: "nota" }, permissao),
      );
      controles.push([no.querySelector("input"), chave]);
      no.classList.toggle("onboarding__modulo--ligado", modulos[chave] === true);
      return no;
    };

    trocar(corpo, 
      h(
        "div",
        { class: "onboarding__modulos" },
        cartaoModulo({
          chave: "input",
          rotulo: MODULOS.input,
          descricao: "Leva o cursor e o teclado para outra máquina pelas bordas que você ligar em Layout de telas. Ligado por padrão.",
          permissao: corrente?.platform === "windows"
            ? "Usa a captura de teclado e mouse do Windows quando houver ligação de saída local. Pausar remove a captura."
            : "Na primeira captura, o Linux pode pedir permissão pelo portal InputCapture. A decisão é sua no diálogo do sistema.",
          desabilitado: !inputSuportado,
          motivo: inputSuportado ? null : INDISPONIVEL,
        }),
        cartaoModulo({
          chave: "clipboard",
          rotulo: MODULOS.clipboard,
          descricao: "Envia o texto que você copiar aos pares autorizados e recebe deles. Só texto nesta versão. Desligado por padrão.",
          permissao: corrente?.platform === "windows"
            ? "Usa a área de transferência da sessão do Windows."
            : "Usa a área de transferência da sessão gráfica Linux. O adaptador disponível aparece na tela Clipboard.",
        }),
      ),
      avisoNenhum,
      erroCriar,
    );
    trocar(rodape, voltar, criar);
    atualizar();
    if (erroInicial) mostrarErro(erroCriar, erroInicial);
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
              h("div", { class: "onboarding__resultado-texto" }, h("strong", null, MODULOS[r.module] ?? r.module), r.detail && h("p", { class: "nota" }, r.detail)),
              seloTeste(r.status),
            ),
          ),
        ),
      );
    };
    pintarResultados();

    const erroTeste = h("div");
    trocar(corpo, 
      pausaObservada(obterApp()) === true && h("div", { class: "aviso aviso--aviso" }, h("p", null, "A malha está pausada. Ao concluir, use Retomar tudo quando quiser. O teste é opcional.")),
      dados.modulos?.clipboard !== false &&
        aviso("aviso", h("p", { id: "aviso-teste-clipboard" }, "O teste escreve um texto de teste na área de transferência e devolve o texto anterior. Conteúdo que não é texto (imagem, arquivo) não volta.")),
      h(
        "div",
        { class: "linha" },
        botao({
          rotulo: "Rodar teste com conteúdo sintético", dataset: { focusKey: "onboarding-testar" },
          variante: "primario",
          "aria-describedby": dados.modulos?.clipboard !== false ? "aviso-teste-clipboard" : null,
          icone: "verificar",
          aoClicar: async () => {
            trocar(erroTeste);
            try {
              dados.resultados = (await api.onboardingSelfTest()).results;
            } catch (erro) {
              if (saiu) return;
              mostrarErro(erroTeste, erro);
              return;
            }
            if (!saiu) { pintarResultados(); anuncios.status("Teste local concluído. Confira o resultado de cada módulo."); }
          },
        }),
      ),
      erroTeste,
      areaResultados,
    );
    trocar(
      rodape,
      botao({ rotulo: "Voltar", icone: "voltar", aoClicar: () => ir("onboarding/2") }),
      botao({
        rotulo: "Concluir sem testar", dataset: { focusKey: "onboarding-concluir" },
        variante: "primario",
        icone: "avancar",
        aoClicar: async () => {
          try {
            await atualizarApp();
            if (!saiu) ir("dispositivos");
          } catch (erro) { if (!saiu) mostrarErro(erroTeste, erro); }
        },
      }),
    );
  }
}
