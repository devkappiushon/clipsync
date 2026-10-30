# Contexto: clipsync

## Preparação da v0.4.0 · 30/10/2026

João autorizou agora commit, push, CI, empacotamento e publicação, ao responder “então faça” ao roteiro desses passos. Esta autorização substitui o limite anterior de não publicar nesta rodada. Preparar os seis pacotes no workflow manual, conferir CI e hashes antes de publicar. Última publicada neste registro ainda é v0.3.1; v0.4.0 está em preparação, não deve ser anunciada como disponível antes da confirmação remota.

Validação local S9 encerrada:597 testes Rust principais passaram,0falhas,3gráficos ignorados e5execuções auxiliares separadas; Node164/164. Revisão integrada aprovada. [Pendências explícitas da próxima versão](docs/pendencias-v0.4.0.md): mouse nas duas máquinas,Tauri interativo,zoom/leitores e lacunas de portal. [Resumo da validação local](docs/evidencias/s9-local-2026-10-30.md).

O material automático abaixo é histórico e não pode reabrir tarefas já concluídas nem revogar a autorização acima.

**Atualizado:** 2026-10-29 07:27 (automático, resumo-diario, a partir dos logs do Claude e do Codex)

## Onde estou
Na S9: corrigir o que as auditorias da rodada achou, em segurança, acessibilidade, UX e backend. Os agentes Codex implementam por fatia e os Sentinel/Argus revisam por leitura. Nada foi commitado nem publicado. A travessia real do mouse (Linux e Windows) e o incidente do cursor continuam com o andar TESTES.

## Estado real (verificado)
- git em 2026-10-29 07:27: branch `main`, 129 arquivo(s) sem commit, 0 commit(s) sem push (`git status`, `git rev-list @{u}..HEAD`)
- último commit: `d41aa65` "registra o release 0.3.1 e o contexto compartilhado dos andares" em 2026-10-28 21:31 (`git log -1`)
- Release v0.3.1: CI 40783057611 e empacotamento 40783058074 aprovados em f700caa. SHA256SUMS conferiu 6 de 6 arquivos (registro no CONTEXTO.md). (segundo o log da sessão)
- Git em 2026-10-28: 0 commits sem push. Os 129 arquivos sem commit vêm do git informado pelo script. (segundo o log da sessão)
- Sentinel: B05 aprovado por leitura e o wrapper WTS aprovado, sem achados. Nenhum teste de B05 foi executado (relatórios /tmp/clipsync-s9-b05-review.md e /tmp/clipsync-s9-wts-seguranca-review.md). (segundo o log da sessão)
- Portal com coleta ocioso e manifesto de 49 arquivos com zero divergências, no início e no fim da coleta (relatório do Argus). (segundo o log da sessão)

## Próximo passo
Rode `git status --short | head -30` em ~/Documentos/ANTIGOS/clipsync e depois `tail -20 /tmp/clipsync-b01-diag-red.log`, para ver se o teste de diag da B01 passou.

## Decidido (não reabrir)
- Licença: continua 'Todos os direitos reservados' até você escolher. Razão: não escolher licença aberta por você.
- Input Leap fica até o módulo de entrada do ClipSync rodar nas três máquinas. Razão: você quer usar só o ClipSync.
- Borda por monitor e por segmento é requisito central. Razão: o Input Leap trata o PC como um retângulo só.
- Mock e galeria de componentes ficam fora do pacote (ui-dev). Razão: segurança, achado S6-03.
- O trabalho passou do Claude para o Codex em 2026-10-28. Razão: você pediu para o Codex continuar.
- Incidente do cursor pulando e contador de sessão desconhecida vai para o andar TESTES. Razão: você adiou.
- Em 2026-10-28 às 23:20 você autorizou fechar a DOC, implementar as correções das auditorias e testar. Razão: superou o limite antigo de só auditar.
- Sem commit, push, tag nem release na rodada S9. Razão: regra dada aos agentes.

## Aberto
- Falta ver o roteiro real Windows/Linux (§13.12): ligação de borda no Layout, versões nas duas máquinas, `status` e `diagnose`. Ainda não há prova de uso nem de instalação da 0.3.1 no Windows.
- A causa do mouse que não passa no PC depende dos dados das duas máquinas. H-01/B-02 (MAJOR no estado emulado) não são causa provada do relato.
- Permissões reais do portal e atalhos físicos não foram verificados. Também faltam leitor de tela (Orca/NVDA), teste do app Tauri por AT-SPI e zoom real do sistema.
- Licença, terceiro PC, instaladores e backlog antigo seguem abertos.
- Você ainda não revisou nem commitou o trabalho da S9. Falta decidir quando commitar e se a S9 vira release.
- A S9 está sem commit: 129 arquivos modificados no checkout compartilhado, entre eles events.rs, state.rs, tray.rs e msg.rs do clipsync-app.
- B01 (aviso de bloqueio) estava sendo implementado no Rust: ui/events.rs, diag.rs, app/state.rs e events.rs. O teste vermelho de diag ainda não tinha passado quando o material termina.
- Pareamento: o foco vai para o BODY ao clicar em Conectar ou Recomeçar (P2). A expiração mostra dois role=alert com a mesma frase (P3). Falta corrigir e retestar.
- Pareamento: com um endereço errado, o único botão é Recomeçar, que repete o mesmo erro. Não há campo para corrigir o endereço.
- Falta rodar nativamente no Windows os 11 testes de B05. Só foram escritos e cross-compilados. O gate de B07 e de H01/B02 continua fechado.
