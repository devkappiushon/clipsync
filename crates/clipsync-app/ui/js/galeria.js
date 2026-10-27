// Catálogo de componentes: ui/componentes.html. Serve para conferir cada estado nos dois temas.

import { h } from "./dom.js";
import { botao, campo, cartao, confirmar, esqueleto, interruptor, segmentado, selo, tabela, tecla } from "./componentes.js";
import { aplicarTema, lerTema } from "./tema.js";

const espera = (ms) => new Promise((resolver) => setTimeout(resolver, ms));

const dispositivos = [
  { id: "aa31-537a", nome: "pc-pessoal", estado: ["ok", "conectado"], endereco: "192.168.115.30:47800", atividade: "há 2 min" },
  { id: "1bc6-7a55", nome: "pc-de-casa", estado: ["aviso", "offline"], endereco: "192.168.115.41:47800", atividade: "ontem, 22:14" },
  { id: "817a-ab9c", nome: "notebook-antigo", estado: ["erro", "bloqueado: chave mudou"], endereco: "192.168.115.55:47800", atividade: "12 out" },
];

function secao(titulo, ...filhos) {
  return cartao({ titulo, filhos: [h("div", { class: "pilha" }, filhos)] });
}

function montar() {
  let selecionada = dispositivos[0].id;
  const areaTabela = h("div");
  const pintarTabela = () =>
    areaTabela.replaceChildren(
      tabela({
        legenda: "Dispositivos pareados",
        colunas: [
          { chave: "nome", rotulo: "Nome", render: (l) => h("div", null, h("strong", null, l.nome), h("div", { class: "mono muted" }, l.id)) },
          { chave: "estado", rotulo: "Estado", render: (l) => selo(...l.estado) },
          { chave: "endereco", rotulo: "Endereço", tipo: "mono" },
          { chave: "atividade", rotulo: "Última atividade", tipo: "num" },
        ],
        linhas: dispositivos,
        selecionada,
        aoSelecionar: (linha) => {
          selecionada = linha.id;
          pintarTabela();
          areaTabela.querySelector("[aria-selected=true]").focus();
        },
      }),
    );
  pintarTabela();

  const resultadoDialogo = h("p", { class: "muted", role: "status" }, "Nenhum diálogo aberto.");

  const pagina = h(
    "div",
    { class: "pagina" },
    h(
      "header",
      { class: "cabecalho" },
      h("div", { class: "cabecalho__texto" }, h("h1", null, "Componentes"), h("p", null, "Todos os estados, nos dois temas. Troque o tema aqui.")),
      segmentado({
        rotulo: "Tema",
        opcoes: [
          { valor: "sistema", rotulo: "Sistema" },
          { valor: "claro", rotulo: "Claro" },
          { valor: "escuro", rotulo: "Escuro" },
        ],
        valor: lerTema(),
        aoMudar: aplicarTema,
      }),
    ),

    secao(
      "Botões",
      h(
        "div",
        { class: "linha" },
        botao({ rotulo: "Parear novo dispositivo", variante: "primario", icone: "mais" }),
        botao({ rotulo: "Pausar este par" }),
        botao({ rotulo: "Revogar", variante: "perigo" }),
        botao({ rotulo: "Copiar", variante: "discreto", icone: "copiar" }),
        botao({ rotulo: "Pausar tudo", icone: "pausa", atalho: "Win+Alt+P" }),
      ),
      h(
        "div",
        { class: "linha" },
        botao({ rotulo: "Desabilitado", variante: "primario", desabilitado: true }),
        botao({ rotulo: "Desabilitado", desabilitado: true }),
        botao({ rotulo: "Desabilitado", variante: "perigo", desabilitado: true }),
        botao({ rotulo: "Salvar ligação (2 s)", variante: "primario", aoClicar: () => espera(2000) }),
        botao({ rotulo: "Rodar teste (2 s)", aoClicar: () => espera(2000) }),
      ),
    ),

    secao(
      "Selos de estado",
      h(
        "div",
        { class: "linha" },
        selo("ok", "conectado"),
        selo("aviso", "offline"),
        selo("erro", "bloqueado: chave mudou"),
        selo("neutro", "não testado"),
        selo("acento", "experimental"),
      ),
      h("div", { class: "linha" }, "Atalho de pausa:", tecla("Win+Alt+P"), "troca de tela:", tecla("Win+Alt+2")),
    ),

    secao(
      "Interruptores",
      interruptor({ rotulo: "Teclado e mouse", descricao: "Move o cursor para outra máquina pelas bordas que você configurar.", marcado: true }),
      interruptor({ rotulo: "Área de transferência", descricao: "Envia o texto que você copiar aos pares autorizados e recebe deles.", aoMudar: () => espera(1200) }),
      interruptor({ rotulo: "Modo privado", descricao: "Desabilitado.", desabilitado: true }),
    ),

    secao(
      "Campos e seletor segmentado",
      campo({ rotulo: "Nome desta máquina", valor: "notebook-a", ajuda: "O nome é só para você reconhecer a máquina." }),
      campo({ rotulo: "Endereço e porta", tipo: "mono", valor: "192.168.115.30:4790", erro: "Use o formato endereço:porta, com a porta entre 1 e 65535." }),
      campo({ rotulo: "Idioma", tipo: "select", valor: "pt-BR", opcoes: [{ valor: "pt-BR", rotulo: "português (Brasil)" }, { valor: "en", rotulo: "English" }] }),
      segmentado({
        rotulo: "Modo do clipboard",
        opcoes: [
          { valor: "automatico", rotulo: "Automático" },
          { valor: "manual", rotulo: "Manual" },
          { valor: "confirmacao", rotulo: "Com confirmação" },
        ],
        valor: "automatico",
      }),
    ),

    cartao({ titulo: "Tabela (linha selecionável por clique, Enter e Espaço)", plano: true, filhos: [areaTabela] }),

    secao(
      "Diálogos",
      h(
        "div",
        { class: "linha" },
        botao({
          rotulo: "Confirmação simples",
          aoClicar: async () => {
            const ok = await confirmar({ titulo: "Pausar este par?", mensagem: "pc-pessoal deixa de enviar e receber até você retomar.", rotuloConfirmar: "Pausar", aoConfirmar: () => espera(1000) });
            resultadoDialogo.textContent = ok ? "Confirmado." : "Cancelado.";
          },
        }),
        botao({
          rotulo: "Revogar com nome digitado",
          variante: "perigo",
          aoClicar: async () => {
            const ok = await confirmar({
              titulo: "Revogar pc-pessoal?",
              mensagem: "A chave deste dispositivo deixa de valer. Para voltar, será preciso parear de novo.",
              rotuloConfirmar: "Revogar",
              perigo: true,
              digitar: "pc-pessoal",
              aoConfirmar: () => espera(1000),
            });
            resultadoDialogo.textContent = ok ? "Revogado." : "Cancelado.";
          },
        }),
      ),
      resultadoDialogo,
    ),

    secao("Esqueleto de carregamento", esqueleto({ linhas: 3, largura: "70%" })),
  );

  document.getElementById("conteudo").replaceChildren(pagina);
}

montar();
