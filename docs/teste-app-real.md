# Teste do app real · S6 (critério 6 da §12.9)

Data: 2026-10-28. Base: pacote do empacotamento manual 40680774165 (commit e40849c), versão 0.2.0.
Quem rodou: Maestro, nesta máquina (Ubuntu 26.04, GNOME Wayland com XWayland).
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

## Falta, depende da tela e do usuário

- Onboarding criando o perfil pela interface (o perfil de teste foi criado pela CLI).
- Pareamento iniciado pela tela do app (o código igual foi provado CLI com CLI sobre o perfil do app).
- Clicar em Aplicar num item da caixa e ver o texto no clipboard real.
- Diagnóstico listando os bloqueios e preferências sobrevivendo a fechar e abrir, vistos na tela.
- O teste manual do critério 3: `cargo test -p clipsync-input --test portal_loopback portal_loopback_from_layout -- --ignored --nocapture`, que pede o diálogo do portal.

A lógica desses itens está coberta pelos testes do `clipsync-ui` (critério 2, Argus 48aedea) e a tela pelo critério 5 (Argus 4f573f5).
