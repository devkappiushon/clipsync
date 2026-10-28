# Teste do app real · S6 (critério 6 da §12.9)

Data: 2026-10-28. Base: pacote do empacotamento manual 40680774165 (commit e40849c), versão 0.2.0.
Quem rodou: Maestro, nesta máquina (Ubuntu 26.04, GNOME Wayland com XWayland). Parte de linha de comando com o pacote 40680774165; parte de tela com o AppImage do release v0.2.0.
App: `~/.local/bin/clipsync-app.AppImage`, instalado por `scripts/instalar-app-linux.sh`.
Perfis de teste em `~/.cache/csrt/` (o perfil real do usuário não foi tocado):
- `t1`: perfil aberto pelo app, clipboard ligado com adaptador real, modo manual.
- `t2`: CLI 0.2.0 do mesmo pacote, adaptadores falsos.

## Verificado por execução

| Item do critério 6 | Como | Resultado |
|---|---|---|
| O AppImage do CI roda nesta máquina | app aberto com `--profile ~/.cache/csrt/t1` | sobe sem erro no log; agente dentro do app ativo, `escuta 0.0.0.0:47812`, adaptador `x11-xfixes` |
| Pareamento mostra o mesmo código nos dois lados | `clipsync pair host` (t1) e `clipsync pair join` (t2), confirmando nos dois | `Código: 986 503` nos dois lados; permissões gravadas |
| `clipsync send --stdin` chega à caixa do app em modo manual | `echo ... \| clipsync --profile t2 send --stdin`; `clipsync --profile t1 inbox` | item `de cli-teste · na caixa de recebidos`; clipboard real intacto |
| Uma cópia no sistema chega ao `clipsync inbox` do outro perfil | `wl-copy` no sistema; `clipsync --profile t2 inbox` | item `de app-teste · aplicado` em 4 s; clipboard original restaurado depois |
| Ícone na bandeja com o menu | D-Bus `StatusNotifierWatcher` e `dbusmenu` do processo do app | item ativo do processo do app com Abrir, Pausar tudo e Sair |
| Pausar e retomar pela bandeja | evento `clicked` no item do menu | status do app `PAUSADO`, rótulo vira Retomar; de novo, volta a `ativo` |
| Sair encerra e o par percebe | `clicked` em Sair | app encerra em 3 s; t2 passa o par para `conectando` |
| Ao abrir de novo, o agente sobe e reconecta | app reaberto com o mesmo perfil | `conectado` ao t2 em 15 s, sem erro no log |
| "ClipSync" na barra de aplicações | `~/.local/share/applications/clipsync.desktop` | `desktop-file-validate` sem erro; ícone de 256 px extraído do AppImage |

## Verificado pela tela do app real (acessibilidade AT-SPI)

Mesmo AppImage, agora o do release v0.2.0 (SHA-256 igual ao SHA256SUMS), aberto com o perfil vazio `~/.cache/csrt/ui1`. A janela do WebKitGTK foi lida e acionada pelo barramento de acessibilidade (AT-SPI), como um leitor de tela: botões, interruptores e textos da própria interface. Nada de navegador automatizado.

| Item | Como, pela tela | Resultado |
|---|---|---|
| Onboarding cria o perfil | passo 1 mostra o nome da máquina e a impressão digital `6425-6da5-90b5-1d27`; passo 2 desliga teclado e mouse e liga a área de transferência; Criar perfil e continuar | perfil gravado (identidade, `config.toml` com clipboard ligado e input desligado); o agente sobe com o id `6425-6da5-90b5-1d27`, o mesmo mostrado |
| Autoteste do passo 3 | Rodar teste com conteúdo sintético | Área de transferência validado em 2 ms, conteúdo anterior devolvido; Teclado e mouse não testado (desligado) |
| Pareamento pela tela | Parear novo dispositivo, Aceitar pareamento; CLI entra pelo endereço mostrado | o mesmo código nos dois lados (`954 136`); São iguais; permissões de clipboard ligadas no painel; Concluir; os dois lados pareados |
| Recusa do outro lado | a CLI respondeu não | a tela mostra na hora que a outra máquina recusou (correção 13255e3) |
| Modo manual pela tela | Clipboard, Manual | `config.toml` passa a `mode = "manual"` |
| Aplicar | item da CLI na caixa; Mostrar exibe o texto; Copiar | o texto enviado está no clipboard real do sistema (`wl-paste`) |
| Diagnóstico lista os bloqueios | Dispositivos, desliga Pode me enviar clipboard; a CLI envia de novo | `peers.toml` com `clipboard_in = false`; a CLI recebe recusado: sem permissão; Diagnóstico, Últimos bloqueios mostra hora, clipboard e sem permissão |
| Preferências sobrevivem a fechar e abrir | Preferências, tema Escuro; Sair pela bandeja; abrir de novo | `ui.toml` com `theme = "dark"`; depois de reabrir, Escuro continua marcado e o modo manual continua valendo; o agente reconecta |

Defeito visto: depois de uma recusa, a tela de pareamento repete a mensagem ("A outra máquina recusou o pareamento. o outro dispositivo recusou o pareamento"). Enviado ao Pixel.
Incidente: o clipboard do usuário tinha 8 bytes antes dos testes e ficou com um texto de teste; os 8 bytes não foram recuperados. O clipboard foi limpo no fim.

## Falta, depende do usuário

- O teste manual do critério 3: `cargo test -p clipsync-input --test portal_loopback portal_loopback_from_layout -- --ignored --nocapture`. Ele pede o diálogo do portal de captura de entrada, que é uma permissão de segurança do sistema: só o usuário aprova.
