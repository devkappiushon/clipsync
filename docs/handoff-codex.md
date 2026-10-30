# Passagem para o Codex · ClipSync

Escrito em 2026-10-28 pelo Maestro (Claude), que coordenou o time do Maestri de S1 a S7. Daqui em diante o trabalho segue com o Codex. As regras de trabalho estão no `AGENTS.md` da raiz; este documento conta o que existe, como chegou aqui e o que está aberto.

> **Histórico anterior à S8 e à v0.3.1.** O estado atual está em `CONTEXTO.md` e no workboard. As seções 6 e 7 descrevem o pedido que virou a S8, registrada na §14 do contrato; não são o estado de hoje.

## 1. Em uma página

- **O que é:** um app só, sem servidor central, que leva teclado, mouse e área de transferência entre os computadores do usuário, no lugar do Input Leap. Requisito central: a passagem do mouse é **por trecho de borda de monitor** (ex.: "a borda inferior só do monitor 2 do PC leva ao notebook"), não por máquina inteira.
- **Releases publicados:** v0.1.0 (CLI), v0.2.0 e v0.2.1 (app de desktop), **v0.3.0** (teclado e mouse no Windows e layout de telas entre Windows e Linux).
- **CI:** verde nos dois sistemas na base da v0.3.0 (`ed4b7e1`, run 40759799821).
- **Problema aberto agora:** o usuário pareou o PC Windows com o notebook Linux e **o mouse não passa**. Evidência e plano na seção 6. A tarefa pedida é achar a causa e **melhorar a usabilidade** (seção 7).

## 2. Máquinas e cenário do usuário

| Máquina | Sistema | Papel | Observações |
|---|---|---|---|
| Notebook `kappxs-Aspire-A515-57` (este repositório) | Ubuntu 26.04, GNOME 50 Wayland com XWayland, 1 monitor `eDP-1` | recebe teclado e mouse | sem sudo; perfil em `~/.config/clipsync` |
| PC `DESKTOP-LSSO4ID`, 192.168.1.11 | Windows | tem o teclado e o mouse físicos, 2 monitores | era o servidor do Input Leap |
| Terceiro PC | a confirmar | | pergunta aberta no contrato §13.14 |

O Input Leap fica instalado até o módulo de input do ClipSync rodar nas máquinas reais. Os dois não podem capturar ao mesmo tempo; hoje o Input Leap está parado.

## 3. Linha do tempo

1. **Nível 1, provas técnicas no notebook** (`docs/matriz-compatibilidade.md`, `docs/evidencias/`): clipboard pela ponte XWayland (XFixes), captura pelo portal InputCapture v1, emulação pelo portal RemoteDesktop v2 com libei.
2. **S1 a S4, MVP de linha de comando** (tag `mvp-2026-10-27`): protocolo, malha QUIC com mTLS por pinagem, pareamento com código de 6 dígitos, módulo de clipboard, módulo de input no Linux, CLI completa. Contrato §0 a §10.
3. **S5, Windows e release** (§11): agente no Windows com named pipe, clipboard do Windows por polling de 250 ms, input "indisponível" no Windows, workflow de release. **v0.1.0**.
4. **S6, interface visual** (§12): app Tauri 2 com o agente no mesmo processo, 9 telas desenhadas no Figma (arquivo "ClipSync · telas", chave `vkXY3Q37AaGe1argTA2STv`), bandeja, AppImage, `.deb`, `.msi` e `setup.exe`. Rodada de fidelidade ao Figma (`docs/telas/comparacao/`, 18 pares). Teste do app real pela linha de comando e pela acessibilidade (`docs/teste-app-real.md`). **v0.2.0** e **v0.2.1**.
5. **S7, layout de telas de verdade** (§13): input no Windows (crate `clipsync-win`, com `unsafe` isolado), protocolo v3, volta decidida no destino, atalhos Win+Alt, tela bloqueada, zonas de monitor em tempo real, teclado entre plataformas (ABNT2 e US). **v0.3.0**.

## 4. Arquitetura

### Crates

| Crate | Responsabilidade |
|---|---|
| `clipsync-proto` | tipos e mensagens (postcard), limites, validação, `PROTOCOL_VERSION = 3` |
| `clipsync-core` | identidade, perfil, TLS com pinagem, malha QUIC (quinn), pareamento, agente, canal de controle |
| `clipsync-clipboard` | módulo de clipboard: adaptador X11 (XFixes) e do Windows (polling), caixa de recebidos, modos auto, manual e com confirmação |
| `clipsync-input` | módulo de input: captura na origem, emulação no destino, layout, atalhos, tradução de teclas (`keymap.rs`), decisões do Windows (`adapter/winmath.rs`, `adapter/wincore.rs`), adaptadores do portal (Linux) e `adapter/win/` (Windows) |
| `clipsync-win` | chamadas Win32 com API segura; todo o `unsafe` em `src/ffi.rs` |
| `clipsync-cli` | comando `clipsync` (`init`, `pair host/join`, `run`, `status`, `send`, `inbox`, `diagnose`, `probe`, `pause`, `resume`) |
| `clipsync-ui` | lógica das telas: `Ui::call(UiRequest)` com cerca de 40 comandos, eventos, preferências (`ui.toml`), autostart |
| `clipsync-app` | casca Tauri 2: janela, bandeja, um comando `clipsync` e o evento `clipsync://event`; frontend em `ui/` (HTML, CSS e JS sem framework), simulador em `ui-dev/` |

### Rede e segurança

- Cada dispositivo tem identidade própria (`identity.key.pem` e `identity.cert.pem`). O `peers.toml` guarda o pino de cada par e as permissões dadas a ele (`clipboard_in`, `clipboard_out`, `input_in`).
- Pareamento por commitment no fluxo de controle; o código de 6 dígitos vem do exportador TLS; cada lado confirma. A primeira recusa de qualquer lado encerra os dois.
- Revogação avisada: quem revoga manda `Bye{Revoked}` a par v2+, e o outro lado remove o par também.
- Controle local: socket unix `DIR/agent.sock` no Linux; named pipe publicado em `DIR\agent.pipe` no Windows. A CLI e o app usam o mesmo canal. O app roda o agente no mesmo processo.

### Perfil

Linux: `~/.config/clipsync` (ou `$XDG_CONFIG_HOME/clipsync`, `$CLIPSYNC_PROFILE`, `--profile DIR`). Windows: `%LOCALAPPDATA%\clipsync`. Arquivos: `config.toml` (nome, módulos, porta 47800, modos), identidade, `peers.toml`, `layout.toml` (só na máquina que captura), `ui.toml` (preferências do app), `state.toml` (pausa).

### Input: como uma travessia acontece

1. A **origem** (máquina com o teclado e o mouse) lê o `layout.toml` e instala barreiras só nos trechos ligados. No Windows: hooks de baixo nível de mouse e teclado; no Linux: portal InputCapture.
2. O cursor empurra o trecho ligado: a origem esconde ou estaciona o cursor, solta localmente o que estava pressionado e abre a sessão com o destino (`EnterSpan` no protocolo v3; `EnterAt` com par v2).
3. O **destino** confere a permissão `input_in`, a pausa e a tela bloqueada (se bloqueado, responde `Refused` e a origem devolve o cursor). Emula pelo `SendInput` (Windows) ou pelo RemoteDesktop com libei (Linux).
4. A volta é decidida no destino, que conhece os próprios monitores: empurrar de volta pelo trecho de entrada manda `Returned`, e a origem solta o cursor no ponto correspondente.
5. Atalhos no Windows: Win+Alt+1 a 9 (ir ao dispositivo com aquele atalho), Win+Alt+L (travar nesta tela), Win+Alt+P (pausar; retomar pelo `RegisterHotKey` enquanto pausado). "Ir para dispositivo" chama `CaptureAdapter::begin`; no Linux o portal não permite e o botão fica desabilitado.
6. Os monitores de cada máquina viajam como `Zones` (no PeerUp e a cada mudança); a tela Layout desenha os monitores reais dos pares.

Formato do `layout.toml` (contrato §12.6):

```toml
version = 1
rev = 7
[options]
dead_corner_px = 8
[[device]]
peer = "<id de 64 hex>"
name = "pc-pessoal"
x = 0
y = 0
hotkey = 2
[[link]]
id = "l1"
a = { peer = "<id>", monitor = "HDMI-1", edge = "right", from = 0.5, to = 1.0 }
b = { peer = "<id>", monitor = "eDP-1", edge = "left", from = 0.0, to = 1.0 }
```

## 5. Estado verificado em 2026-10-28

- **S7, critérios do contrato §13.11:** 1 a 7 passam (vereditos do Argus em `5b5ff92` e `d00fa5f`; segurança do Sentinel em `a38d25c` e `520e031`, com S7-05 a S7-08 corrigidos). O critério 8, o roteiro do usuário (§13.12, 9 passos) e as 5 perguntas (§13.14), **não foram feitos**.
- **Provado só no CI do Windows:** hooks sobem e descem sem vazar handles, evento injetado nunca chega ao callback, pânico no callback passa o evento e avisa a perda uma vez, monitores e desktop lidos, `Win+Alt+P` aceito pelo `RegisterHotKey` no runner.
- **Nunca rodou numa máquina Windows real com pessoa usando:** o fluxo completo do worker (cruzar, estacionar o cursor, `SendInput`, troca de desktop). Risco registrado no contrato (decisões 20 e 21 da §13).
- **Linux real só nos simuladores:** trocar barreira com a sessão ativa, tela bloqueada de verdade, troca de monitor, `xkb_symbols` do EIS.
- **Pontos de atenção do roteiro:** o cursor pode errar 1 px; andar colado à borda pode ativar a travessia; o reaperto de Shift na volta depende do `GetAsyncKeyState`.

## 6. Problema aberto: o mouse não passa do PC para o notebook

Relato do usuário: "estou conectado no clipsync, notebook client e pc servidor, não consigo passar o mouse para o outro dispositivo, verifique o que está falhando e melhore a usabilidade".

### Evidência coletada no notebook

Com a CLI 0.3.0 do release em `~/.cache/clipsync-cli-v0.3.0/clipsync`, contra o agente do app em execução:

```
$ dpkg -s clip-sync | grep Version
Version: 0.2.1                         # o notebook roda o app 0.2.1 (.deb), não o 0.3.0

$ clipsync status
kappxs-Aspire-A515-57 · id 35a0-f4de-fbbb-167b · escuta 0.0.0.0:47800 · ativo
Pares:
  DESKTOP-LSSO4ID (8327-f0aa-7ee7-6a61) · conectado · 192.168.1.11:47800 · módulos do par: clipboard, input · permissões: clipboard-in, clipboard-out, input-in
Módulos:
  clipboard: ligado · adaptador x11-xfixes · último evento: oferta enviada a 1 par(es)
  input: ligado · adaptador portal-inputcapture+portal-remotedesktop · último evento: nenhum · último bloqueio: nenhum

$ clipsync diagnose   (trecho)
[ok] sistema · Input Leap: instalado (Flatpak), parado
[ok] input · portal InputCapture: versão 1
[ok] input · portal RemoteDesktop: versão 2; dispositivos Keyboard | Pointer | Touchscreen
[ok] input · tela bloqueada: desbloqueada (logind)
[ok] input · monitores: eDP-1
```

O `config.toml` do notebook tem `input = true` e `clipboard = true`. Não há `layout.toml` no notebook, o que é normal: o layout fica na origem (o PC).

### Leitura

- A conexão e as permissões estão certas: o par está conectado, o notebook deu `input-in` ao PC, e o PC anuncia o módulo input. Antes da v0.3.0 o Windows não tinha input, então o PC muito provavelmente está na v0.3.0.
- O input do notebook marca **"último evento: nenhum"**: o PC nunca abriu uma travessia. O problema está na origem, antes de qualquer mensagem chegar.
- Versões diferentes: PC v0.3.0 (protocolo 3) e notebook v0.2.1 (protocolo 2). A malha aceita (mínimo 1), e com par v2 a origem usa `EnterAt` e o cursor virtual. Deve funcionar, mas é um caminho menos testado.

### Hipóteses, da mais provável para a menos

1. **Nenhuma ligação no `layout.toml` do PC.** Por desenho (§13.2 regra 2a), sem ligação desta máquina o PC não instala hook nenhum e nada cruza. O app não avisa isso em lugar nenhum além da tela Layout e do item "atalhos" do Diagnóstico. Quem vem do Input Leap espera que parear já baste.
2. **Ligação feita no lugar errado** (ex.: na tela Layout do notebook, que não captura nada, ou num trecho de borda que o usuário não empurra).
3. **O PC sem os monitores do notebook** na tela Layout: as `Zones` do notebook só chegam a par com `input_in`; com o notebook na 0.2.1 isso deveria funcionar, mas vale conferir se o `eDP-1` aparece no PC.
4. **Hooks do Windows não sobem ou caem** (nunca testado com uma pessoa usando): ver o Diagnóstico do PC ("atalhos" e bloqueios do input) e o bloqueio "a captura do Windows parou: os hooks do sistema caíram 4 vezes em 10 s".
5. **Mistura de versões** (v3 com v2) num ponto não coberto.

### Plano de diagnóstico

1. **No PC** (Windows, PowerShell), com a CLI do mesmo release (`clipsync-v0.3.0-windows-x86_64.zip`):
   - `.\clipsync.exe --version`, `.\clipsync.exe status` e `.\clipsync.exe diagnose`;
   - `Get-Content $env:LOCALAPPDATA\clipsync\layout.toml` (procure `[[link]]`);
   - no app, a tela Layout: aparecem os 2 monitores do PC e o `eDP-1` do notebook? Há alguma ligação?
   Peça ao usuário para colar as saídas; você não tem acesso ao PC.
2. **Atualizar o notebook para a v0.3.0**: `sudo apt install ./clipsync-app-v0.3.0-linux-amd64.deb` (o usuário roda; não há sudo para você). Os dois lados na mesma versão tiram a hipótese 5.
3. **Criar a ligação no PC**: tela Layout, clicar na borda do monitor que leva ao notebook (ex.: inferior do monitor 2), escolher o `eDP-1` e a borda de entrada, Ligar. É o passo "Antes" do roteiro §13.12.
4. Repetir a travessia e conferir no notebook `clipsync status` ("último evento" do input deve mudar) e, se o portal pedir, o diálogo de permissão do RemoteDesktop aparece no notebook na primeira vez.
5. Se ainda não cruzar: Diagnóstico do PC (item "atalhos": "ativos" só com hook), bloqueios recentes do input, e o log do app no PC.

## 7. Melhorias de usabilidade propostas

O usuário pediu "melhore a usabilidade". Ordem sugerida, da que mais resolve a confusão atual para a menor:

1. **Dizer por que o mouse não cruza.** Um estado de "travessia" visível na tela Dispositivos, na tela Layout e na bandeja, com a primeira coisa que falta, em português: módulo input desligado aqui ou no par; par sem `input_in`; **nenhuma ligação desta máquina no layout**; par em versão antiga; hooks inativos ou caídos; portal sem permissão; tela bloqueada; pausado. O dado já existe quase todo (`InputState`, `features`, bloqueios); falta juntar e mostrar. Precisa de comando ou campo no `clipsync-ui` e de texto na interface.
2. **Ligação sugerida ao parear.** Quando um par recebe `input_in`, oferecer na hora uma ligação padrão ("o notebook fica abaixo do monitor 2", "à direita do PC") com um clique, gravando o `[[link]]` no `layout.toml` da origem. É o que o Input Leap faz por padrão e o que o usuário esperava.
3. **Estado vazio da tela Layout** com chamada clara ("Nenhuma borda ligada: o mouse não sai desta tela. Clique numa borda para ligar") e as bordas ligáveis destacadas.
4. **Aviso de versão diferente** entre pares (ex.: "o notebook está na 0.2.1; atualize para a 0.3.0") em Dispositivos.
5. **"Testar travessia"** na tela Layout, usando o `input_test` que já existe, com o resultado explicado.
6. **CLI**: `diagnose_for(profile_dir, self_id)` para o `clipsync diagnose` aplicar a mesma regra dos atalhos que a interface aplica (hoje mostra um aviso informativo).

Cada item mexe em contrato (§13 ou uma §14 nova): registre antes de codificar, com critério de pronto testável.

## 8. Pendências gerais

- Roteiro manual da S7 (§13.12, 9 passos) e as 5 perguntas da §13.14 com o usuário.
- Licença: hoje "Todos os direitos reservados" (`clipsync_ui::prefs::LICENSE` e README); o usuário ainda não escolheu (recomendação dada: MIT).
- Juntar as três cópias de `write_private` (core, `clipsync-ui` em `autostart/fsio.rs` e `clipsync-input` em `layout.rs`) num lugar só (informativo do Sentinel).
- Teste dos instaladores numa máquina Windows de verdade.
- Terceiro PC: sistema desconhecido; KDE e X11 ficaram fora da S7.

## 9. Como o time trabalhava

O Maestri rodava um time com posse por arquivo: Keel (contrato e workboard), Rivet (proto, core e `clipsync-ui`), Flux (clipboard, CLI, casca Tauri e `clipsync-win`), Vector (input), Pixel (frontend), Sentinel (segurança), Argus (vereditos, era o Codex) e Maestro (CI, release, lockfile, instalação). O ciclo de cada fatia:

1. contrato com critérios de pronto testáveis;
2. código com testes, `clippy -D warnings` e checagem do Windows;
3. revisão de segurança com achados por severidade;
4. veredito por critério, com evidência;
5. CI verde nos dois sistemas na base da tag;
6. release, `sha256sum -c` e instalação.

Sozinho, siga o mesmo ciclo e mantenha `docs/contrato.md`, `docs/workboard.md`, `docs/seguranca.md` e `docs/verdicts.md` em dia: são a memória do projeto.
