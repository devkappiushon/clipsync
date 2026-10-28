# Plano S8 · diagnóstico e configuração da travessia

> Para os implementadores: executar por fatia com revisão independente, usando `superpowers:executing-plans` ou o fluxo de delegação já determinado pelo Maestro. O pedido do usuário já autoriza realizar as pendências; não há novo gate de design. Sem commits nesta rodada.

**Objetivo:** explicar o primeiro requisito local ausente, orientar a ligação no computador correto e tornar o teste existente compreensível e seguro.

**Arquitetura:** `app_state.traversal` agrega configuração e estado já disponível, sem inventar telemetria. Frontend e bandeja compartilham a interpretação; `diagnose_for` evita divergência entre CLI e UI. A configuração reutiliza `layout_set_link`, com gravação explícita e controle de revisão.

**Stack:** Rust 1.99.1, Tauri 2, HTML/CSS/JS sem framework. Sem nova dependência.

**Especificação:** `docs/contrato.md` §14 (v1.34); contexto em `docs/handoff-codex.md` §6/§7.

## Restrições globais

- `PROTOCOL_VERSION = 3`, `MIN_PROTOCOL_VERSION = 1`; sem mudar postcard ou permissões.
- `input_in` local concede entrada; não representa permissão remota de saída.
- Ausência de layout local não impede receber controle de outra máquina.
- Nenhum hook, teste de entrada, gravação de layout ou diálogo do sistema iniciado por inspeção.
- Só portais Maestri para inspeção visual; Playwright proibido.
- Preservar alterações prévias; posse de arquivos na §14.1. Sem commit/push/tag.

## Foco de revisão

- Ligação só entre remotos ou para par revogado não conta como saída local.
- Protocolo desconhecido não vira release inventado nem resultado positivo.
- Bloqueio antigo não vira falha atual; falta de telemetria não vira “funcionando”.
- Sugestão após conceder controle não confunde receptor com computador do mouse.
- Bloquear a origem durante teste interrompe envio e solta teclas no destino.

## Tarefa 1 · estado e diagnóstico com contexto (backend)

Arquivos: `clipsync-ui/src/{traversal,status,lib,diag,events}.rs`, testes/fixtures correspondentes; `clipsync-input/src/{diag,lib}.rs`; `clipsync-cli/src/main.rs` e testes; `clipsync-app/src/{tray,events,msg}.rs` e testes.

Interface produzida: `AppState.traversal = {code,detail,local_links,last_block,peers:[{peer_id,code,detail,protocol_warning}]}`; enum e ordem da §14.2; evento local `traversal_changed {traversal}` quando o resumo muda. `diagnose_for(&Path, Option<PeerId>, Option<&AgentStatus>) -> Vec<DiagItem>` assíncrono, preservando `diagnose()`.

- [x] Escrever testes dos estados e das três primeiras classes do foco de revisão; JSON antigo sem traversal desserializa como checking.
- [x] Rodar testes e observar falha antes da implementação.
- [x] Implementar agregação e helper, sanitização e atualização da bandeja pela mesma informação.
- [x] Rodar `cargo test --locked -p clipsync-ui -p clipsync-input -p clipsync-cli` e clippy dos crates; relatar limite da casca Tauri local.

## Tarefa 2 · configuração guiada e explicações (frontend)

Arquivos: `crates/clipsync-app/ui/**` e `ui-dev/**`. Consome a interface da tarefa 1; usa comandos existentes.

- [x] Mostrar estado em Dispositivos/Layout, protocolo corretamente rotulado e aviso local de ausência de ligação.
- [x] Acrescentar CTA depois do pareamento/permissão e formulário de sugestão por posição relativa (esquerda/direita/acima/abaixo), derivação automática em bordas externas de monitores reais alinhados ao destino, excluindo trechos internos/ocupados e cantos mortos mesmo após conversão para f32, prévia e gravação somente em Ligar nesta posição. Escolha manual de bordas fica em Avançado.
- [x] Explicar `input_test`, exigir clique, mostrar andamento e resultado inconclusivo para zero eventos; não atestar retorno S7 nem latência entre máquinas.
- [x] Criar cenários do simulador para vazios, receptor, protocolo antigo, conflito de revisão e teste sem eventos; verificar invariantes por busca e entregar roteiro ao Maestro para portal.

## Tarefa 3 · encerramento do teste ao bloquear (input)

Arquivos: `crates/clipsync-input/src/module.rs` e teste dedicado de regressão. Posse só após liberação do Maestro, sem editar diag/lib.

- [x] Criar regressão com FakeCapture/FakeEmulate/MemHub: `SessionLocked(true)` antes/depois da ativação termina o teste; eventos seguintes não chegam e pressionados voltam a zero.
- [x] Rodar teste e observar falha na implementação antiga.
- [x] Tratar bloqueio no drive, usando a liberação existente e enviando Leave/ReleaseAll da sessão; não registrar teclas.
- [x] Rodar regressão e suíte de input, fmt, clippy e script de privacidade.

## Tarefa 4 · revisão e validação integrada

- [x] Warden revisa critérios, segurança revisa diagnóstico/configuração e encerramento do teste; registrar seções novas em verdicts/seguranca.
- [x] Maestro roda fmt, clippy e testes com `--workspace --exclude clipsync-app --locked` (incluindo clipsync-ui, fora dos default-members), scripts unsafe/privacidade e checagem cruzada Windows permitida.
- [x] Maestro sobe simulador em background com log, prova HTTP com curl, confere interface pelo portal e encerra o processo no fim.
- [x] Atualizar workboard com resultados reais. Windows físico, permissões do portal, Tauri sem headers e CI não executado ficam explicitamente não verificados.


## Fechamento da execução · 2026-10-28

Implementação e revisão local concluídas; prova física permanece pendente. Integração: 441 testes Rust passaram, 0 falhas e 3 ignorados; fmt, clippy local e cross Windows dos crates input/win/ui/cli passaram. Frontend: 22 testes JS e sintaxe de 33 arquivos; portal validou fluxo simples, id preservado, alinhamento LG, casos vazios e falhas parciais antes/depois de gravar. Warden fechou S8-W1/W2/W3; segurança registrou revisão independente. Evidência e limitações em `docs/verdicts.md` e bloco S8 do workboard.

Os checkpoints RED das tarefas 1 e 3 são relato dos implementadores, sem log persistido conferido pelo warden; a execução GREEN foi repetida independentemente. Nenhum checkbox atesta Tauri, execução nativa Windows ou travessia entre as máquinas reais. O simulador foi encerrado pelo Maestro (PID 156631, porta 5174; curl posterior recusado). Sem commit, push ou release.
