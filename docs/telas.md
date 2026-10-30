# Telas do ClipSync (protótipo Figma e base da UI Tauri)

> Especificação da S6. O fluxo atual de configuração da passagem está na §14.3 do contrato e em `crates/clipsync-app/ui/README.md`. Os atalhos desta página são os da especificação; na v0.3.1 valem Win+Alt+1 a 9, L e P, e C e V aparecem indisponíveis.

Desktop 1280×800, temas claro e escuro. Sistema: Inter no corpo, Geist nos títulos, Geist Mono em códigos, ids e valores de tabela (`tabular-nums`). Atalhos sempre com a tecla Windows como modificador, escrita "Win" (no Linux a mesma tecla é a Super; nunca o símbolo de command da Apple): padrão `Win+Alt+<tecla>`. Escala de 8 pt. Um raio de borda (8 px) e uma elevação. Paleta: fundo neutro, texto alto contraste, um acento azul-escuro para ação primária, verde e vermelho só para estado, sempre com ícone ou texto junto. Sem roxo, sem emoji como ícone, sem travessão em texto de interface, sem kicker decorativo. Foco visível em tudo. Textos em português, preparados para tradução. O que não tem backend na versão atual aparece desabilitado com o texto "indisponível nesta versão", nunca escondido (contrato §12.2 regra 6).

## Estrutura comum

Janela com barra lateral fixa de 240 px (Dispositivos, Layout de telas, Clipboard, Diagnóstico, Preferências) e um rodapé de estado na lateral: nome desta máquina, estado da malha (3 de 3 conectados), botão de pausa global com atalho visível. Conteúdo à direita com título, descrição de uma linha e ação primária no topo.

## 1. Onboarding (3 passos, mesma janela)

1. **Esta máquina:** campo de nome (pré-preenchido com o hostname, editável), explicação de que o nome não é identidade, impressão digital curta em mono.
2. **O que sincronizar:** dois cartões com interruptor. "Teclado e mouse" (ligado por padrão) e "Área de transferência" (desligado por padrão). Cada cartão explica o alcance em duas linhas e as permissões de sistema que vai pedir. Aviso quando os dois estão desligados.
3. **Teste:** botão "Rodar teste com conteúdo sintético", com aviso de que o teste pode alterar o clipboard; resultado por módulo em lista com estado (validado, experimental, indisponível, não testado).

## 2. Dispositivos

Lista em tabela: nome, estado (conectado, pausado, offline, bloqueado por chave diferente), últimos endereços em mono, permissões por módulo em duas colunas (envia, recebe; controla, é controlado), última atividade. Linha selecionada abre painel lateral com ações: pausar par, editar permissões (interruptores por direção e por módulo), revogar (confirmação com nome digitado), reparear. Botão primário: "Parear novo dispositivo". Estado vazio: instrução de três linhas e o mesmo botão.

## 3. Pareamento

Dois modos em abas: "Aceitar" (mostra endereço e porta em mono, tempo restante de 2 minutos, botão cancelar) e "Conectar" (campo endereço:porta). Depois da conexão: código numérico de 6 dígitos em Geist Mono, grande, com a frase "Confirme que a outra máquina mostra o mesmo código", dois botões: "São iguais" e "São diferentes, cancelar". Em seguida, permissões iniciais por módulo, todas desligadas por padrão: o usuário liga o que quiser (prompt-mestre §2.1). Ligar o controle pede a confirmação de confiança total. Botão "Concluir".

## 4. Layout de telas

Canvas em escala sobre uma malha de pontos (16 px) com os monitores reais de cada máquina, agrupados por dispositivo, com nome e resolução em mono. Arrastar um dispositivo reposiciona o grupo e ele encaixa na malha (snap), para alinhar bordas entre máquinas sem precisão de pixel; a malha some ao exportar o layout. Clicar numa borda de monitor seleciona um segmento (alças para encurtar) e um painel pede o destino: dispositivo e borda de monitor de destino. Ligações aparecem como linhas entre as bordas. Bordas internas entre monitores locais aparecem travadas, com dica. Lista lateral das ligações com atalho de troca e botão remover. Rodapé: "Travar nesta tela" (atalho), "Cantos mortos", "Copiar layout para os pares".

## 5. Clipboard

Topo: estado do módulo (ligado ou desligado, adaptador em uso, classificação da plataforma), seletor de modo (automático, manual, com confirmação), interruptores "Pausar captura", "Modo privado". Caixa de recebidos: lista com origem, horário local, tipo, tamanho, estado (recebido, validado, aplicado, recusado com motivo). Prévia recolhida por padrão com botão "Mostrar". Ações por item: copiar para o clipboard, descartar. Rodapé: política de retenção da caixa (sem histórico persistente neste nível).

## 6. Diagnóstico

Blocos: Sistema (SO, sessão gráfica, protocolos, portais e versões), Adaptadores (clipboard e input, caminho usado, classificação), Rede (interfaces, portas, pares e RTT, p50 e p95 de entrada), Conflitos (Input Leap, Barrier e gerenciadores de clipboard detectados, com orientação), Últimos bloqueios (motivo, par, horário, sem conteúdo). Botão "Exportar pacote de suporte" com prévia do que vai e remoção de identificadores.

## 7. Preferências

Grupos: Atalhos (abrir Win+Alt+C, pausar tudo Win+Alt+P, enviar uma vez Win+Alt+V, travar nesta tela Win+Alt+L, ir para dispositivo Win+Alt+1 a 3), Inicialização com a sessão (consentimento explícito), Notificações por tipo (item recebido, dispositivo conectado, bloqueio) com padrão "sem conteúdo", Aparência (tema claro, escuro, sistema; reduzir movimento), Idioma. Rodapé: versão, licença, "feito por montesouza.dev".

## Arquivo no Figma

Arquivo: https://www.figma.com/design/vkXY3Q37AaGe1argTA2STv (rascunhos da equipe "A equipe de João pedro").

Criado em 2026-10-27 pelo MCP do Figma, a partir desta especificação:

- Página "Telas": 9 quadros de 1280×800 no tema claro: 01a, 01b e 01c (os três passos do onboarding), 02 Dispositivos, 03 Pareamento, 04 Layout de telas, 05 Clipboard, 06 Diagnóstico, 07 Preferências. Tudo em camadas editáveis com auto layout.
- Seção "Componentes": Botão (Primário, Secundário, Perigo, Discreto, propriedade de texto "Rótulo"), Interruptor (Ligado, Desligado), Selo de estado (ok, aviso, erro, neutro, acento, propriedade "Texto"), Tecla de atalho ("Teclas") e Barra lateral (240 px, navegação e rodapé de estado).
- Variáveis: coleções "Cor claro" e "Cor escuro" com os mesmos 16 nomes (fundo, borda, texto, acento, estado) e "Espaço" (4 a 40, raio 8). O plano Starter permite só um modo por coleção, por isso duas coleções em vez de dois modos.
- Fontes: Inter no corpo, Geist nos títulos, Geist Mono em código, ids e valores.

Conferido com captura: as 9 telas claras. 01b, 03, 05, 06 e 07 pela captura do MCP; 01a, 01c, 02 e 04 pelo portal do Maestri (02 e 04 depois de refeitas: a coluna Nome estava esmagada e o monitor do notebook, cortado).

Ajustes aplicados no arquivo em 2026-10-27 pelo plugin Scripter, com o script `docs/telas/figma-ajustes.js`: `Win+Alt` em todas as teclas de atalho, malha de 16 px no canvas de 04 Layout de telas, quebra de linha do rótulo "É controlada por esta máquina" em 02 e rótulo do notebook encurtado em 04. Conferido por captura do portal (tela 04). O protótipo HTML em `docs/telas/index.html` tem as mesmas mudanças.

Tema escuro: os 9 quadros escuros ("· Escuro", em y = 900) foram criados em 2026-10-27 pelo plugin Scripter com `docs/telas/figma-escuro.js`, religando as cores para a coleção "Cor escuro". Conferido por captura do portal (05 Clipboard · Escuro). Nada pendente no Figma. Parou no limite do MCP do Figma: o plano Starter dá 20 chamadas por mês, todas usadas em 2026-10-27. Caminho sem esperar o mês virar: abrir o arquivo no Figma, rodar o plugin Scripter e colar o script do tema escuro.

## Revisão de UX/UI pós-v0.3.1 (2026-10-28)

**Adendo novo, sem alterar a especificação acima.** A revisão do andar UX & UI, com fonte integral em `/tmp/clipsync-uxui-fecho-final.txt` (consolidado nas linhas 24-56), tem veredito **PRECISA REVISAR** no recorte de clareza e recuperação da interface. A base é `d41aa65`, com o Figma e o simulador vistos em portais separados. A evidência é do andar UX & UI. Não é veto retroativo ao release publicado nem veredito de travessia física; nenhum BLOCKER foi demonstrado nessa leitura limitada.

### Seis P1 · propostas para implementação futura

Os seis itens abaixo são propostas para uma implementação futura, não mudanças feitas nesta revisão.

- **P1 · A1/R1.** Preferências promete alterar módulos, mas não há controle lá; quem desliga clipboard ou input fica sem saída. Evidência: `onboarding.js:198`, `travessia-vista.js:12-19` e `prefs.rs:85-93`; no cenário `?cenario=input-desligado#/dispositivos`, "Abrir preferências" não leva a um controle de módulo. Refazer o perfil não é troca inofensiva: recria a identidade e exige novo pareamento, então não é o caminho para trocar um interruptor.
- **P1 · A2.** A permissão de controle não diz qual máquina concede; o painel é igual nos dois lados, então é possível conceder do lado errado. Evidência: `pareamento.js:327-373` e `dispositivos.js:167-187`; `devices.rs:61-77` não traz o que o par concedeu.
- **P1 · R2.** O selo de atalhos "ativos nesta máquina" aparece sem ligação local, porque deriva de capacidade. Evidência: `preferencias.js:102` e `status.rs:112-124` (`cfg!(windows) && input_live`); em `?cenario=sem-ligacao&plataforma=windows#/preferencias`, o selo aparece sem passagem configurada.
- **P1 · M1.** Na máquina que recebe o controle, o botão leva a configurar o Layout local, que grava a ligação invertida. Evidência: `pareamento.js:410-411` e `dispositivos.js:249`; o texto manda configurar no computador com o mouse físico, mas o CTA abre o Layout desta máquina.
- **P1 · M2.** A prévia da posição não diz de que lado o mouse sai nem de que lado entra; em `padrao#/layout`, "Atualizar passagem: eDP-1 para HDMI-1" só troca o monitor em "À direita", e "À esquerda", "Acima" e "Abaixo" repetem o mesmo texto. A diagonal de "Abaixo" é medida do Lens, não reproduzida.
- **P1 · M3.** Em 960x632, "Ligar nessa posição" fica abaixo da dobra: em `windows#/layout`, o main tem `scrollHeight` 1127 e o botão top 934, 302 px abaixo da dobra, enquanto o canvas manda clicar nele.

### P2 e P3

- **P2 · R3.** Texto de Linux no onboarding do Windows: `?cenario=sem-perfil&plataforma=windows#/onboarding/2` mostra "portal InputCapture" e "X11 ou pelo portal".
- **P2 · Pixel-1 / m9.** O rótulo do layout cobre o monitor vizinho: medida [642,253 269×17] sobre HDMI-1 [528,258 180×101]; m9 é o mesmo defeito, com 8 px em 960.
- **P2 · Pixel-2.** O Diagnóstico rola a janela inteira: viewport 1280×792, documento 911, e `scrollTo(0,600)` desloca a lateral de 0 para -119.
- **P2 · M4.** Recuperação com três avisos e estado contraditório depois de "Iniciar agente"; reproduzido só no simulador, onde o app real pode ajustar o estado pelo `peer_connected`.
- **P2 · M6.** Atalho recusado anunciado no botão pausado: em `windows-recusa`, a lateral oferece "Retomar tudo Win+Alt+P", mas o aviso de recusa só existe em Preferências.
- **P3 · A3, A4, A5 e m1-m13.** A3 é endereço:porta (`pairing.rs:125-142`); A4 é confirmação ao vivo na revisão do andar, e depois de criar o perfil, Voltar e "Criar perfil e continuar" abre "Refazer o perfil desta máquina?"; A5 é a terminologia; m1-m13 incluem o m9.

**Divergência de severidade:** o Pixel classificou PUI-03 e PUI-04 como P1; o consolidado manteve P2. PUI-05, PUI-06, PUI-07 e PUI-11 são preservação da v0.3.1, não defeito.

### Acessibilidade

A consolidação tem 42 entradas brutas (17 do Access, 12 do Meridian e 13 do Echo), conferidas ID a ID; os seis P1 de acessibilidade são a lista deduplicada por fluxo, não o total bruto, e listas duplicadas não devem ser somadas. Reproduzidos pelo Argus: P1-1, P1-3, P1-4 e N1; P1-2 não foi reproduzido no "Pausar tudo".

### O que preservar

**O que não regredir:** posição com "Ligar nessa posição"; estado da passagem por par; aviso de protocolo; orientação a quem só recebe; teste explicado; Ajustes avançados; versão real na lateral.

**O que não copiar do Figma antigo:** clipboard ligado por padrão; "É controlada por esta máquina" no sentido inverso; borda obrigatória com "Salvar layout"; "QUIC com mTLS" no onboarding; dados fictícios.

**Limites:** temas escuros restantes; onboarding em 960; frame Componentes do Figma sem captura válida; prova assistiva e teste físico Windows/Linux não verificados; M4 só no simulador.

## S9 · estados das telas (2026-10-30)

> **Leitura histórica, marcada em 30/10/2026 pela S10.** Este adendo descreve a S9 antes do fechamento documentado em `docs/workboard.md` e da publicação da v0.4.0 (`docs/evidencias/release-v0.4.0.md`). A linha de B07 em "O que segue sem aprovação" registra aquele momento e não é o estado atual: a revisão integrada da S9 foi aprovada e o FIX1 de B07 foi preservado nela. Nada disso é prova física, de travessia ou de tela em app nativo. Os achados e as evidências abaixo seguem como foram registrados.

**Adendo novo, sem alterar a especificação acima nem a revisão de 28/10.** Registra o efeito das fatias C01, B09 e B01 da S9 nas telas, conforme `docs/contrato.md` §15.7, §15.8 e §15.10. Descreve comportamento de interface; o aceite de produto continua separado, com app Tauri real, leitor assistivo, zoom e travessia física pendentes (§15.4 item 7 e §15.6).

### 1. Onboarding, passo "O que sincronizar" (C01)

Com perfil já criado, o passo mostra o aviso de que a máquina já tem perfil, que os módulos podem ser revisados sem alterar identidade, pares ou permissões, e o botão passa a ser "Aplicar módulos e continuar", usando a mesma edição de Preferências. Criar o perfil continua sendo recriação de identidade, com novo pareamento, e não é o caminho para trocar um interruptor (§15.7 item 4). Depois de aplicar, a reconsulta atualiza o estado e não retoma o agente: retomar segue explícito.

### 5. Clipboard (B09)

O seletor de modo ganha a frase que faltava: se a troca reiniciar o agente deste app, a malha fica pausada até "Retomar tudo". O aviso é anterior à troca, não posterior. Com agente externo, a tela salva a configuração e pede o reinício externo, sem afirmar que o modo já vale no processo em execução.

Aviso de pausa pendente: o topo do conteúdo, acima da tela aberta, mostra texto persistente quando `app_state.pause_warning` vem preenchido, distinguindo "aplicada em memória, não salva" de "não confirmada". `paused: null` é estado não observado, nunca retomado por dedução. Aviso antigo não é apagado por polling nem por resposta anterior à falha atual.

### 6. Diagnóstico, bloco "Últimos bloqueios" (B01)

A tabela passa a ter primeira ocorrência, última ocorrência e ocorrências observadas, agrupadas pelo agente por par, módulo e motivo, com no máximo 20 grupos. Repetição soma na contagem em vez de criar linha nova; chegada nova move o grupo para o topo; causa nova expulsa o último. `occurrences` ausente vale 1, `last_unix_ms` ausente cai para `at_unix_ms` e contagem acima do inteiro seguro do JavaScript aparece como "pelo menos N". Aviso de bloqueio tem cadência própria da casca, com resumo após pelo menos 30 s monotônicos, e o bloco continua mostrando a história mesmo com a notificação desligada. No simulador, `?cenario=b01-grupos` mostra legado sem agregação, grupos repetidos, causa geral sem par e contador no teto.

### 7. Preferências (C01 e B09)

Novo cartão "Módulos", com os dois interruptores, "Aplicar módulos" e resumo dos efeitos devolvidos pelo agente. `changed: false` aparece como "já estavam configurados assim". Erro mostra a mensagem do agente com os efeitos e o estado observado, e o botão só volta depois de reconsulta. Aplicar com o agente próprio reinicia pausado e restaura a trava desta tela; retomar é ação explícita. Cada atalho da lista mostra disponibilidade e motivo observados, sem prometer registro ou atividade dos hooks.

### O que segue sem aprovação

**Pareamento:** o round4, com foco na espera, anúncio terminal e caminho de corrigir endereço, foi aprovado pelos revisores nativos, com relatório nos ledgers. A especificação da seção 3 desta página continua sendo o alvo de produto; aprovação de revisão não é prova em portal, app real ou duas máquinas.

**B07:** o ciclo de vida da captura independente dos dados continua em revisão, sem aceite. Nenhuma tela deve ser descrita como prova desse comportamento.

**Limites:** o simulador escreve `effects`, `observed`, `recent_blocks` e `pause_warning` à mão. `?cenario=b01-grupos` cobre repetições e contador no teto, mas não a agregação dinâmica, a cadência de 30 s nem o teto de 20 grupos; nenhum cenário reinicia processo. Medição de portal, zoom real, leitor assistivo e duas máquinas continuam fora desta rodada.

## S10 · clareza da disponibilidade (30/10/2026)

**Adendo novo, sem alterar a especificação acima nem os adendos de 28/10 e da S9.** Registra o efeito da S10 nas telas conforme `docs/contrato.md` §16.1 e a spec `docs/superpowers/specs/2026-10-30-clareza-disponibilidade-design.md`. Vale para a fonte deste repositório, que está à frente da v0.4.0 publicada (commit `6cc1384`, evidência em `docs/evidencias/release-v0.4.0.md`): os instaladores publicados não têm esta clareza. Nenhuma API, dependência, capability, protocolo, grant ou regra de habilitação muda nesta fatia.

### 1. Por que um controle está desabilitado

A regra antiga, "recurso sem backend aparece desabilitado com `indisponível nesta versão`", era ampla demais para explicar motivo. Recurso futuro sem backend continua visível e desabilitado, como manda §12.2 regra 6. Dentro de um ramo já desabilitado de Dispositivos e Pareamento, a descrição passa a separar três causas, nesta ordem: suporte explicitamente ausente (`input_supported === false`), módulo local comprovadamente desligado (`modules.input === false`) e estado não confirmado. Só a segunda orienta conferir Módulos em Preferências. Bloqueio de confiança e etapa do pareamento mantêm precedência. O helper é `crates/clipsync-app/ui/js/disponibilidade.js`, chamado apenas nesses ramos; os gates e os handlers continuam os mesmos.

### 2. Disponibilidade remota não é prova de módulo ausente

Booleano remoto falso deixa de ser lido como "o par não tem o módulo". Em Dispositivos, `remote_modules` falso nos dois módulos passa a "Não foi possível confirmar os recursos de {nome}." Em Pareamento, o motivo agregado de input passa a "Não foi possível confirmar a disponibilidade de teclado e mouse para este pareamento.", sem atribuir a causa ao outro computador. Conexão e versão conhecidas não demonstram o estado atual do módulo remoto, e a disponibilidade agregada dos dois lados não ganha causa exclusiva.

### 3. Controle remoto: orientação estática, autorização no destino

O interruptor sempre falso "Controlar {nome} a partir deste computador" sai de Dispositivos e de Pareamento e dá lugar a orientação estática no mesmo grupo: "Para controlar {nome} a partir deste computador, autorize este computador na tela Dispositivos de {nome}. O ClipSync não informa aqui se essa autorização já foi concedida." A autorização é feita no destino e a tela não afirma conhecê-la. O controle real de conceder entrada nesta máquina, seu valor e a confirmação de confiança total permanecem iguais; texto, navegação e teste não criam grant.

### 4. Clipboard com o módulo desligado não avalia plataforma

`enabled === false` mantém o aviso de módulo desligado e mostra a coluna Plataforma como "não avaliada com o módulo desligado", em tom neutro, sem inferir incompatibilidade. Ausência do booleano aparece como módulo "não confirmado" e "estado da plataforma não confirmado", que é estado desconhecido, não desligado. Com `enabled === true`, a classificação recebida continua aparecendo, sem afirmar validação física.

### 5. Diagnóstico não promete latência

O estado vazio de pares deixa de prometer latência e passa a orientar acompanhar estado e endereço disponível. A nota de métricas diz que as medições por computador ainda não foram implementadas. Esta versão não mede RTT nem latência de entrada por par, e o pareamento não deve prometer isso ao concluir.

### 6. Nomes longos quebram sem truncamento

`css/telas/dispositivos.css` e `css/telas/pareamento.css` ganharam `overflow-wrap: anywhere` nos textos de permissão com nome e no título da lista. A altura cresce e o texto aparece inteiro: não há corte, reticências, `overflow` oculto nem redução de fonte, e os interruptores e ações continuam alcançáveis.

### Evidência e limites

A revisão independente do delta de tela aprovou spec, qualidade e segurança dos textos, sem achado Critical, Important ou Minor, e confirmou que nenhum motivo nulo virou não nulo nem o inverso. A revisão estática do CSS também foi aprovada, com o reteste geométrico reservado ao portal. O reteste focal mediu no DOM, em temas claro e escuro e em 1280×800, 960×640 e 320×640, contenção do conteúdo e quebra dos nomes: 12/12 combinações no Pareamento e no Dispositivos, mais 3/3 no estado vazio, com restauração dos nomes sintéticos, sem conceder controle nem concluir pareamento.

**Limites:** essas medições são geometria de DOM em simulador, não raster, contraste, foco real, leitor assistivo, app Tauri ou travessia física; o histórico de 0/30 aceites visuais integrais da Janela A permanece. O app real, o Windows nativo e o teste em duas máquinas não foram verificados nesta rodada. A v0.4.0 publicada não contém esta clareza.
