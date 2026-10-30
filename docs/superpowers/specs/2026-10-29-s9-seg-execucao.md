# S9 · B01/B03/B05/B07 · contrato e plano de execução

> Registrado pelo Maestro para execução da rodada autorizada 2/3/4. Usar o fluxo de subagentes com superpowers-maestri. A posse é liberada por arquivo e tarefa; B07 ainda exige revisão do schema e o FIX1 H01/B02 aprovado. Sem novo commit/push/release.

**Objetivo:** corrigir as quatro causas confirmadas preservando H01/B02, C01/B09 e os limites da S9.

**Arquitetura:** agrupamento limitado de negativas com cadência monotônica em UI/casca; admissão explícita de relógio do clipboard; pendências conservadoras no emulador; snapshot de lifecycle separado dos dados Windows. APIs locais são aditivas; wire e FFI permanecem intactos.

**Stack:** Rust/Tokio/Serde; Tauri na casca; JS existente no Diagnóstico. Sem dependências novas.

**Fontes vinculantes:** `docs/superpowers/specs/2026-10-29-s9-seg-complemento.md`, inclusive TODOS os adendos Sentinel; `docs/contrato.md` §§15.1–15.9. Baseline de segurança inspecionado: `/tmp/clipsync-s9-backend-review-src`; relato de implementação: `/tmp/clipsync-s9-seguranca-report.md`; revisão independente: `/tmp/clipsync-s9-backend-review.md`. Não usar `d41aa65` como base de implementação destas quatro tarefas: ele é a base histórica da auditoria, anterior a H01/B02. Usar o checkout integrado depois da liberação dos arquivos e preservar os deltas revisados, inclusive FIX1 obrigatório descrito abaixo. O snapshot congelado é evidência de interfaces, não baseline aprovado.

## Restrições e sequência

- Nenhuma mudança de protocolo v3/enums postcard, grants, dependências, permissões Tauri, perfil real, instaladores, versão publicada ou política de consentimento. `unsafe` só no FFI existente, que estas tarefas não editam. H03/exclusão/retenção de perfil fica fora.
- Sem commit/push/tag/release nesta rodada, conforme §15.1. Sem runtime por builders; somente Maestro. Sem Playwright. Este trabalho de arquitetura não executou testes nem alterou repositório.
- Maestro registra §§15.10–15.13 e os schemas abaixo; depois confirma handoff por arquivo, não por diretório genérico. Revisão independente de segurança/warden por tarefa. Falhas da revisão H01/B02 corrigem primeiro o baseline, sem reaplicar patches sobre snapshot antigo.
- T1/B03 e T2/B05 podem executar em paralelo. T1 aguarda liberação de `clipboard/src/module.rs` pelo backend H01/B02. T2 não compartilha arquivo com H01/B02. T3/B01 depende da liberação de `core/src/transport.rs` e dos arquivos de Rivet; T4/B07 depende da liberação dos arquivos input H01/B02 e de seu snapshot WTS revisado.
- T3 tem UM implementador dono de todos os seus arquivos, mesmo atravessando crates; isso evita duas versões do schema. `ui/src/msg.rs` e `app/src/events.rs` só são entregues após Rivet terminar/revisar os três findings de C01+B09. `diagnostico.js` exige liberação do frontend-base. Sem edição simultânea nem substituição dos deltas B09.
- B01 e B07 podem executar em paralelo após os handoffs, pois suas posses são disjuntas. B05 não toca `adapter/mod.rs`; sua seam de teste fica em `win/emulate.rs`, evitando conflito com B07.
- **Gate FIX1 backend para B07:** a revisão encontrou I1 (stop/join espera produtor Portal bloqueado em send enquanto rx fica aberto) e I2 (rearme antes de observar Stop/Paused/Lock/host.paused recebidos durante await de stop). O dono H01/B02 corrige ambos primeiro e fornece revisão/evidência. B07 incorpora o caminho de parada cancelável/fechamento coordenado aprovado por FIX1, sem recolocar `stop().await` antes de resolver a contrapressão. Antes de CADA rearme após await, observar controles/cancelamento pendentes e autoridade host.paused atual. A exceção de observar unlock continua sujeita a isso. Não fazer essas correções paralelas em B07 nem declarar os dois pontos corretos com base no snapshot congelado.

## Cláusulas prontas para o contrato

### 15.10 B01 · grupos de negativas e cadência de avisos

`recent_blocks` conserva no máximo 20 grupos pela chave `(peer, module, reason)`; nome é apresentação. Chegada nova/repetida move o grupo ao início; causa nova expulsa o último, sem ordenar pelo relógio de parede. `at_unix_ms` é o primeiro instante, `occurrences: u64` tem default 1 e soma saturante, `last_unix_ms: u64` tem default 0 e leitura normalizada para `at_unix_ms` quando ausente. `u64::MAX` significa pelo menos. Controle JSON/UI/Diagnóstico são aditivos/default, sem alteração do fio QUIC. UI calcula diferenças por chave/início/contagem, aceita recriação sem subtração negativa e preserva baseline em polls sem agente. Uma ocorrência observada abre aviso; repetições acumulam para resumo após pelo menos 30 s monotônicos do aviso anterior, globalmente, sem reset por resync. Resumo diz ocorrências/causas/pares observados; diversidade guarda no máximo 20 chaves/20 pares e usa pelo-menos quando excedida. Casca aplica o mesmo teto a `kind=block`, respeitando preferências e sem afetar remoção real de dispositivo/outros kinds. Diagnóstico conserva história observada independentemente da supressão de notificações. Posse e aceites: T3 deste plano.

### 15.11 B03 · relógio lógico do clipboard

Manter `(lamport, PeerId)` e contador por instância, sem persistência nova. `MAX_REMOTE_AHEAD = 1_000_000` é restrição explícita de compatibilidade entre instâncias, não tempo/taxa: 0, MAX e salto acima da janela são `Invalid`. Conteúdo/tamanho/duplicação/pausa/TTL são validados antes de admitir relógio; oferta inadmissível não muda `current/last/lamport_max` nem entra na caixa por rejeição de relógio. Incremento local verificado nunca emite MAX: falha em MAX-1 e MAX, com erro explícito nas ações, diagnóstico fixo e laço vivo. Remoto admitido avança somente `lamport_max`; `last` representa maior par aplicado/copied localmente e só avança após escrita confirmada, nunca por InInbox/falha/conflito. ClipApply stale mantém permissão existente, sem fazer `last` retroceder. Todos os criadores tratam Result; sem reset automático ou nova variante wire. Posse e aceites: T1.

### 15.12 B05 · soltura conservadora no emulador Windows

Conjuntos representam códigos possivelmente pressionados. Para cada lote, calcular `desired` na ordem dos eventos e `possible = anterior ∪ downs tentados`; sucesso integral confirma desired, qualquer erro conserva possible e marca cleanup_pending. release_all manda ups da cópia e só limpa após sucesso integral; falha retém tudo. emit E enter reais tentam limpeza pendente antes de qualquer injeção nova e recusam a nova operação se ela falhar, sem replay de movimento/down. Em erro, zerar restos fracionários. Stop tenta soltura e devolve falha; não promete recuperação após destruição da instância nem soltura física. FFI/Result, desktop seguro e política H01/B02 ficam intactos. Aceites devem atravessar os métodos reais com send falso, além de testes de bookkeeping. Posse: T2.

### 15.13 B07 · lifecycle independente dos dados de captura

Notice Windows usa mailbox fixa de atomics/latches; lock/desktop/hooks invalidam e fecham flags locais imediatamente, sem I/O/consulta/espera no callback. Worker reconcilia antes dos dados, em timeout e durante contrapressão de saída; overflow descarta somente dados. `CaptureAdapter::lifecycle_receiver() -> Option<watch::Receiver<CaptureLifecycle>>`, default None, publica por instância o último estado e uma geração monotônica de invalidação. Schema interno abaixo, sem wire/FFI. Consumidor lê o valor atual imediatamente ao adquirir/reobter receiver, conserva geração já reconciliada e fecha gate até snapshot válido. Invalidação encerra sessão/abertura/batch e descarta produtor/rx antigos pelo B02, mesmo se o último estado já for desbloqueado. Unlock habilita somente ativação nova. Windows mantém WTS e desktop separados (§15.9); None conserva obrigatoriamente snapshots SessionLocked iniciais do caminho atual Linux/Windows. Stop/pausa/trava prevalecem; somente bloqueio do SO permite produtor novo sem barreiras para observar unlock. Stop não apaga snapshot/geração antes do consumo nem recria hooks. Posse e aceites: T4.

## Interfaces fixadas antes do código

### B01 · JSON, diferenças e estado de avisos

1. Em `crates/clipsync-proto/src/control.rs`, `BlockRecord` preserva todos os campos e acrescenta `#[serde(default = "one")] pub occurrences: u64` e `#[serde(default)] pub last_unix_ms: u64`; `fn one() -> u64 { 1 }` privado. Normalização em produtores/consumidores: `last_unix_ms == 0` usa `at_unix_ms`. Não comparar igualdade do registro inteiro para decidir novidade. Não depender de monotonicidade dos instantes de parede.
2. `clipsync_ui::events::PeerBlocked` e `clipsync_ui::diag::Block` recebem os mesmos dois campos/defaults. Notification `{kind,title,body}` NÃO muda. PeerBlocked transporta o total do grupo; sua emissão não dispara uma segunda notificação. Exportação textual de suporte inclui contagem/primeiro/último sem conteúdo de clipboard.
3. `BlockKey` privado em `ui/events.rs`: campos `peer: Option<PeerId>`, `module: Option<ModuleKind>`, `reason: String`; comparação por igualdade é suficiente, sem exigir Ord novo em ModuleKind. Delta: mesmo key+at e contagem maior => diferença; iguais => zero; contagem menor ou início diferente => total do grupo recriado; chave ausente => total observado. Atualização apenas de nome/último/reordenação não cria delta. Extremos saturados não inventam número adicional desconhecido. Um grupo novo com key+at+count exatamente igual ao antigo é indistinguível neste JSON; não prometer detectar recriação invisível.
4. `BlockNotifications` privado em `ui/events.rs`, criado UMA vez em `poll`, fora de Snapshot/diff: `last_sent: Option<tokio::time::Instant>`, `pending_occurrences: u64`, `pending_keys: Vec<BlockKey>`, `pending_peers: Vec<PeerId>`, `keys_overflow: bool`, `peers_overflow: bool`. Vecs têm limite 20; contagem satura. Sem par não conta como dispositivo. Métodos recebem `now: Instant` para relógio fake. `observe` acumula deltas; `flush` pode devolver no máximo `Option<Notification>` por tick. Se há pendência e nenhum aviso anterior ou decorreram 30 s, emitir um aviso e esvaziar só a pendência. Após o primeiro aviso, próximos polls acumulam até o prazo; poll vazio também libera resumo vencido. Sem pendência não emite. Preferência block desligada descarta pendências e continua atualizando baseline, mas NÃO zera last_sent; religar não reproduz história antiga nem contorna cadência. Resync/agent_state não recria agregador.
5. Casca real: `AppState` ganha `block_notice_at: Mutex<Option<std::time::Instant>>` e método `allow_block_notice(&self, now: std::time::Instant) -> bool`; atualizar timestamp quando admitir a tentativa de notificar, mesmo se `show()` falhar. Em `events::notify`, primeiro conferir `notify_prefs().allows(kind)`, depois aplicar gate somente para `kind == "block"`, antes de `.show()`. Estado fica no AppState, não em `spawn`, `forward`, handler de resync ou janela. As preferências observadas hoje vivem em `NotifyPrefs`; não inventar campo novo `os_notifications`. `device_removed` segue a política atual sem throttle de block. Testar gate+chamada fake, sem exigir AppHandle real.
6. `diagnostico.js` mostra repetição e último instante, usando fallback `occurrences ?? 1` / `last_unix_ms || at_unix_ms`; saturação é apresentada como pelo-menos, sem prometer precisão inteira além do Number do JS. Chave da linha inclui peer/module/reason/início, serializada como tupla, sem usar texto remoto como seletor. Manter acessibilidade/foco existentes.

### B07 · schema interno e reconciliação

Em `crates/clipsync-input/src/adapter/mod.rs` (tipos públicos do trait, sem Serialize):

```rust
#[derive(Debug, Clone)]
pub struct CaptureLifecycle {
    pub revision: u64,
    pub invalidation_generation: u64,
    pub valid: bool,
    pub session_locked: Option<bool>, // WTS; None desconhecido
    pub desktop_available: Option<bool>, // desktop separado, None desconhecido
    pub zones: Vec<Zone>,
    pub terminal_error: Option<&'static str>, // texto fixo, sem teclas/IDs/payload
}
// no trait CaptureAdapter, default para os adaptadores existentes:
fn lifecycle_receiver(&self) -> Option<tokio::sync::watch::Receiver<CaptureLifecycle>> {
    None
}
```

- Inicial: revision=0, invalidation_generation=0, valid=false, sessão/desktop=None, zones vazias, terminal_error=None. Gate só abre com valid=true, session_locked=Some(false), desktop_available=Some(true), terminal_error=None. `valid` significa snapshot reconciliado desta geração de produtor; pode ser válido E bloqueado. Zonas são só o último snapshot, sem histórico; respeitam os limites de zonas já usados pelo módulo/protocolo. Não transformar erro de consulta em `zones_now().unwrap_or_default()` silencioso na reconciliação.
- Um watch::Sender fica em WinCapture durante toda a instância, criado em new; stop/start substituem worker/rx de dados, não o canal. Clone é entregue por subscribe. Worker e inicialização assíncrona publicam; callback nunca chama watch nem adquire Mutex. `revision` avança em publicação; `invalidation_generation` avança em invalidação, inclusive lock intermediário, desktop desconhecido/trocado, hooks perdidos e overflow que invalida os dados. Contadores não dão wrap; exaustão fecha terminalmente com erro fixo, sem reusar uma geração.
- Helper novo `adapter/lifecycle.rs` contém mailbox fixa (latest session conhecido/desconhecido, dirty desktop/displays/hooks, invalidation generation) e o cursor de reconciliação testável. Não contém InputEvent nem histórico. Dirty/invalidation não podem ser apagados por leitura concorrente ou consulta mais antiga. Notices registrados durante consultas iniciais prevalecem antes de abrir gate; reconciliar/consultar outra vez se a revisão mudou. Lock+Unlock mantém incremento mesmo que latest session seja false. Callback de lock/desktop/hooks seta locked/end, limpa begin/ready e marca mailbox imediatamente. Worker consulta desktop/monitores fora do callback, publica snapshot e só então libera dados da geração nova. `emit_data` interrompe espera por stop OU nova invalidação; notice nunca disputa vaga em Msg/Out.
- Preservar política de RestartPolicy. Ao recriar hooks, conferir stop antes e depois; reset_input limpa held/ativação antigos. A política do worker não pode levar aviso terminal a `out.send` bloqueante. `terminal_error` é reservado a fim definitivo (hooks GiveUp/erro de instalação/invariante irrecuperável); falha de snapshot transitória mantém valid=false e gate fechado com diagnóstico fixo, permitindo reconciliação posterior. Não tratar desktop protegido conhecido como WTS desconhecido.
- Aquisição continuous: em `capture::run`, obter receiver do cap antes de `cap.start`, consumir imediatamente `borrow_and_update().clone()` com gate fechado; depois do start, consumir novamente ANTES de tornar dados/abertura elegíveis. Aquisição loopback: o mesmo em `module::drive`, antes e logo depois de start, antes do select/primeiro Activated. Nunca esperar só changed para a primeira leitura.
- Cursor `last_generation: Option<u64>` pertence ao consumidor, não ao receiver; `pending_invalidation: bool` não se perde ao trocar rx. Na primeira aquisição, se não existe sessão/abertura/dados antigos, reconciliar snapshot atual e só então estabelecer baseline; ao substituir receiver, comparar geração atual com último valor reconciliado ANTES de atualizar baseline. Se houve aumento, marcar/consumir invalidação, cancelar sessão/abertura, limpar batch e parar/join/descartar rx velho pelo B02. Atualizar baseline somente depois dessa limpeza. Um snapshot false válido não recupera o rx descartado. Reobter subscriber de mesmo adaptador mantém baseline; nova instância exige primeiro limpar a anterior e começar fechada. O helper não armazena apenas `last_generation = borrow().generation` na troca.
- `stop` pode publicar valid=false/revision nova, mas conserva geração/fatos pendentes e não zera mailbox cegamente. A troca interna stop/start não deve inventar uma invalidação por publicação e entrar em ciclo infinito de restart. Após start, snapshot novo é obrigatório antes da ativação. Um callback ocorrido durante a troca não pode desaparecer. A sequência de fechar/drenar/stop/join será a liberada por FIX1/I1, não a sequência defectiva da cópia congelada; antes de start aplicar a revalidação FIX1/I2, mesmo fora do select biased.
- Ordem no select biased: Stop/pausa/trava/cancelamento explícito; lifecycle atual/mudança; eventos de dados; conclusão de abertura/replay. Antes de aceitar Activated/forward ou concluir abertura, consumir lifecycle já pronto. Fila de controles não é movida para a fila de dados. Mudança de geração com estado final unlocked usa o mesmo encerramento B02, não apenas `SessionLocked(false)` (que não invalida sessão antiga).
- Quando Some(watch), Windows publica lock/zones/erro terminal SOMENTE pelo watch, retirando Notice da sync_channel; não duplica esses eventos pelo rx de dados. CaptureEvent continua existindo para adaptadores None, hotkeys/dados/deactivation existentes. No fechamento/drain B02, processar controles remanescentes sem reabrir captura, sem duplicar efeitos do watch.
- Quando None, manter `session::LockWatch::start` Linux e `initial_lock_snapshot` Windows do baseline: `SessionLocked(true|false)` explícito antes de produtor/enable. Consumidores começam fechados/sem snapshot, e somente um SessionLocked(false) inicial válido libera ativação nova; unknown publica true+erro, nunca false. Nenhuma edição em session.rs/portal.rs é necessária nesta fatia: tests demonstram preservação. A função Windows existente pode ser adaptada para publicar no watch, mas sua semântica WTS + desktop e testes não são removidos. `None` não autoriza usar false default.
- Unlock durante pausa não inicia hooks/captura. Ao retomar, novo start entrega snapshot fresco; não depender de evento de unlock que ocorreu enquanto o watcher estava parado. Em bloqueio de SO sem pausa/trava, manter a exceção B02 de produtor sem barreiras para observar unlock; false válido permite recriação posterior com barreiras, nunca replay. Loopback invalida/cancela o job e passa por sua limpeza atual, sem tentar ressuscitar a sessão do ensaio. Stop encerra o consumidor sem instalar novo produtor.

## Plano de quatro tarefas

### T1 · B03: relógio de clipboard

**Brief:** fechar overflow/envenenamento lógico mantendo ordenação e ações existentes. Um dono backend-clipboard; nenhuma alteração proto.

**Posse exata:** modificar `crates/clipsync-clipboard/src/module.rs`, `crates/clipsync-clipboard/src/msg.rs`, `crates/clipsync-clipboard/tests/module_mem.rs`. Testes unitários que precisam semear MAX ficam em `module.rs`; sem novo helper/crate. Entradas H01/B02 de ModuleEvent com `..` permanecem.

**Interface:** `const MAX_REMOTE_AHEAD: u64 = 1_000_000`; `fn new_offer(&mut self, text: String) -> Result<ClipOffer, &'static str>`. `msg::CLOCK_EXHAUSTED` e `msg::CLOCK_AHEAD` são textos fixos; CLOCK_AHEAD explica a janela lógica, sem conteúdo. ClipSend responde ModuleReply::Error; captura local registra last_block e retorna ao laço. ClipSendCurrent já delega a clip_send, preservá-lo.

- [ ] RED dirigido: `clock_admission_preserves_state` cobre 0/MAX/janela+1, limite exato e pausa/expiração/duplicação extrema; capturar current/last/lamport_max/caixa antes/depois. Não alterar semântica de TTL fora deste achado (o código atual usa ttl_ms==0 e expiração existente).
- [ ] RED dirigido: `local_clock_exhaustion_keeps_module_alive` semeia MAX-1 e MAX, tenta cópia/ClipSend/ClipSendCurrent, prova zero ofertas/MAX/0, erro/diagnóstico fixo e resposta posterior de estado/pausa. `equal_timestamp_tiebreak_and_last` usa dois PeerId, entrega invertida, Written/falha/conflito/Manual/ClipApply stale.
- [ ] Implementar checked_add com reserva de MAX; mover incorporação remota após pré-validação; usar maior par após escrita/aplicação e tratar Result em todos os chamadores. Não aumentar last em envio explícito que não copia localmente.
- [ ] GREEN/revisão: `cargo test --locked -p clipsync-clipboard`; `cargo clippy --locked -p clipsync-clipboard --all-targets -- -D warnings`. Registrar testes/ignorados e limites fake; entregar diff ao Sentinel/warden. Não executar profile/UI por arrasto.

### T2 · B05: bookkeeping do WinEmulate real

**Brief:** tornar falha parcial conservadora e impedir nova injeção enquanto soltura não foi confirmada. Um dono backend-emulação.

**Posse exata:** somente `crates/clipsync-input/src/adapter/win/emulate.rs`, inclusive testes cfg(windows) no próprio arquivo. Não editar win/ffi.rs, adapter/mod.rs, win/capture.rs ou H01/B02. Helper de conjuntos privado fica neste arquivo, evitando posse cruzada.

**Interface:** adicionar `cleanup_pending: bool`; transformar `send(batch)` estático em seam privada por instância `send(&mut self, batch: &[Synth]) -> Result<(), InputError>` e todos os callsites passam por ela. A implementação de produção continua chamando clipsync_win::send. Test backend privado cfg(test) também fornece desktop/monitores/cursor/virtual_rect usados pelo método real; evita chamada física acidental antes/depois do send falso. Não mudar trait público nem FFI; sem logs/Debug de conjuntos.

- [ ] RED cfg(windows): chamar `EmulateAdapter::emit` real com down anterior+up falho, down+up, dois códigos e erro parcial; conferir possible/cleanup_pending. Chamar `release_all` duas vezes com primeiro Err/segundo Ok, verificar mesmos ups e limpeza só no segundo.
- [ ] RED cfg(windows): `emit_waits_for_cleanup` e `enter_waits_for_cleanup` chamam os dois métodos reais; send falso da limpeza falha => nenhuma chamada nova de down/move absoluto; reparação da limpeza => nova operação ocorre uma vez. Cobrir sucesso integral e restos zerados após erro. Não substituir estes casos por teste do helper.
- [ ] Implementar desired/possible, commit conservador e limpeza antes de emit/enter; conservar todos os códigos no erro, inclusive up do mesmo lote; release não drena e stop devolve erro. Nenhuma repetição automática de eventos do lote falho.
- [ ] GREEN Windows: `cargo test --locked -p clipsync-input adapter::win::emulate::tests` em runner Windows; localmente cross-clippy com target já usado no report (`CC_x86_64_pc_windows_gnu=true AR_x86_64_pc_windows_gnu=true cargo clippy --locked --target x86_64-pc-windows-gnu -p clipsync-input --all-targets -- -D warnings`). Ausência de runner deixa execução desses testes PENDENTE, mesmo que cross-clippy passe. Review por Sentinel/warden; fake não prova SendInput/UIPI/soltura física.

### T3 · B01: histórico agrupado até Diagnóstico e casca

**Brief:** implementar o schema acima de ponta a ponta e verificar candidatos/chamadas fake. Um único dono B01 após liberação dos atuais donos.

**Posse exata:** `crates/clipsync-core/src/transport.rs`; `crates/clipsync-proto/src/control.rs`; `crates/clipsync-ui/src/{events,diag,msg}.rs`; `crates/clipsync-ui/src/traversal.rs` SOMENTE literal de BlockRecord no teste existente; `crates/clipsync-ui/tests/{events,diag,fixtures}.rs`; `crates/clipsync-app/src/{events,state}.rs`; `crates/clipsync-app/ui/js/telas/diagnostico.js`. Testes unitários adicionais ficam nos próprios arquivos; JSON antigo das fixtures permanece como entrada de compatibilidade. Nenhuma edição em app/tray/msg, ui/lib/status ou mock necessários. Se fixture de saída exata exigir atualização, preservar um caso antigo sem campos em fixtures.rs em vez de substituir toda evidência por JSON novo.

**Interfaces:** schema B01 integral acima; LRU em VecDeque já existente; estado de agregação único no poll e gate único em AppState. Não criar notificador por PeerBlocked.

- [ ] RED backend: 1.000 negativas => um grupo/count1000; A/B/repetir A conserva B e move A ao início; causa21 expulsa LRU; nomes diferentes mesma chave; relógio de parede regressivo não muda LRU; count MAX satura. JSON antigo => count1/último normalizado.
- [ ] RED UI/casca: snapshot idêntico zero delta; aumento só no grupo correto; reordenação/nome sem aviso; count reiniciado/início novo sem underflow; 21 causas/pares marcam diversidade pelo-menos e memória≤20. Fake Instant testa primeiro aviso, 29.999 s, 30 s, resumo em poll sem novos eventos, resync, ausência/reaparecimento de agente, block off/on sem replay/reset, e device_removed independente. Casca testa candidatos duplicados e erro do notificante dentro da mesma janela com no máximo uma chamada fake. Nenhuma contagem é chamada de total absoluto da rede.
- [ ] Implementar backend/schema primeiro, depois UI/diag, então casca e tela, respeitando gates de posse; ajustar literal unitário em traversal.rs. Render/export mostra contagem/última ocorrência e defaults; preservar redação de dados no suporte.
- [ ] GREEN: `cargo test --locked -p clipsync-proto`; `cargo test --locked -p clipsync-core --lib`; `cargo test --locked -p clipsync-ui --test events --test diag --test fixtures`; testes unitários dirigidos de events/diag; `cargo test --locked -p clipsync-app` onde toolchain Tauri local suportar. Clippy dos crates modificados. Se Tauri não compilar por infraestrutura, registrar limite sem declarar gate testado. Maestro valida Diagnóstico via portal depois da integração; notificações físicas Windows/Linux permanecem manuais. Review Sentinel/warden antes do handoff.

### T4 · B07: mailbox Windows, watch e consumidores B02

**Brief:** remover perda de lifecycle na origem/contrapressão mantendo snapshot inicial, prioridades e cancelamento atuais. Um dono backend-lifecycle, após revisão/liberação H01/B02 INCLUINDO FIX1/I1+I2; não partir os consumidores entre dois implementadores. Ao iniciar, reler os arquivos finais e a revisão de FIX1 para localizar a fronteira correta; as linhas do snapshot são apenas referências históricas.

**Posse exata:** `crates/clipsync-input/src/adapter/{mod.rs,lifecycle.rs(new),fake.rs,win/capture.rs}`; `crates/clipsync-input/src/{capture,module}.rs`; novo `crates/clipsync-input/tests/lifecycle_watch.rs`. Fakes/harness do teste novo ficam nesse arquivo, sem reescrever support/revocation atuais. Tests cfgWindows adicionais no próprio win/capture.rs. `session.rs` e `portal.rs` são fontes de regressão, somente leitura; `clipsync-win`, proto e rede não mudam.

**Interfaces:** CaptureLifecycle/receiver/schema e protocolo de aquisição acima. FakeCapture deve poder escolher `None` (default preserva testes atuais) ou watch pela mesma instância e publicar snapshots/gerações sem depender do rx de dados; controles de teste não viram API wire. Registrar helper lifecycle em adapter/mod.rs e usar pelos dois consumidores.

- [ ] RED portável: encher dados, aplicar lock+unlock antes de leitura, consumir snapshot final unlocked => sessão/abertura antiga cancelada, batch/rx descartados e nenhuma Activated/Input antiga transmitida. Reaquisição de receiver com geração maior conserva invalidação; estado inicial false válido permite só captura nova; snapshot inválido/None sem evento inicial permanece fechado. Testes distinguem `changed` de leitura imediata.
- [ ] RED Windows/mailbox: fila8000/saída4096 cheias, cada Notice e overflow conservam fatos; callback não espera fila; worker publica mesmo com out cheio. Display/hooks não somem no drain, mudança durante consulta inicial não é sobrescrita; Stop+HooksLost não reinstala; generation/revision não resetam no stop/start; held antigo não reaparece. Mailbox constante, watch mantém um snapshot, sem vetor/fila de notices.
- [ ] RED consumidores/fallback: continuous E loopback, abertura simultânea a lifecycle, pausa/trava/Stop simultâneos a unlock, start pausado zero hooks, retomada após unlock durante pausa lê snapshot fresco; None executa snapshots explícitos Linux e semântica WTS/desktop do backend atual. Cobrir SessionLocked(true) seguido de produtor sem barreiras para desbloqueio, sem permitir exceção durante pausa/manual lock. Tests existentes de WTS known/unknown+desktop continuam compilando. Preservar e reexecutar regressões FIX1: produtor realmente bloqueado em send precisa concluir join, e stop controlado por gate com Stop/Paused/Lock/host.paused/cancelamento da bridge durante a espera não pode aumentar starts.
- [ ] Implementar mailbox/watch no adaptador, depois reconciliação compartilhada e ambos consumidores; revisar pontos de reset_rx, drain e finalização. Snapshot inicial e startup são parte do mesmo handoff; não integrar só metade do canal. Se revisão H01/B02 já tiver introduzido canal equivalente, adaptar este schema ao canal existente e registrar equivalência no contrato antes do código, sem duplicar.
- [ ] GREEN: `cargo test --locked -p clipsync-input`; clippy input/win em host e cross target com flags do report; `scripts/conferir-unsafe.sh`, `scripts/conferir-privacidade.sh`. Executar testes cfgWindows em CI Windows quando disponível; cross-clippy é só compilação. Reexecutar os cenários H01/B02 afetados do baseline, sem baixar asserções/limites. Sentinel/warden revisa especialmente ordem de snapshot/geração/cleanup e liberação de Stop; portal/desktop/hook físico fica manual.

## Foco da revisão e entrega

1. Resync/restart e relógio de parede não burlam cadência nem LRU — T3.
2. Estado extremo/saturado não gera timestamp emitido inválido ou delta negativo — T1/T3.
3. Falha parcial não apaga up pendente e enter não contorna cleanup — T2.
4. Unlock coalescido, troca de rx e consulta inicial concorrente não apagam invalidação — T4.
5. Pausa/Stop e fallback None nunca abrem gate por default ou ressuscitam hooks — T4.

Entrega por tarefa: arquivos exatos, RED/GREEN observado (não inferido), comandos/logs, review independente, limitações de plataforma, handoff de posse e riscos residuais já delimitados. Não atribuir novos testes ao RED histórico H01/B02. O report atual registra 192 input/92 proto+clipboard/42 core/4 win e checks como verdes relatados, mas também execução ampla interrompida em profile de outra fatia; a revisão independente posterior reprovou I1/I2 e aprovou por leitura o wrapper WTS. Isso NÃO é baseline integrado verde certificado. Maestro fixa baseline após as correções Rivet/backend FIX1 e respectivas revisões, então faz a rodada integrada pertinente, sem reabrir auditoria nem inventar achados.

## Decisões de arquitetura tomadas nesta proposta

- T3 tem um dono de ponta a ponta; serializar somente handoffs necessários, não toda a rodada.
- Estado do throttle UI no poll existente; defesa da casca no AppState existente. Nenhum schema novo de Notification.
- Contagem é observada; key+at+count não consegue identificar reincarnação idêntica invisível e não recebe ID wire novo.
- Watch por instância Windows, default None nos outros adaptadores, com sessão/desktop explícitos e consumo inicial imediato.
- Baseline de generation pertence ao consumidor e só avança depois da limpeza correspondente; stop/start não reinicializa canal/geração.
- B05 testa métodos reais via seam local sem FFI, e prova Windows executada permanece separada de compilação.

Registro derivado da proposta /tmp/clipsync-s9-seg-complemento-execucao.md, conferida pelo Maestro contra os quatro itens e os adendos Sentinel. Registrar desenho não prova implementação, testes ou aprovação física.

### Adendo de posse B07 · retomada de 29/10/2026
O Maestro conferiu que Capture.start nos harnesses tests/support/capture_backpressure.rs e module_backpressure.rs devolve rx sem snapshot inicial. Fica ampliada a posse Task4 SOMENTE para esses starts publicarem SessionLocked(false) explícito antes de disponibilizar o produtor, adequando o fake ao contrato None. Preservar todos os asserts, gates, limites, contadores e testes FIX1; não introduzir default unlocked no consumidor nem dispensar o caso sem snapshot no teste novo. Esta exceção substitui a proibição genérica de editar support apenas nesse recorte. Nenhum outro arquivo support/revocation liberado. Baseline13/13 anterior registrado no plano segurança; reexecutar FIX1 depois da mudança.

Adendo adicional de posse B07: autorizado o mesmo snapshot SessionLocked(false) no start de tests/support/stop_capture.rs, antes de expor tx, pois os dois testes FIX1 de send realmente bloqueado usam esse fake None. Preservar o produtor pendente, capacidade/asserções, join real e todos os gates; sem outra mudança nesse arquivo.

### Adendo de estímulo FIX1 / B07 · retomada posterior de 29/10
O Maestro conferiu a regressão documentada: o snapshot falso automático em todo start de stop_capture contradiz o lock intencional enfileirado pelo cenário FIX1. A posse B07 agora permite remover SOMENTE essa publicação automática de stop_capture::Capture.start e inserir SessionLocked(false) explícito nos dois testes fix1_full_producer_send_is_joined_on_reset_and_shutdown e fix1_loopback_joins_producer_blocked_in_full_send_before_leave antes da primeira Activated; no continuous, também após start3 antes da nova Activated. Preservar o lock herdado no start2 até o unlock explícito existente, todos os asserts, prazos, contadores, produtor realmente pendente e join. Não mudar gate None do produto para unlocked por default. Esse recorte de estímulos substitui o adendo anterior de start do terceiro fake; os outros dois starts mantêm snapshot explícito. Não autoriza outra alteração de support.
