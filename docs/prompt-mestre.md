# Prompt mestre v4 · ClipSync: teclado, mouse e clipboard entre meus computadores

> Revisado em 2026-10-27, 11h40. Esta v4 **vence** v3, v2 e o original (`prompt_mestre_clipsync.md`, hoje `docs/prompt-mestre-anexo-original.md`). O original continua como anexo detalhado do módulo de clipboard (seções 5 a 39). Novidades da v4: módulos opcionais (§13), o MVP testável hoje (§14) e as telas do Figma (§15).
> "ClipSync" é nome de código. Renomear antes do primeiro release, porque o nome já não descreve o produto.

## 0. O que mudou da v2 para a v3

O usuário decidiu que quer **um único aplicativo** no lugar de Input Leap + ClipSync. Dois motivos concretos: o clipboard do Input Leap só sincroniza na troca de tela, e o Input Leap trata cada PC como um retângulo só, então com dois monitores o cursor só passa para a outra máquina no fim do desktop inteiro. O produto passa a ter dois módulos sobre uma base comum. O requisito novo central é **borda por monitor** (e por segmento de borda), não por dispositivo.

## 1. Forma do produto: um aplicativo desktop, dois módulos, uma base

Um pacote por máquina, instalado por usuário, sem root, rodando na sessão gráfica. Um processo no início (base + módulos + bandeja + CLI); separar processos só se a prova técnica exigir, especialmente para isolar as permissões do módulo de entrada.

- **Base comum:** identidade criptográfica do dispositivo, pareamento, permissões por par, por direção e **por módulo**, malha de conexões, configuração versionada, diagnóstico, bandeja, janela de gerenciamento, CLI.
- **Módulo Clipboard:** como na v2 §2 (observar, política, enviar, validar, aplicar ou caixa de recebidos, ciclo, pausa, privado).
- **Módulo Input (teclado e mouse):** captura no dispositivo que tem o teclado e o mouse físicos, emulação nos outros, troca de máquina por barreiras configuradas em bordas de monitor ou por atalho.

Não é plugin: Input Leap não tem API de extensão, extensão de navegador não enxerga nem clipboard nem entrada global, serviço de sistema não acessa a sessão do usuário.

Enquanto o módulo Input não estiver validado nas três máquinas, o Input Leap continua em uso. Os dois **não podem capturar ao mesmo tempo**: o diagnóstico detecta Input Leap ou Barrier ativos e orienta a pausa, sem encerrar nada sozinho.

## 2. Como funciona

### 2.1 Base

1. Primeira execução: gera par de chaves e certificado autoassinado, pede nome amigável, explica o alcance de cada módulo. Nada é transmitido.
2. Pareamento: A aceita pareamento por tempo limitado; B entra com endereço manual. Conexão QUIC com TLS 1.3 mútuo e certificados autoassinados; os dois lados mostram o mesmo código numérico curto derivado do material da sessão (`export_keying_material`). Confirmação nas duas pontas grava o par.
3. Permissões por módulo, concedidas **no dispositivo que cede**: "pode me enviar clipboard", "pode receber meu clipboard", "pode controlar meu teclado e mouse". Todas desligadas por padrão; a de controle exige confirmação separada e aparece como estado visível na bandeja enquanto ativa.
4. Malha completa (3 máquinas = 3 ligações), re-autenticação a cada reconexão, chave diferente gera alerta e bloqueio, conexões duplicadas resolvidas pelo id, backoff com aleatoriedade.

### 2.2 Módulo Clipboard

Igual à v2 §2, passos 4 a 7. Acréscimo: como o módulo Input sabe qual máquina está ativa, o clipboard ganha uma política opcional "sincronizar ao trocar de máquina" (comportamento do Input Leap) além do modo "sempre, para os pares autorizados". O padrão continua "sempre".

### 2.3 Módulo Input

1. **Monitores reais:** cada dispositivo publica seus monitores (nome, geometria lógica, escala, posição no desktop virtual). Mudanças de monitor (conectar, desconectar, reordenar) republicam e invalidam barreiras obsoletas.
2. **Layout por borda de monitor:** o editor mostra os monitores reais de cada máquina. O usuário liga **um segmento de borda de um monitor** a **uma borda de um monitor de outro dispositivo**. Exemplo do usuário: "borda inferior só do monitor 2 do PC leva ao notebook". Bordas entre dois monitores locais nunca cruzam, salvo configuração explícita. Cantos mortos e segmentos parciais são suportados. No início, o layout pode ser um arquivo TOML; o editor gráfico entra com a UI Tauri.
3. **Quem guarda o layout:** cada dispositivo guarda as barreiras das suas próprias bordas (o que ele precisa para capturar). "Copiar layout para os pares" é opcional. O destino só precisa saber em que borda e em que posição normalizada (0 a 1 ao longo do segmento) o cursor entra, para mapear resoluções e escalas diferentes.
4. **Captura (origem):** barreiras só nas bordas configuradas. Ao cruzar, o sistema entrega os eventos ao ClipSync, o cursor local some, e os eventos vão ao destino. Wayland: portal `InputCapture` (zonas por monitor + barreiras) com eventos por EIS/libei. Windows: hooks de baixo nível de mouse e teclado, barreira calculada nas coordenadas do desktop virtual. X11: barreiras XFixes + XInput2 raw events. macOS: `CGEventTap` com limites por display, permissão de Acessibilidade.
5. **Emulação (destino):** Wayland: portal `RemoteDesktop` + libei. Windows: `SendInput`. X11: XTest. macOS: `CGEvent`. Cursor visível no destino; posição inicial na borda de entrada.
6. **Volta e atalhos:** voltar cruzando a borda correspondente no destino (barreira virtual calculada a partir da posição emulada) ou por atalho. Atalhos configuráveis: ir direto a um dispositivo, "travar nesta tela", pausar tudo. Ao trocar, soltar teclas e botões pressionados e reconciliar modificadores.
7. **Teclado:** transmitir keycodes evdev e o keymap XKB da origem quando o destino aceitar keymap (libei aceita); nos demais, tabela de tradução com testes por layout. Testar **ABNT2** (ç, acentos com dead keys, AltGr) e US. Arrastar com botão pressionado através da borda deve funcionar; rolagem suave e alta resolução preservadas; movimento relativo (jogos) fica experimental.
8. **Tela bloqueada e sessão inativa:** destino bloqueado rejeita entrada remota; origem para a captura ao bloquear. Windows: nunca injetar no secure desktop (UAC). Pausa global para os dois módulos na bandeja e por atalho.

## 3. Premissas: medidas, a preencher e assumidas

### Medidas em 2026-10-27 no Notebook (`kappxs-Aspire-A515-57`)

```text
SO:                  Ubuntu 26.04.1 LTS, GNOME Shell 50.1, Wayland, XWayland ativo (DISPLAY=:0), 1 monitor (eDP-1, escala 1)
Portais:             InputCapture v1 (capacidades: teclado, ponteiro, touchscreen), RemoteDesktop v2, Clipboard v1
                     xdg-desktop-portal 1.21.1, xdg-desktop-portal-gnome 50.0
Clipboard Wayland:   wl_data_device_manager v3 e primary_selection; SEM zwlr_data_control nem ext_data_control
                     -> observação em segundo plano depende da ponte XWayland (XFixes) ou de extensão GNOME Shell
Input Leap:          3.0.3 Flatpak, rodando como CLIENTE do servidor 192.168.115.30:24800, flag --disable-crypto
                     Barrier 2.4.0 (apt) também instalado. Binário 3.0.3 contém clipboardSharing / clipboardSharingSize
Toolchains:          Node 24.15, Python 3.14; SEM Rust; sudo exige senha; build-essential presente; 168 GB livres
Deps Tauri:          ausentes (libwebkit2gtk-4.1-dev, libgtk-3-dev, librsvg2-dev, libayatana-appindicator3-dev, pkg-config)
```

### Informado pelo usuário

```text
O PC servidor do Input Leap (192.168.115.30) tem 2 monitores. O cursor só cruza para o notebook no fim do desktop inteiro.
```

### A preencher (não bloqueiam o nível 1)

```text
PC pessoal e PC de casa: SO e versão? sessão gráfica (Wayland, X11, Windows)? qual tem o teclado e o mouse físicos no uso normal?
Rede: os três na mesma LAN 192.168.115.0/24? VPN existente entre casas?
Nome definitivo do produto.
```

### Assumidas até prova em contrário

- Pelo menos um PC é Windows; adaptadores Windows ficam atrás de `cfg(windows)` e marcados não testados até rodar lá.
- Mesma LAN no cenário inicial; rede diferente só por VPN já autorizada.
- Input Leap não é alterado sem pedido; só diagnóstico e orientação.

## 4. Por que não ficar com o Input Leap

Clipboard só na troca de tela; borda por dispositivo, não por monitor; papéis fixos de servidor e cliente; sem pausa, modo privado, direção ou caixa de recebidos; criptografia desligada no setup atual do usuário. O objetivo do ClipSync: **um app só** que faça teclado, mouse e clipboard entre as minhas máquinas, com borda por monitor e sem servidor central.

## 5. Stack e ambiente de desenvolvimento

- **Núcleo em Rust.** Crates previstos (confirmar versões e manutenção na ADR): `tokio`; `quinn` (QUIC) com `rustls` e `rcgen`; `arboard` (clipboard; backend X11 via `x11rb`, Wayland via `wl-clipboard-rs`); `ashpd` (portais InputCapture e RemoteDesktop); `reis` (libei/EIS em Rust); `windows` (hooks, `SendInput`, monitores); `x11rb` (XFixes, XInput2, XTest); `core-graphics` no macOS; `serde` + formato binário com limites para o protocolo. Nada disso exige lib de sistema para compilar no Linux além do que já existe; `rustup` em espaço de usuário; versão fixada em `rust-toolchain.toml`.
- **Transporte: QUIC desde o início.** Um transporte só, que serve os dois módulos: fluxos confiáveis para clipboard e datagramas para eventos de entrada, TLS 1.3 mútuo embutido, migração de conexão quando o IP do notebook muda. A ADR registra a comparação com TLS/TCP + UDP separado e o motivo da escolha.
- **UI em Tauri 2 + TypeScript** (janela, bandeja, editor de layout). Entra quando o usuário rodar:
  `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev pkg-config`
  Até lá, CLI e arquivo de configuração. WebView recebe só dados necessários; criptografia, rede, política, captura e emulação ficam no núcleo; capabilities e IPC explícitos.
- **Persistência:** TOML versionado para configuração e layout; segredos no cofre da plataforma; SQLite só quando o histórico opcional entrar.
- **Licença:** decidir antes do primeiro commit. Lan Mouse (Rust, GPL-3) é referência de leitura para portais e libei; copiar código dele obriga GPL-3 no projeto inteiro. Preferir escrever sobre as crates diretamente.
- Organização: workspace Cargo com crates por fronteira (base, clipboard, input, agente/CLI; depois `apps/desktop`). Árvore exata é do Arquiteto, no contrato.

## 6. Nível 1 · provas técnicas, todas neste notebook

Instâncias múltiplas usam `--profile <dir>` e adaptadores **falsos** de clipboard e de entrada. Adaptadores reais rodam em instância única.

### P1 · Clipboard no GNOME 50 / Wayland
Igual à v2 §6 P1: `probe clipboard --watch`, `probe clipboard --write`, ponte XWayland como caminho primário, primeira linha da matriz. Se falhar, abrir a tarefa da extensão GNOME Shell.

### P2 · Base: pareamento e malha QUIC
Duas instâncias em `127.0.0.1`: pareamento por comparação numérica; gravação do par; terceira instância desconhecida recusada com motivo; reconexão re-autentica; chave trocada gera alerta. Demonstrar um fluxo confiável e um datagrama no mesmo canal. Se `tshark` ou `tcpdump` existirem, mostrar que o texto sintético não aparece em claro; senão, registrar como não verificado.

### P3 · Ciclos e política do clipboard
Igual à v2 §6 P3, com adaptador falso em três instâncias e testes em `cargo test`.

### P4 · Entrada no GNOME 50 / Wayland, numa máquina só
- **P4a captura:** `probe input --capture`: via portal InputCapture, listar zonas (deve aparecer `eDP-1`), criar barreira **só na metade superior da borda direita** (prova de segmento parcial), habilitar, encostar o mouse: evento `Activated` e eventos de ponteiro e teclado chegando por EIS; `Release` devolve o cursor. A CLI imprime contagem e latência, **nunca teclas**. Diálogo de permissão do portal documentado.
- **P4b emulação:** `probe input --emulate`: via portal RemoteDesktop + libei, mover o cursor em um quadrado e digitar texto sintético (com ç e acento) numa janela de teste aberta pelo usuário, com confirmação antes.
- **P4c ponta a ponta local:** processo A captura e envia por QUIC (datagramas) a processo B no mesmo notebook, que só registra contagem e latência (sem emular, para não realimentar a barreira). Medir p50 e p95 captura → recepção.
- Saída: linhas "input: captura" e "input: emulação" na matriz para Ubuntu 26.04 / GNOME 50.1 / Wayland, com classificação honesta e limitações observadas.

### Critério de pronto do nível 1
`cargo build`, `cargo test`, `cargo clippy`, `cargo fmt --check` verdes; os quatro `probe` executados neste notebook com saída colada no relatório; `docs/matriz-compatibilidade.md`; ADRs de stack, transporte e licença; relatório no formato da §10.

## 7. Nível 2 · MVP em duas entregas, depois da base

- **2A Clipboard:** texto Unicode entre as três máquinas, convite de uso único, modos automático, manual e com confirmação, direção por par, reconexão, diagnóstico, bandeja e janela Tauri, adaptador Windows testado em máquina real, guia das três máquinas.
- **2B Input:** teclado e mouse entre as três máquinas; layout por borda de monitor (arquivo primeiro, editor gráfico depois); atalhos básicos; tela bloqueada; pausa; ABNT2 validado; adaptadores Windows e X11 ou Wayland testados nas máquinas reais; checagem de conflito com Input Leap e Barrier.
- 2A e 2B podem andar em paralelo com builders diferentes. 2A tende a ficar utilizável antes.

**Fora do MVP:** histórico persistente, imagens e formatos ricos, arquivos, arrastar e soltar entre máquinas, descoberta mDNS, relay, VPN além da já existente, touchscreen, gestos, movimento relativo para jogos, clientes móveis, extensão GNOME Shell (salvo se P1 exigir).

## 8. Segurança do módulo Input

Captura global de teclado e mouse é a natureza deste módulo. Isso reescreve, só para ele, as seções 3, 18 e 25 do original:

1. Permissão "pode controlar este dispositivo" é por par, desligada por padrão, concedida e revogável **no dispositivo controlado**, com indicador visível enquanto houver controle remoto ativo.
2. Conceder controle é confiança total naquele par: ele pode digitar e clicar qualquer coisa. A interface diz isso com todas as letras no momento da concessão.
3. Destino nunca aceita entrada com tela bloqueada, sessão inativa, secure desktop ou módulo pausado. Origem para a captura ao bloquear.
4. Interrupção local sempre disponível: atalho e bandeja param captura e emulação na hora.
5. Nenhum evento de entrada vai para log, nem keycodes; logs têm contagem, latência e estado. Nenhum modo de depuração grava teclas.
6. Eventos de entrada só trafegam no canal QUIC autenticado, só entre pares com a permissão de controle, com limite de taxa, validação de coordenadas e descarte do que estiver fora da zona.
7. O módulo Input fica isolado em processo ou capability própria; o módulo Clipboard não herda permissão de entrada. No macOS, só o módulo Input pede Acessibilidade.
8. Clipboard permanece com as regras da v2 §8, todas.

## 9. Regras que não se negociam

As dez da v2 §8, mais as oito da §8 acima. Em conflito, segurança por padrão vence conveniência, e o usuário é avisado do custo.

## 10. Como os agentes reportam

Item por item: **implementado e testado aqui** (comando e saída), **implementado, depende de outra máquina**, **planejado**. Sem número inventado, sem "funciona no Windows" antes do Windows. Conflitos entre requisitos resolvidos por escrito em ADR.

## 11. Backlog experimental

Editor de layout com arrastar e soltar; descoberta mDNS; relay cego ao conteúdo; histórico de clipboard; imagens e formatos ricos; transferência de arquivos e arrastar e soltar entre máquinas; extensão GNOME Shell para clipboard; touchscreen e gestos; movimento relativo para jogos; convite por QR; clientes móveis como controle remoto manual.

## 12. Ordem de execução

1. Arquiteto: contrato (workspace, crates, nomes, versões fixadas, interface dos adaptadores de clipboard e de entrada, formato do evento, quadro do protocolo sobre QUIC, formato do layout) e as fatias P1, P2, P3, P4 no workboard. ADRs de stack, transporte e licença.
2. Setup: `rustup` em espaço de usuário, workspace, lint, fmt, teste vazio verde, `git init`, CI que compila sem sessão gráfica e marca os testes que exigem sessão.
3. Builders em paralelo, um por fatia: P1, P2 (base), P3, P4. Cada um dono dos seus arquivos, conforme o contrato.
4. Warden executa os quatro `probe` e os testes e posta veredito com evidência. Segurança revisa P2 e P4 antes de qualquer "pronto".
5. Relatório ao usuário no formato da §10, com 2A e 2B já fatiadas.

## 13. Módulos opcionais: o usuário escolhe o que sincronizar

Decisão do usuário em 2026-10-27: o aplicativo precisa oferecer a opção de **usar só teclado e mouse, sem sincronizar a área de transferência**.

- Configuração versionada: `[modules] input = true | false` e `[modules] clipboard = true | false`. Três combinações válidas: só teclado e mouse; teclado, mouse e clipboard; só clipboard. Pelo menos um módulo ligado.
- Onboarding (`clipsync init` hoje, tela Tauri depois) pergunta, com uma explicação curta do alcance: "Quer sincronizar também a área de transferência entre as máquinas?" O padrão é **não** até o usuário responder.
- Módulo desligado não carrega adaptador, não pede permissão de sistema, não anuncia a capacidade aos pares, não envia nem aceita eventos daquele tipo. O par que tentar envia recebe recusa com motivo legível.
- Capacidades são negociadas por conexão: ligar ou desligar um módulo renegocia com os pares. No MVP, mudar exige reiniciar o agente; a UI avisa.
- Permissões por par continuam por módulo (§2.1). Módulo ligado localmente não significa autorizado para todo par.
- `clipsync status` mostra, por módulo: ligado ou desligado, adaptador em uso, pares autorizados, último evento e motivo do último bloqueio.

## 14. MVP testável hoje (2026-10-27)

"Testável hoje" significa: um binário `clipsync` (CLI) compilado neste notebook, que o usuário consegue exercitar sozinho em dez minutos, com evidência real. Sem Tauri hoje, salvo se o usuário instalar as libs de sistema (§5). Repositório privado `devkappiushon/clipsync` no GitHub, commits em português.

### Roteiro de teste do usuário (o que precisa funcionar)

1. `clipsync init --profile ~/.clipsync-a` pergunta nome e módulos e grava identidade e configuração. Repetir com `~/.clipsync-b`.
2. `clipsync pair host --profile ~/.clipsync-a` e `clipsync pair join --profile ~/.clipsync-b 127.0.0.1:PORTA` mostram o mesmo código numérico nas duas pontas; confirmar nas duas grava o par com permissões por módulo.
3. `clipsync run --profile ~/.clipsync-a` e `... -b` sobem os dois agentes; `clipsync status` mostra os dois conectados, módulos e permissões.
4. Clipboard (se ligado): `clipsync probe clipboard --watch 15s` registra cópias feitas em outro app (mime e tamanho, nunca conteúdo). `clipsync probe clipboard --write` escreve texto sintético, lê de volta idêntico e não dispara cópia nova. Entre as duas instâncias locais, `clipsync send --profile ~/.clipsync-a --stdin` entrega um texto à instância B, que o mostra na caixa de recebidos (`clipsync inbox --profile ~/.clipsync-b`). Com `clipboard = false` no perfil B, a mesma tentativa é recusada com motivo.
5. Input: `clipsync probe input --capture` cria barreira só na metade superior da borda direita do `eDP-1`, e encostar o mouse ali dispara a captura; `Esc` ou atalho libera. `clipsync probe input --emulate` move o cursor num quadrado e digita "ção" numa janela que o usuário abriu, após confirmar. `clipsync probe input --loopback` mede p50 e p95 de captura → QUIC → recepção entre as duas instâncias locais.
6. Pausa: `clipsync pause --profile ~/.clipsync-b` bloqueia envio e aplicação; `resume` volta sem republicar nada.
7. `cargo test` verde no workspace; `clipsync diagnose` lista portais, protocolos Wayland, Input Leap e Barrier detectados.

### O que não estará hoje

Teclado e mouse entre duas máquinas reais (depende do sistema dos outros PCs); editor gráfico de layout; bandeja e janela Tauri; adaptadores Windows e macOS testados; mDNS; histórico; imagens; arquivos.

## 15. Telas no Figma

Protótipo das telas da UI Tauri, feito via Figma e mostrado num portal do Maestri. Sete telas, tema claro e escuro, desktop 1280×800:

1. **Onboarding:** nome da máquina, escolha de módulos (teclado e mouse; clipboard) com explicação de alcance, teste com conteúdo sintético.
2. **Dispositivos:** lista com estado, permissões por módulo e por direção, ações parear, pausar par, revogar.
3. **Pareamento:** endereço e porta, código numérico grande, "confirme na outra máquina", expiração.
4. **Layout de telas:** monitores reais de cada máquina em escala, ligação de segmentos de borda entre monitores de máquinas diferentes, cantos mortos, atalhos de troca.
5. **Clipboard:** estado do módulo, modo automático ou manual, caixa de recebidos com prévia recolhida, pausa e modo privado.
6. **Diagnóstico:** capacidades do sistema, adaptador em uso, portais, conflitos (Input Leap, Barrier), latência, últimos bloqueios com motivo.
7. **Preferências:** atalhos, inicialização com a sessão, notificações por tipo, tema, idioma.

Regras visuais: Inter no corpo, Geist nos títulos, Geist Mono em códigos e ids; um raio de borda; escala de 8 pt; sem roxo gratuito, sem emoji como ícone, sem travessão em texto de interface; estados não dependem só de cor; foco visível.
