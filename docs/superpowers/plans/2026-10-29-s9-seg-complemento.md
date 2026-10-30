# S9 · complemento SEG B01/B03/B05/B07

**Objetivo:** executar as quatro correções já autorizadas, preservando as entregas revisadas.
**Contrato:** §§15.10–15.13; specs 2026-10-29-s9-seg-complemento.md e 2026-10-29-s9-seg-execucao.md.

## Global Constraints
- Um editor por arquivo; handoff explícito do Maestro. B07 somente após revisão do schema e FIX1 H01/B02. B01 preserva B09 e aguarda liberação de JS congelado.
- Sem wire/dependências/grants/capabilities novos, unsafe fora ffi, perfil real, segredos, consentimentos, runtime por builder, commit/push/tag/release.
- Não criar target em /tmp. Execução Windows/Tauri indisponível não é substituída por compilação; nenhum teste de send falso pode executar input real.
- Candidatos/contagens/fakes/loopback provam somente seu recorte; manter provas físicas pendentes.

## Review Focus
- Relógio não emite MAX nem avança por entrada inadmissível; last nunca retrocede.
- Falha parcial conserva pendências e bloqueia emit/enter até soltura confirmada.
- Cadência global sobrevive resync; memória e diversidade limitadas; JSON antigo tem defaults.
- Lifecycle observa snapshot imediato e geração antes de liberar dados; Stop/pausa prevalecem.

### Task 1: B03: relógio de clipboard

**Brief:** fechar overflow/envenenamento lógico mantendo ordenação e ações existentes. Um dono backend-clipboard; nenhuma alteração proto.

**Posse exata:** modificar `crates/clipsync-clipboard/src/module.rs`, `crates/clipsync-clipboard/src/msg.rs`, `crates/clipsync-clipboard/tests/module_mem.rs`. Testes unitários que precisam semear MAX ficam em `module.rs`; sem novo helper/crate. Entradas H01/B02 de ModuleEvent com `..` permanecem.

**Interface:** `const MAX_REMOTE_AHEAD: u64 = 1_000_000`; `fn new_offer(&mut self, text: String) -> Result<ClipOffer, &'static str>`. `msg::CLOCK_EXHAUSTED` e `msg::CLOCK_AHEAD` são textos fixos; CLOCK_AHEAD explica a janela lógica, sem conteúdo. ClipSend responde ModuleReply::Error; captura local registra last_block e retorna ao laço. ClipSendCurrent já delega a clip_send, preservá-lo.

- [ ] RED dirigido: `clock_admission_preserves_state` cobre 0/MAX/janela+1, limite exato e pausa/expiração/duplicação extrema; capturar current/last/lamport_max/caixa antes/depois. Não alterar semântica de TTL fora deste achado (o código atual usa ttl_ms==0 e expiração existente).
- [ ] RED dirigido: `local_clock_exhaustion_keeps_module_alive` semeia MAX-1 e MAX, tenta cópia/ClipSend/ClipSendCurrent, prova zero ofertas/MAX/0, erro/diagnóstico fixo e resposta posterior de estado/pausa. `equal_timestamp_tiebreak_and_last` usa dois PeerId, entrega invertida, Written/falha/conflito/Manual/ClipApply stale.
- [ ] Implementar checked_add com reserva de MAX; mover incorporação remota após pré-validação; usar maior par após escrita/aplicação e tratar Result em todos os chamadores. Não aumentar last em envio explícito que não copia localmente.
- [ ] GREEN/revisão: `cargo test --locked -p clipsync-clipboard`; `cargo clippy --locked -p clipsync-clipboard --all-targets -- -D warnings`. Registrar testes/ignorados e limites fake; entregar diff ao Sentinel/warden. Não executar profile/UI por arrasto.

### Task 2: B05: bookkeeping do WinEmulate real

**Brief:** tornar falha parcial conservadora e impedir nova injeção enquanto soltura não foi confirmada. Um dono backend-emulação.

**Posse exata:** somente `crates/clipsync-input/src/adapter/win/emulate.rs`, inclusive testes cfg(windows) no próprio arquivo. Não editar win/ffi.rs, adapter/mod.rs, win/capture.rs ou H01/B02. Helper de conjuntos privado fica neste arquivo, evitando posse cruzada.

**Interface:** adicionar `cleanup_pending: bool`; transformar `send(batch)` estático em seam privada por instância `send(&mut self, batch: &[Synth]) -> Result<(), InputError>` e todos os callsites passam por ela. A implementação de produção continua chamando clipsync_win::send. Test backend privado cfg(test) também fornece desktop/monitores/cursor/virtual_rect usados pelo método real; evita chamada física acidental antes/depois do send falso. Não mudar trait público nem FFI; sem logs/Debug de conjuntos.

- [ ] RED cfg(windows): chamar `EmulateAdapter::emit` real com down anterior+up falho, down+up, dois códigos e erro parcial; conferir possible/cleanup_pending. Chamar `release_all` duas vezes com primeiro Err/segundo Ok, verificar mesmos ups e limpeza só no segundo.
- [ ] RED cfg(windows): `emit_waits_for_cleanup` e `enter_waits_for_cleanup` chamam os dois métodos reais; send falso da limpeza falha => nenhuma chamada nova de down/move absoluto; reparação da limpeza => nova operação ocorre uma vez. Cobrir sucesso integral e restos zerados após erro. Não substituir estes casos por teste do helper.
- [ ] Implementar desired/possible, commit conservador e limpeza antes de emit/enter; conservar todos os códigos no erro, inclusive up do mesmo lote; release não drena e stop devolve erro. Nenhuma repetição automática de eventos do lote falho.
- [ ] GREEN Windows: `cargo test --locked -p clipsync-input adapter::win::emulate::tests` em runner Windows; localmente cross-clippy com target já usado no report (`CC_x86_64_pc_windows_gnu=true AR_x86_64_pc_windows_gnu=true cargo clippy --locked --target x86_64-pc-windows-gnu -p clipsync-input --all-targets -- -D warnings`). Ausência de runner deixa execução desses testes PENDENTE, mesmo que cross-clippy passe. Review por Sentinel/warden; fake não prova SendInput/UIPI/soltura física.

### Task 3: B01: histórico agrupado até Diagnóstico e casca

**Brief:** implementar o schema acima de ponta a ponta e verificar candidatos/chamadas fake. Um único dono B01 após liberação dos atuais donos.

**Posse exata:** `crates/clipsync-core/src/transport.rs`; `crates/clipsync-proto/src/control.rs`; `crates/clipsync-ui/src/{events,diag,msg}.rs`; `crates/clipsync-ui/src/traversal.rs` SOMENTE literal de BlockRecord no teste existente; `crates/clipsync-ui/tests/{events,diag,fixtures}.rs`; `crates/clipsync-app/src/{events,state}.rs`; `crates/clipsync-app/ui/js/telas/diagnostico.js`. Testes unitários adicionais ficam nos próprios arquivos; JSON antigo das fixtures permanece como entrada de compatibilidade. Nenhuma edição em app/tray/msg, ui/lib/status ou mock necessários. Se fixture de saída exata exigir atualização, preservar um caso antigo sem campos em fixtures.rs em vez de substituir toda evidência por JSON novo.

**Interfaces:** schema B01 integral acima; LRU em VecDeque já existente; estado de agregação único no poll e gate único em AppState. Não criar notificador por PeerBlocked.

- [ ] RED backend: 1.000 negativas => um grupo/count1000; A/B/repetir A conserva B e move A ao início; causa21 expulsa LRU; nomes diferentes mesma chave; relógio de parede regressivo não muda LRU; count MAX satura. JSON antigo => count1/último normalizado.
- [ ] RED UI/casca: snapshot idêntico zero delta; aumento só no grupo correto; reordenação/nome sem aviso; count reiniciado/início novo sem underflow; 21 causas/pares marcam diversidade pelo-menos e memória≤20. Fake Instant testa primeiro aviso, 29.999 s, 30 s, resumo em poll sem novos eventos, resync, ausência/reaparecimento de agente, block off/on sem replay/reset, e device_removed independente. Casca testa candidatos duplicados e erro do notificante dentro da mesma janela com no máximo uma chamada fake. Nenhuma contagem é chamada de total absoluto da rede.
- [ ] Implementar backend/schema primeiro, depois UI/diag, então casca e tela, respeitando gates de posse; ajustar literal unitário em traversal.rs. Render/export mostra contagem/última ocorrência e defaults; preservar redação de dados no suporte.
- [ ] GREEN: `cargo test --locked -p clipsync-proto`; `cargo test --locked -p clipsync-core --lib`; `cargo test --locked -p clipsync-ui --test events --test diag --test fixtures`; testes unitários dirigidos de events/diag; `cargo test --locked -p clipsync-app` onde toolchain Tauri local suportar. Clippy dos crates modificados. Se Tauri não compilar por infraestrutura, registrar limite sem declarar gate testado. Maestro valida Diagnóstico via portal depois da integração; notificações físicas Windows/Linux permanecem manuais. Review Sentinel/warden antes do handoff.

### Task 4: B07: mailbox Windows, watch e consumidores B02

**Brief:** remover perda de lifecycle na origem/contrapressão mantendo snapshot inicial, prioridades e cancelamento atuais. Um dono backend-lifecycle, após revisão/liberação H01/B02 INCLUINDO FIX1/I1+I2; não partir os consumidores entre dois implementadores. Ao iniciar, reler os arquivos finais e a revisão de FIX1 para localizar a fronteira correta; as linhas do snapshot são apenas referências históricas.

**Posse exata:** `crates/clipsync-input/src/adapter/{mod.rs,lifecycle.rs(new),fake.rs,win/capture.rs}`; `crates/clipsync-input/src/{capture,module}.rs`; novo `crates/clipsync-input/tests/lifecycle_watch.rs`. Fakes/harness do teste novo ficam nesse arquivo, sem reescrever support/revocation atuais. Tests cfgWindows adicionais no próprio win/capture.rs. `session.rs` e `portal.rs` são fontes de regressão, somente leitura; `clipsync-win`, proto e rede não mudam.

**Interfaces:** CaptureLifecycle/receiver/schema e protocolo de aquisição acima. FakeCapture deve poder escolher `None` (default preserva testes atuais) ou watch pela mesma instância e publicar snapshots/gerações sem depender do rx de dados; controles de teste não viram API wire. Registrar helper lifecycle em adapter/mod.rs e usar pelos dois consumidores.

- [ ] RED portável: encher dados, aplicar lock+unlock antes de leitura, consumir snapshot final unlocked => sessão/abertura antiga cancelada, batch/rx descartados e nenhuma Activated/Input antiga transmitida. Reaquisição de receiver com geração maior conserva invalidação; estado inicial false válido permite só captura nova; snapshot inválido/None sem evento inicial permanece fechado. Testes distinguem `changed` de leitura imediata.
- [ ] RED Windows/mailbox: fila8000/saída4096 cheias, cada Notice e overflow conservam fatos; callback não espera fila; worker publica mesmo com out cheio. Display/hooks não somem no drain, mudança durante consulta inicial não é sobrescrita; Stop+HooksLost não reinstala; generation/revision não resetam no stop/start; held antigo não reaparece. Mailbox constante, watch mantém um snapshot, sem vetor/fila de notices.
- [ ] RED consumidores/fallback: continuous E loopback, abertura simultânea a lifecycle, pausa/trava/Stop simultâneos a unlock, start pausado zero hooks, retomada após unlock durante pausa lê snapshot fresco; None executa snapshots explícitos Linux e semântica WTS/desktop do backend atual. Cobrir SessionLocked(true) seguido de produtor sem barreiras para desbloqueio, sem permitir exceção durante pausa/manual lock. Tests existentes de WTS known/unknown+desktop continuam compilando. Preservar e reexecutar regressões FIX1: produtor realmente bloqueado em send precisa concluir join, e stop controlado por gate com Stop/Paused/Lock/host.paused/cancelamento da bridge durante a espera não pode aumentar starts.
- [ ] Implementar mailbox/watch no adaptador, depois reconciliação compartilhada e ambos consumidores; revisar pontos de reset_rx, drain e finalização. Snapshot inicial e startup são parte do mesmo handoff; não integrar só metade do canal. Se revisão H01/B02 já tiver introduzido canal equivalente, adaptar este schema ao canal existente e registrar equivalência no contrato antes do código, sem duplicar.
- [ ] GREEN: `cargo test --locked -p clipsync-input`; clippy input/win em host e cross target com flags do report; `scripts/conferir-unsafe.sh`, `scripts/conferir-privacidade.sh`. Executar testes cfgWindows em CI Windows quando disponível; cross-clippy é só compilação. Reexecutar os cenários H01/B02 afetados do baseline, sem baixar asserções/limites. Sentinel/warden revisa especialmente ordem de snapshot/geração/cleanup e liberação de Stop; portal/desktop/hook físico fica manual.
