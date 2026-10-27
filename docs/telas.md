# Telas do ClipSync (protótipo Figma e base da UI Tauri)

Desktop 1280×800, temas claro e escuro. Sistema: Inter no corpo, Geist nos títulos, Geist Mono em códigos, ids e valores de tabela (`tabular-nums`). Escala de 8 pt. Um raio de borda (8 px) e uma elevação. Paleta: fundo neutro, texto alto contraste, um acento azul-escuro para ação primária, verde e vermelho só para estado, sempre com ícone ou texto junto. Sem roxo, sem emoji como ícone, sem travessão em texto de interface, sem kicker decorativo. Foco visível em tudo. Textos em português, preparados para tradução.

## Estrutura comum

Janela com barra lateral fixa de 240 px (Dispositivos, Layout de telas, Clipboard, Diagnóstico, Preferências) e um rodapé de estado na lateral: nome desta máquina, estado da malha (3 de 3 conectados), botão de pausa global com atalho visível. Conteúdo à direita com título, descrição de uma linha e ação primária no topo.

## 1. Onboarding (3 passos, mesma janela)

1. **Esta máquina:** campo de nome (pré-preenchido com o hostname, editável), explicação de que o nome não é identidade, impressão digital curta em mono.
2. **O que sincronizar:** dois cartões com interruptor. "Teclado e mouse" (ligado por padrão) e "Área de transferência" (desligado por padrão). Cada cartão explica o alcance em duas linhas e as permissões de sistema que vai pedir. Aviso quando os dois estão desligados.
3. **Teste:** botão "Rodar teste com conteúdo sintético", com aviso de que o teste pode alterar o clipboard; resultado por módulo em lista com estado (validado, experimental, indisponível, não testado).

## 2. Dispositivos

Lista em tabela: nome, estado (conectado, pausado, offline, bloqueado por chave diferente), últimos endereços em mono, permissões por módulo em duas colunas (envia, recebe; controla, é controlado), última atividade. Linha selecionada abre painel lateral com ações: pausar par, editar permissões (interruptores por direção e por módulo), revogar (confirmação com nome digitado), reparear. Botão primário: "Parear novo dispositivo". Estado vazio: instrução de três linhas e o mesmo botão.

## 3. Pareamento

Dois modos em abas: "Aceitar" (mostra endereço e porta em mono, tempo restante de 2 minutos, botão cancelar) e "Conectar" (campo endereço:porta). Depois da conexão: código numérico de 6 dígitos em Geist Mono, grande, com a frase "Confirme que a outra máquina mostra o mesmo código", dois botões: "São iguais" e "São diferentes, cancelar". Em seguida, permissões iniciais por módulo com padrão conservador (clipboard recebe e envia; controle desligado) e botão "Concluir".

## 4. Layout de telas

Canvas em escala com os monitores reais de cada máquina, agrupados por dispositivo, com nome e resolução em mono. Arrastar um dispositivo reposiciona o grupo. Clicar numa borda de monitor seleciona um segmento (alças para encurtar) e um painel pede o destino: dispositivo e borda de monitor de destino. Ligações aparecem como linhas entre as bordas. Bordas internas entre monitores locais aparecem travadas, com dica. Lista lateral das ligações com atalho de troca e botão remover. Rodapé: "Travar nesta tela" (atalho), "Cantos mortos", "Copiar layout para os pares".

## 5. Clipboard

Topo: estado do módulo (ligado ou desligado, adaptador em uso, classificação da plataforma), seletor de modo (automático, manual, com confirmação), interruptores "Pausar captura", "Modo privado". Caixa de recebidos: lista com origem, horário local, tipo, tamanho, estado (recebido, validado, aplicado, recusado com motivo). Prévia recolhida por padrão com botão "Mostrar". Ações por item: copiar para o clipboard, descartar. Rodapé: política de retenção da caixa (sem histórico persistente neste nível).

## 6. Diagnóstico

Blocos: Sistema (SO, sessão gráfica, protocolos, portais e versões), Adaptadores (clipboard e input, caminho usado, classificação), Rede (interfaces, portas, pares e RTT, p50 e p95 de entrada), Conflitos (Input Leap, Barrier e gerenciadores de clipboard detectados, com orientação), Últimos bloqueios (motivo, par, horário, sem conteúdo). Botão "Exportar pacote de suporte" com prévia do que vai e remoção de identificadores.

## 7. Preferências

Grupos: Atalhos (abrir, pausar tudo, enviar uma vez, travar nesta tela, ir para dispositivo), Inicialização com a sessão (consentimento explícito), Notificações por tipo (item recebido, dispositivo conectado, bloqueio) com padrão "sem conteúdo", Aparência (tema claro, escuro, sistema; reduzir movimento), Idioma. Rodapé: versão, licença, "feito por montesouza.dev".
