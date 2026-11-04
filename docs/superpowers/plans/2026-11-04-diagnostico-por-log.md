# Diagnóstico verificável por log: plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** fazer o ClipSync contar, no arquivo de log de cada máquina, até onde uma tentativa de travessia chegou e qual foi o primeiro ponto que falhou, com a linha de log como prova, para o usuário e a IA chegarem à mesma conclusão sem adivinhar.

**Architecture:** um `Layer` próprio do `tracing` escreve JSONL no perfil, com escritor único no processo do agente, rotação de 5 MiB e descarga por linha. Os eventos do vocabulário do contrato §17.2 são emitidos nos pontos reais de inventário, rotas, captura, sessão, emulação e retorno. Um verificador offline lê os arquivos das duas máquinas e imprime a cadeia com `[ok]`, `[falhou]` ou `[sem dado]`.

**Tech Stack:** Rust do workspace, `tracing`/`tracing-subscriber` já fixados, `serde_json`, `time`, `tempfile` (testes), JavaScript ES modules sem framework novo.

**Spec:** `docs/superpowers/specs/2026-11-04-diagnostico-por-log-design.md`, aprovado pelo usuário em 04/11/2026.

**Contrato:** `docs/contrato.md` §17 (v1.40). Onde este plano e o contrato divergirem, vale o contrato; a divergência volta ao Maestro antes de qualquer código.

## Restrições globais

- Sem crate de terceiros novo. Só entram arestas para crates já fixados no `[workspace.dependencies]`: `tracing-subscriber` em `clipsync-core`; `serde_json` em `clipsync-cli` e `clipsync-input`; `time = { workspace = true, features = ["parsing"] }` em `clipsync-cli` para o `--desde` (absoluto e duração relativa); `tracing-subscriber` como dev-dependency em `clipsync-input`. `Cargo.lock` só muda se o grafo de pacotes mudar; conferir `git diff Cargo.lock` e não atualizar versão por conta própria. **Toda mudança em `Cargo.toml` exige `cargo update --workspace --offline` imediato**, antes de teste ou build.
- `unsafe` continua só em `clipsync-win/src/ffi.rs`. Nenhum `unwrap`, `expect`, `panic`, `todo`, `unimplemented` ou `dbg!`. `clippy -D warnings` nos dois sistemas.
- Nenhum evento carrega conteúdo de clipboard, preview, keycode, tecla ou texto digitado. `peer` é sempre `PeerId::short()` (o `Display` já é curto). Nome de par só quando o status já expõe.
- O arquivo é escrito só pelo processo do agente. Nenhum comando da CLI, teste ou interface abre o arquivo para escrita; testes usam diretório temporário próprio.
- Escritor único de `crates/clipsync-input/src/module.rs`, `module/organization.rs` e `capture.rs` por vez. F1 entrega os pontos de input e só então F2a começa; F2a e F2b são em série.
- As duas únicas mudanças de comportamento são as do contrato §17.4. O que a interface mostra em `capture.rs` não muda; o destino passa a recusar monitor ausente com `ApplyFailed`.
- Texto em português; sem travessão em texto de interface; sem `innerHTML`, `style=` ou `<script>` inline. A interface só fala pelo `Ui::call` e pelos eventos `clipsync://event`.
- O app Tauri não compila neste notebook (faltam headers do WebKitGTK e não há sudo). A prova dele é o CI. Não criar `target/` em `/tmp`; não usar automação de navegador; o simulador é do Maestro em `127.0.0.1:5174`.
- Cada tarefa termina com o teste inicialmente vermelho, o verde pertinente e a verificação registrada. Commit em português, minúsculo, imperativo, sem coautor. Push, tag e release são do Maestro.

## Foco da revisão

1. **F1, escritor:** um único processo abre o arquivo; 0600/0700; uma linha por evento com `flush`; rotação sem perder a linha corrente; contador de descartados limitado; `log_path` puro e idêntico em CLI, status e interface.
2. **F1, esquema:** `v`, `ts` com milissegundos, `host`, `alvo`, `evento`, listas como JSON; nenhum campo proibido; `session` e `attempt` correlacionados; `capture.failed` separado de `capture.start`; `capture.ready` só com barreira.
3. **F2a:** `attempt.begin`/`barrier.activated`/`attempt.filtered` sem mudar a interface; erros de `open`/`opened` por variante; `session.closed` com motivo e totais, sem duplicata; `return.received` no ponto certo.
4. **F2b:** monitor ausente recusa com `ApplyFailed` no fio e `monitor_not_found` no log; a origem vê a recusa; emulação só depois de `enter` válido; nenhuma tecla em log; portal/hooks só com contagem e motivo.
5. **F3:** ordem da cadeia; `--ultima`, `--sessao` e `--desde` reproduzíveis em logs copiados; esquema futuro para com código 3; arquivo ausente/rotacionado como `[sem dado]`, nunca como sucesso.
6. **F4:** cauda de 400 linhas redigida com as mesmas agulhas; caminho visível na tela; mock e Node cobrem o estado sem log.
7. **F5:** documento e contrato dizem a mesma coisa; nenhum comando promete o que o código não faz.

## Posse, ordem e pontos de integração

Executar F1 (tarefas 1 a 4) → F2a (5 e 6) → F2b (7 e 8) → F3 (9) → F4 (10) → F5 (11). F3 pode começar depois da tarefa 4, em paralelo com F2, porque só lê os eventos já definidos. F4 depende de F2 para a cauda ter conteúdo real. F5 é o último.

| Fatia | Dono | Arquivos novos e existentes sob posse |
| --- | --- | --- |
| F1 · core/CLI/app/UI | Rivet · slice | `crates/clipsync-core/src/logging.rs` (novo), `lib.rs`, `agent.rs`, `msg.rs` (só texto), `transport.rs` (só `agent.up`/`peer.*`), `Cargo.toml`; `crates/clipsync-proto/src/control.rs`; `crates/clipsync-cli/src/main.rs`, `verificar.rs` (novo), `cmds.rs`, `msg.rs`, `Cargo.toml`; `crates/clipsync-app/src/main.rs`; `crates/clipsync-ui/src/status.rs` |
| F1 · input | Vector · input | `crates/clipsync-input/src/module.rs`, `module/organization.rs`, `capture.rs` (só `capture.start`/`failed`/`ready`), `Cargo.toml`; `tests/support/logs.rs` (novo), `tests/s11_log.rs` (novo) |
| F2a · origem | Vector · input | `crates/clipsync-input/src/capture.rs`, `route_session.rs`, `msg.rs`; trecho de `module.rs` de `drive`/`run_loopback`; testes `capture_backpressure.rs`, `stop_capture.rs`, `s11_log.rs` |
| F2b · destino | Vector · input | `crates/clipsync-input/src/module.rs` (`on_enter`, `on_datagram`, emulação), `module/organization.rs` (`enter_routes`/emulação), `adapter/portal.rs`, `adapter/win/*`; testes `s10_routes.rs`, `module_mem.rs`, `layout_capture.rs` |
| F3 · verificador | Rivet · slice | `crates/clipsync-cli/src/verificar.rs`, `main.rs`, `Cargo.toml`; `crates/clipsync-cli/tests/verificar_travessia.rs` (novo) |
| F4 · interface | Pixel · interface | `crates/clipsync-ui/src/diag.rs`; `crates/clipsync-app/ui/js/telas/diagnostico.js`; `crates/clipsync-app/ui-dev/mock/*`, `ui-dev/testes/diagnostico-log.test.mjs` (novo) |
| F5 · documento | Maestro | `docs/verificar-por-log.md` (novo), `docs/contrato.md` §17 |

Arquivos compartilhados e regra de entrega: `crates/clipsync-input/src/module.rs` é de Vector em F1, F2a e F2b, sempre um por vez. `crates/clipsync-cli/src/main.rs` é de Rivet em F1 e F3; a tarefa 4 entrega a posse depois do commit. `crates/clipsync-ui/src/status.rs` é de Rivet em F1; `diag.rs` é de Pixel em F4. Nenhum arquivo de `docs/` além dos da F5, exceto o contrato §17 já escrito pelo Maestro.

## Interfaces compartilhadas

### `clipsync_core::logging` (F1)

```rust
pub fn log_path(profile_dir: &Path) -> PathBuf;
pub fn init();
pub fn open_file(profile_dir: &Path, device_name: &str) -> Result<(), CoreError>;
pub fn preopen_dropped() -> u64;
```

- `log_path` devolve `<profile_dir>/logs/clipsync.jsonl` sem tocar no disco.
- `init` instala a camada de stderr (mesmo filtro de `CLIPSYNC_LOG`, padrão `warn`) e a camada de arquivo com filtro próprio; usa `try_init` e não entra em pânico se já houver assinante global.
- `open_file` cria `logs/` e abre o arquivo em append; é idempotente; falha devolve `CoreError::Io` e o agente segue sem arquivo.
- `preopen_dropped` devolve o contador de eventos descartados antes de `open_file`, com teto de 200 no valor reportado.

### Estado exposto (F1)

```rust
// clipsync-proto/src/control.rs
pub struct AgentStatus {
    // campos atuais
    #[serde(default)]
    pub log_path: Option<String>,
}

// clipsync-ui/src/status.rs
pub struct AppState {
    // campos atuais
    #[serde(default)]
    pub log_path: String,
}
```

`AgentStatus.log_path` é `Some` no agente novo e `None` em agente antigo. `AppState.log_path` é sempre preenchido por `logging::log_path(&p.dir)`; não depende de agente, status ou arquivo existente.

### Verificador (F1 esqueleto, F3 completo)

```rust
pub(crate) struct Entrada {
    pub origem: PathBuf,
    pub destino: Option<PathBuf>,
    pub sessao: Option<u32>,
    pub ultima: bool,
    pub desde: Option<String>,
    pub json: bool,
}

pub(crate) fn executar(e: &Entrada) -> Result<Saida, FalhaVerificar>;

pub(crate) struct Saida {
    pub tentativa: Option<u32>,
    pub sessao: Option<u32>,
    pub passos: Vec<Passo>,
    pub quebra: Option<String>,
}

pub(crate) struct Passo {
    pub nome: &'static str,
    pub estado: Estado,
    pub linha: Option<String>,
    pub motivo: Option<String>,
}

pub(crate) enum Estado { Ok, Falhou, SemDado }
```

`Saida::texto()` devolve o texto humano; `Saida::json()` devolve o JSON do contrato §17.3.5. `FalhaVerificar` tem `tipo: TipoFalha` (`Arquivo`, `Esquema`) e `Display`; o `main` mapeia para código 3. Quebra de cadeia não é `FalhaVerificar`: é `Saida` com `quebra: Some(...)` e código 1.

Nomes dos passos, na ordem: `inventario_origem`, `inventario_destino`, `rotas`, `captura`, `tentativa`, `sessao`, `entrada_destino`, `emulacao`, `datagramas`, `retorno`, `fechamento`. F1 implementa os cinco primeiros; F3 acrescenta os seis restantes sem mudar o tipo.

### Eventos (F1 a F2b)

O vocabulário e os campos são os do contrato §17.2. A emissão é direta:

```rust
tracing::info!(evento = "routes.built", rotas = n, barreiras = m, pares = %json, "...");
```

Campos de lista (`barreiras`, `uids`, `pares`, `totais`) são serializados com `serde_json::to_string` e o escritor os reconhece pelo nome, inserindo o valor JSON em vez de string. `pos` é fração `0.0..=1.0` com três casas. Nenhum evento usa `Debug` de tecla ou conteúdo.

### Teste de eventos (F1, Vector)

`crates/clipsync-input/tests/support/logs.rs`:

```rust
pub struct Evento { pub evento: String, pub campos: BTreeMap<String, String> }

pub struct Captura {
    pub eventos: Arc<Mutex<Vec<Evento>>>,
    _guard: tracing::subscriber::DefaultGuard,
}

pub fn capturar() -> Captura;

impl Captura {
    pub fn tem(&self, evento: &str) -> bool;
    pub fn primeiro(&self, evento: &str) -> Option<Evento>;
    pub fn todos(&self, evento: &str) -> Vec<Evento>;
    pub fn limpar(&self);
}
```

Usar `#[tokio::test]` (current_thread) para o `DefaultGuard` valer nas tarefas do `MemHub`. O `Layer` de teste não filtra: captura todo evento com `evento`, inclusive os de produção. `tracing-subscriber` entra como dev-dependency de `clipsync-input`.

## F1 · Fatia 1: vertical mínimo

### Tarefa 1: camada de arquivo, rotação, permissões e caminho puro

**Dono:** Rivet · slice.

**Arquivos:** criar `crates/clipsync-core/src/logging.rs`; editar `crates/clipsync-core/src/lib.rs` (`pub mod logging;`) e `crates/clipsync-core/Cargo.toml` (`tracing-subscriber.workspace = true`; `serde_json` e `time` já existem).

**Interfaces:** as de `clipsync_core::logging` acima. Interno: `FileState`, `Sink`, `FileLayer`; `Sink::open(dir, host, limit)` com `limit` injetável só em teste; `FileState::write_line(&str)`; `FileLayer` implementa `Layer<S>`.

- [ ] **1. Escrever o RED.** Em `logging.rs`, `#[cfg(test)] mod tests`:
  - `caminho_puro_nao_cria`: `log_path(tmp.path()) == tmp.path().join("logs").join("clipsync.jsonl")` e `logs/` não existe depois.
  - `abre_0600_e_uma_linha_por_evento`: `Sink::open` num diretório temporário; escrever dois eventos; ler o arquivo; duas linhas, cada uma JSON válido com `v:1`, `ts` com `.` e `Z`, `nivel`, `host`, `alvo`, `evento`; no Unix, `mode & 0o777 == 0o600` e o diretório `0o700`.
  - `rotaciona_em_5_mib`: `Sink::open` com limite pequeno; escrever até passar; existem `clipsync.jsonl`, `.1` e, no segundo giro, `.2`; a primeira linha do arquivo atual é a que excedeu o limite, não uma linha perdida.
  - `descarta_antes_de_abrir_e_conta`: montar `FileLayer` sobre `tracing_subscriber::registry()` com `tracing::subscriber::with_default`; emitir um `evento` antes de `open_file`; o arquivo não existe e o contador é 1.
  - `lista_vira_json`: emitir `evento="capture.ready"` com `barreiras = %"[{\"peer\":\"aa\"}]"`; a linha tem `barreiras` como array, não string.
- [ ] **2. Rodar e ver falhar.** `cargo test -p clipsync-core --locked logging`; esperado: não compila ou os testes novos falham.
- [ ] **3. Implementar.** `log_path`; `Sink` com `OpenOptions::append`, `mode(0o600)` no Unix, `DirBuilder` 0700; `write_line` com uma `write_all` da linha mais `\n` e `flush`; rotação antes da linha que excede 5 MiB, `remove_file(.2)`, `rename(.1,.2)`, `rename(atual,.1)`, reabrir; `FileState` com `Mutex<Option<Sink>>` e `AtomicU64`; `FileLayer` com `Visit` que ignora `message` (vira `mensagem`) e `evento` (vira `evento`), converte `i64`/`u64`/`f64`/`bool`/`str`/`Debug` e trata os nomes de lista; `ts` com `time::format_description` (milissegundos, `Z`) em `OnceLock` e fallback vazio; `init` com `EnvFilter::try_from_env("CLIPSYNC_LOG")` para stderr e a diretiva do contrato para o arquivo; `CLIPSYNC_LOG_ARQUIVO` substitui a diretiva; `try_init` sem pânico; `open_file` idempotente e `preopen_dropped` com teto 200.
- [ ] **4. Rodar e ver passar.** `cargo test -p clipsync-core --locked logging`; `cargo clippy -p clipsync-core --all-targets --locked -- -D warnings`.
- [ ] **5. Commit.** `adiciona arquivo de log verificável no perfil`.

### Tarefa 2: inicialização única, status, `log caminho` e infraestrutura

**Dono:** Rivet · slice.

**Arquivos:** `crates/clipsync-core/src/agent.rs`, `transport.rs`, `msg.rs` (só se precisar de texto); `crates/clipsync-proto/src/control.rs`; `crates/clipsync-cli/src/main.rs`, `cmds.rs`, `msg.rs`; `crates/clipsync-app/src/main.rs`; `crates/clipsync-ui/src/status.rs`; `crates/clipsync-cli/tests/verificar_log.rs` (novo).

**Interfaces:** `AgentStatus.log_path`, `AppState.log_path`, `Cmd::Log` com `LogKind::Caminho`; `agent::run_reserved` chama `open_file`; `main` dos dois binários chama `logging::init()`; `cmds::status` imprime a linha do log nos dois ramos (agente rodando e parado), com `msg::log_line(path: &str) -> String` devolvendo `log: <path>`.

- [ ] **1. Escrever o RED.** 
  - `crates/clipsync-proto/tests/control.rs` (ou junto do tipo): um `AgentStatus` antigo, sem `log_path`, desserializa com `None`; o novo serializa o campo.
  - `crates/clipsync-cli/tests/verificar_log.rs`: rodar `CARGO_BIN_EXE_clipsync --profile <tmp> log caminho`; saída igual a `<tmp>/logs/clipsync.jsonl`, código 0, e o arquivo não foi criado.
  - No mesmo `verificar_log.rs`, com perfil criado por `init --name t --modules input --fake-adapters` e agente parado, `status` sai 4 e a saída contém `log: <tmp>/logs/clipsync.jsonl`; teste unitário de `msg::log_line` cobre o texto.
  - Em `status.rs`, teste de `app_state`/serialização: `log_path` presente e igual a `log_path(&p.dir)`, mesmo com agente parado (usar o harness de status existente; se não houver, testar o campo no `AppState` construído).
- [ ] **2. Rodar e ver falhar.** `cargo test -p clipsync-proto -p clipsync-cli -p clipsync-ui --locked`; esperado: campo/comando inexistente.
- [ ] **3. Implementar.** `AgentStatus.log_path: Option<String>` com `#[serde(default)]`; `status()` preenche `Some(logging::log_path(&self.profile.dir).display().to_string())`; `AppState.log_path: String` com `#[serde(default)]` e `app_state` preenche; `run_reserved`, após `config` e antes de `Mesh::start`, chama `logging::open_file(&p.dir, &cfg.device_name)` e em erro só faz `tracing::warn!` (sem abortar); `agent.up` ganha `evento="agent.up"` e `descartados`; `peer.connected`/`peer.disconnected` ganham `evento` e mantêm campos; remover `init_logging` dos dois binários e chamar `clipsync_core::logging::init()`; `Cmd::Log` com `LogKind::Caminho` imprime `logging::log_path(&profile.dir).display()`; `cmds::status` imprime `msg::log_line(&logging::log_path(&profile.dir).display().to_string())` no ramo do agente rodando e no `AgentNotRunning`, sem depender de `AgentStatus`; texto novo só em `clipsync-cli/src/msg.rs`.
- [ ] **4. Rodar e ver passar.** `cargo test -p clipsync-core -p clipsync-proto -p clipsync-cli -p clipsync-ui --locked`; `cargo clippy -p clipsync-core -p clipsync-proto -p clipsync-cli -p clipsync-ui --all-targets --locked -- -D warnings`.
- [ ] **5. Commit.** `abre o log no agente e mostra o caminho`.

### Tarefa 3: eventos de inventário, rotas, captura e entrada

**Dono:** Vector · input. **Depende:** tarefa 1 (para o esquema) e tarefa 2 (para `agent.up`).

**Arquivos:** `crates/clipsync-input/src/module/organization.rs`, `module.rs`, `capture.rs`; `crates/clipsync-input/Cargo.toml`; criar `tests/support/logs.rs` e `tests/s11_log.rs`.

**Interfaces:** eventos `inventory.prepared|sent|received|missing`, `routes.built|empty`, `capture.start`, `capture.failed`, `capture.ready`, `enter.received|refused`; helper `refuse_on_with` (privado) para o motivo fino; `serde_json` como dependência.

- [ ] **1. Escrever o RED.** Em `tests/s11_log.rs`, usar o harness de `tests/layout_capture.rs` (dois nós `MemHub`, `FakeCapture`, layout com uma ligação):
  - `inventario_rotas_e_captura_aparecem`: após preparar/aplicar o layout e abrir a captura, o log tem `inventory.prepared` (origem), `inventory.sent` (origem→destino), `inventory.received` (destino), `routes.built` com `rotas >= 1`, `barreiras >= 1` e `capture.ready` com `barreiras` não vazio; o `peer` é curto.
  - `sem_inventario_remoto_nao_monta_rotas`: par conectado v4, autorizado, sem módulo de entrada no destino (não envia `InventoryV4`); a origem tem `inventory.sent`, mas não `inventory.received`; `routes.empty{motivo:"no_remote_inventory"}` e nenhum `capture.ready`.
  - `falha_do_portal_vira_capture_failed`: fake de captura que devolve `InputError::PermissionDenied` no `start`; a origem loga um `capture.start` e um `capture.failed{adaptador, motivo:"portal_denied"}`, não dois `capture.start`; a interface continua com o mesmo `Note::Block`.
  - `entrada_registra_recebida_e_recusada`: sessão legada/v4 que chega ao destino; `enter.received` com `mensagem` correta; um `Enter` pausado/não autorizado gera `enter.refused` com motivo e o fio com `Rejected`; nenhum evento contém tecla.
- [ ] **2. Rodar e ver falhar.** `cargo test -p clipsync-input --locked --test s11_log`; esperado: eventos ausentes.
- [ ] **3. Implementar.**
  - `organization.rs::observe_inventory`, depois de `catalog.insert`: `inventory.prepared` com `monitores` e `uids` (JSON).
  - `announce_inventory`: dentro do futuro já spawnado, no `Ok` de `send_reliable_on`, `inventory.sent` com `peer`, `monitores`, `revisao`; no `Err`, `inventory.missing` com `peer` e `motivo="send_failed"`.
  - `receive_inventory`, depois do `insert` Ok: `inventory.received` com `peer=from`, `monitores`, `revisao`.
  - `organization_targets`: um `motivo` local (`Option<&'static str>`) marcado em cada `continue` (os nomes do contrato §17.2.3); no fim, `routes.built` com `rotas` (links), `barreiras` (targets) e `pares`; se `targets.is_empty()`, `routes.empty{motivo}`. Não logar dentro do laço por ligação.
  - `capture.rs`, antes de `cap.start`: `capture.start` com `adaptador=cap.name()`; no `Err`, `capture.failed` com `adaptador` e `motivo` (`portal_denied`/`adapter_failed`); no `Ok` com `rx`, antes do `Note::Event(capture_ready)`: `capture.ready` com `adaptador` e `barreiras` (lista de `{peer, monitor, borda, trecho}` a partir de `targets`, `monitor` nulo quando não há).
  - `module.rs`, nos quatro braços de entrada (`EnterRoutes`, `Enter`, `EnterAt`, `EnterSpan`): `enter.received` com `session`, `peer`, `mensagem` (`enter_routes`, `enter`, `enter_at`, `enter_span`).
  - `refuse_on`: logar `enter.refused` com `session`, `peer` e `motivo` do `RejectReason`; criar `refuse_on_with(..., motivo: &'static str)` que envia o mesmo `Rejected` e loga o motivo fino, para F2b usar `monitor_not_found`.
  - `Cargo.toml`: `serde_json.workspace = true`; `[dev-dependencies] tracing-subscriber.workspace = true`.
- [ ] **4. Rodar e ver passar.** `cargo test -p clipsync-input --locked --test s11_log`; `cargo test -p clipsync-input --locked --test layout_capture --test module_mem`; `cargo clippy -p clipsync-input --all-targets --locked -- -D warnings`.
- [ ] **5. Commit.** `registra inventário rotas e entrada no log`.

### Tarefa 4: esqueleto do verificador com os pré-requisitos

**Dono:** Rivet · slice.

**Arquivos:** criar `crates/clipsync-cli/src/verificar.rs`; editar `crates/clipsync-cli/src/main.rs` (`mod verificar;`, `Cmd::VerificarTravessia`), `crates/clipsync-cli/Cargo.toml` (`serde_json.workspace = true`; trocar `time.workspace = true` por `time = { workspace = true, features = ["parsing"] }`); criar `crates/clipsync-cli/tests/verificar_travessia.rs`.

**Interfaces:** as de `Entrada`/`Saida`/`Passo` acima, com os cinco primeiros passos.

- [ ] **1. Escrever o RED.** Em `verificar_travessia.rs`, escrever logs sintéticos (uma linha JSON por evento) e chamar `executar`:
  - `sem_tentativa_mostra_prerequisitos`: origem com `inventory.prepared`, `inventory.sent`, `routes.empty{no_remote_inventory}`, sem `capture.ready`; saída com `[falhou] rotas`, `quebra = Some("rotas")`, `tentativa = None`; texto contém "nenhuma tentativa registrada".
  - `tudo_pronto_sem_tentativa`: inventário dos dois lados, `routes.built`, `capture.ready`; cinco passos `[ok]`, `tentativa = None`, sem quebra; conclui pela ausência de tentativa.
  - `destino_ausente_e_sem_dado`: `--destino` ausente; `inventario_destino` é `[sem dado]` com motivo, não `[falhou]`.
  - `captura_falhou`: origem com `capture.start` e `capture.failed{portal_denied}`, sem `capture.ready`; o passo `captura` é `[falhou]` com o motivo.
  - `--json` devolve o objeto do contrato, com os mesmos estados.
- [ ] **2. Rodar e ver falhar.** `cargo test -p clipsync-cli --locked --test verificar_travessia`; esperado: módulo/comando inexistente.
- [ ] **3. Implementar.** Parser de JSONL com `v`, `ts`, `evento`; seleção da origem/destino; `--ultima`/`--sessao` (nesta fatia, só pré-requisitos e `tentativa`); cálculo dos cinco passos, com `captura` ok só com `capture.ready` de ao menos uma barreira, `falhou` com `capture.failed` e o motivo, e `sem dado` sem os dois; `Saida::texto` e `Saida::json`; `main` despacha e mapeia `FalhaVerificar` para `Fail::refused` (código 3), quebra para `Fail::error` (código 1). Nesta fatia, `--desde` pode ser aceito e aplicado só por `ts` absoluto; a duração relativa fica para F3.
- [ ] **4. Rodar e ver passar.** `cargo test -p clipsync-cli --locked --test verificar_travessia`; `cargo clippy -p clipsync-cli --all-targets --locked -- -D warnings`.
- [ ] **5. Commit.** `adiciona esqueleto do verificador de travessia`.

## F2a · Fatia 2a: origem

### Tarefa 5: tentativa, barreira e filtros em `capture.rs`

**Dono:** Vector · input. **Depende:** tarefa 3 (eventos e teste layer).

**Arquivos:** `crates/clipsync-input/src/capture.rs`, `msg.rs`; trecho de `module.rs` de `drive`/`run_loopback`; testes `capture_backpressure.rs`, `s11_log.rs`.

**Interfaces:** `attempt: u32` em `Loop`; `next_attempt() -> u32` (1..=`u32::MAX`, sem 0); `OpenError` com `code()`; `send_error_code(&SendError) -> &'static str`; eventos `attempt.begin`, `barrier.activated`, `attempt.filtered` com `origem` `barrier`/`switch`/`test`.

- [ ] **1. Escrever o RED.** Em `s11_log.rs` (harness de captura):
  - `switch_abre_tentativa`: `InputSwitch` gera `attempt.begin{origem:"switch"}`, `session.open{mensagem:"enter"}` e `session.sent`; `attempt` e `session` não são zero.
  - `barreira_filtrada_termina_tentativa`: `Activated` com trava/pausa gera `attempt.begin{origem:"barrier"}`, `barrier.activated` e `attempt.filtered{motivo:"locked"}` (ou `paused`), sem `session.open`.
  - `open_falho_registra_motivo`: host que devolve `SendError::Transport` em `open` gera `attempt.filtered{motivo:"transport"}`; o texto mostrado na interface é o mesmo de antes.
  - `teste_sem_ativacao_termina`: `InputLoopback` de 5 s sem `Activated` gera `attempt.begin{origem:"test"}` e `attempt.filtered{motivo:"no_activation"}`.
- [ ] **2. Rodar e ver falhar.** `cargo test -p clipsync-input --locked --test s11_log`; esperado: eventos ausentes.
- [ ] **3. Implementar.** Em `Loop`, `attempt: 0`; `next_attempt` incrementa e evita 0. Em `switch`, cria a tentativa, loga `attempt.begin{origem:"switch"}` e passa `attempt` a `open`. Em `CaptureEvent::Activated`, cria a tentativa, loga `attempt.begin{origem:"barrier"}` e `barrier.activated{attempt, barreira, peer, pos}` **antes** dos filtros; cada filtro loga `attempt.filtered{motivo}` e retorna. Trocar `open` para `Result<(), OpenError>` com `CleanupPending`, `InventoryChanged`, `PeerOffline`, `RouteMissing`; logar `attempt.filtered` no erro e mapear para o texto atual no chamador. Em `opened(Err)`, logar `attempt.filtered{motivo: send_error_code(&e)}` e manter o `Note::Block` atual. No `drive`/`run_loopback`, `attempt=1`, `attempt.begin{origem:"test"}` depois das checagens de pausa/par e antes de `cap.start`; no fim sem sessão, `attempt.filtered{motivo:"no_activation"}`; em `Activated`, `barrier.activated` e `session.open{mensagem:"enter"}`.
- [ ] **4. Rodar e ver passar.** `cargo test -p clipsync-input --locked --test s11_log --test capture_backpressure --test layout_capture`; `cargo clippy -p clipsync-input --all-targets --locked -- -D warnings`.
- [ ] **5. Commit.** `registra tentativa e barreira na origem`.

### Tarefa 6: ciclo da sessão, datagramas e retorno

**Dono:** Vector · input.

**Arquivos:** `crates/clipsync-input/src/capture.rs`, `route_session.rs`, `msg.rs`; testes `stop_capture.rs`, `capture_backpressure.rs`, `s11_log.rs`.

**Interfaces:** `CloseReason` (`return`, `pause`, `lock`, `stop`, `peer_down`, `switch`, `inventory`, `refused`, `error`); `Session { sent: u64, dropped: u64 }`; eventos `session.open`, `session.sent`, `session.active`, `session.refused`, `datagram.first`, `return.received`, `session.closed`.

- [ ] **1. Escrever o RED.** 
  - `sessao_abre_envia_e_fecha`: `InputSwitch` até o destino; `session.open`, `session.sent`/`session.active`, `datagram.first` no primeiro datagrama; retorno do cursor gera `return.received`; `session.closed` com `motivo:"return"` e `totais.enviados >= 1`.
  - `pausa_fecha_com_motivo`: pausa no meio da sessão gera `session.closed{motivo:"pause"}` uma única vez.
  - `recusa_do_destino_fecha`: o destino recusa; a origem loga `session.refused{motivo}` e depois `session.closed{motivo:"refused"}` ou só o `session.refused`; nenhuma sessão ativa fica sem evento de fim.
  - `erro_de_envio_conta_descartado`: um datagrama que falha não é contado em `enviados` e entra em `descartados`; o fechamento mostra o total.
- [ ] **2. Rodar e ver falhar.** `cargo test -p clipsync-input --locked --test s11_log --test stop_capture`; esperado: eventos ausentes.
- [ ] **3. Implementar.** Contadores `sent`/`dropped` na sessão; `session.open` em `open` (antes do envio) com `mensagem` e `rota`; `session.sent` quando o envio do `Enter` termina Ok; `session.active` quando a sessão é aceita localmente (após `opened` Ok), sem duplicar com `session.sent` no mesmo caminho; `datagram.first` no primeiro `send_datagram` Ok; `return.received` no ponto em que o `back`/`Returned` é reconhecido; `session.closed` em `close_session`/`end_session` com `motivo` e `totais`. Adicionar `motivo` aos caminhos existentes; proibido `"unknown"`. `session.refused` no tratamento de `Rejected`/`Refused` vindo do destino. Não logar `session.closed` para uma abertura que nunca chegou a `session.open`.
- [ ] **4. Rodar e ver passar.** `cargo test -p clipsync-input --locked --test s11_log --test stop_capture --test capture_backpressure --test layout_capture`; `cargo clippy -p clipsync-input --all-targets --locked -- -D warnings`.
- [ ] **5. Commit.** `registra ciclo da sessão de entrada`.

## F2b · Fatia 2b: destino

### Tarefa 7: `enter.received`, `enter.refused` e a recusa de monitor ausente

**Dono:** Vector · input.

**Arquivos:** `crates/clipsync-input/src/module/organization.rs`, `module.rs`; testes `s10_routes.rs`, `s11_log.rs`.

**Interfaces:** `refuse_on_with(..., motivo)`; `enter.received` já vem da tarefa 3; o destino v4 recusa com `RejectReason::ApplyFailed` e loga `enter.refused{motivo:"monitor_not_found"}`.

- [ ] **1. Escrever o RED.** Em `s10_routes.rs`:
  - `monitor_ausente_recusa_apply_failed`: origem envia `EnterRoutes` com um UID de monitor que o destino não tem; o destino não chama `emu.enter`, envia `Rejected(ApplyFailed)` e loga `enter.refused{session, peer, motivo:"monitor_not_found"}`.
  - `origem_ve_a_recusa`: a origem recebe `Rejected` e fecha a sessão; não fica `active`; o log da origem tem `session.refused`/`session.closed` coerente.
  - `monitor_presente_nao_muda`: o caminho feliz continua emulando; sem `enter.refused`.
- [ ] **2. Rodar e ver falhar.** `cargo test -p clipsync-input --locked --test s10_routes`; esperado: o destino não recusa (hoje só retorna) e o evento não existe.
- [ ] **3. Implementar.** Em `enter_routes`, trocar o `let Some(m) = ... else { return; }` por `enter.refused{monitor_not_found}` seguido de `refuse_on_with(from, session, context.connection, RejectReason::ApplyFailed, "monitor_not_found")`. Manter o `return` depois do envio. Não mudar o protocolo nem o `PROTOCOL_VERSION`; o fio continua `ApplyFailed`. No `on_enter` legado, manter o fallback atual; só logar as recusas existentes via `refuse_on`.
- [ ] **4. Rodar e ver passar.** `cargo test -p clipsync-input --locked --test s10_routes --test s11_log`; `cargo clippy -p clipsync-input --all-targets --locked -- -D warnings`.
- [ ] **5. Commit.** `recusa monitor ausente no destino com prova no log`.

### Tarefa 8: emulação, datagrama do destino, retorno e infraestrutura

**Dono:** Vector · input.

**Arquivos:** `crates/clipsync-input/src/module.rs` (`on_enter`, `on_datagram`, emulação, `Module::run`), `module/organization.rs` (emulação v4), `adapter/portal.rs`, `adapter/win/*`; testes `module_mem.rs`, `s11_log.rs`, `wincore.rs` (o que der sem Windows real).

**Interfaces:** `emulate.start`, `emulate.positioned`, `emulate.error`, `datagram.first` (destino), `return.sent` com totais, `module.input.start/stop`, `portal.session.created`, `portal.connect_eis`, `hooks.installed/dropped`.

- [ ] **1. Escrever o RED.**
  - `emulacao_registra_inicio_e_posicao`: `Enter` válido gera `enter.received`, `emulate.start` e `emulate.positioned` com `session`; `emu.enter` com erro gera `emulate.error` e a recusa/limpeza atual.
  - `destino_conta_primeiro_datagrama_e_retorno`: primeiro datagrama gera `datagram.first{session}`; o retorno gera `return.sent{session, peer, pos, totais}` com `recebidos`/`ignorados`/`perdidos`/`fora_de_ordem`.
  - `portal_e_hooks_sem_tecla`: `portal.session.created`/`portal.connect_eis` com `motivo` quando falham; `hooks.installed/dropped` com `contagem`, sem keycode. Testar o que for possível com fake; o resto é CI Windows.
  - `module_input_start_stop`: `module.input.start`/`stop` com `contagem` de sessões/rotas.
- [ ] **2. Rodar e ver falhar.** `cargo test -p clipsync-input --locked --test s11_log --test module_mem`; esperado: eventos ausentes.
- [ ] **3. Implementar.** Logar `emulate.start` antes de `emu.start`, `emulate.positioned` depois de `emu.enter` Ok, `emulate.error` no `Err` com texto curto e sem conteúdo; no v4 (`organization.rs`) e no legado (`module.rs`). No `on_datagram`, após `inc.received +=`, se for o primeiro da sessão, `datagram.first`; no envio de `Returned`/`ReturnedRoute`, `return.sent` com `pos` e os totais de `Incoming`. Em `adapter/portal.rs`, `portal.session.created` na criação e `portal.connect_eis` no resultado, com `motivo` (`already_connected`, `denied`, `failed`); em `adapter/win`, `hooks.installed`/`hooks.dropped` com `contagem`, sem tecla. Em `Module::run`/`stop`, `module.input.start`/`stop`.
- [ ] **4. Rodar e ver passar.** `cargo test -p clipsync-input --locked --test s11_log --test module_mem --test layout_capture`; `cargo clippy -p clipsync-input --all-targets --locked -- -D warnings`; cross-clippy do Windows conforme o AGENTS.
- [ ] **5. Commit.** `registra emulação retorno e portal no destino`.

## F3 · Fatia 3: verificador completo

### Tarefa 9: cadeia, `--ultima`, `--sessao`, `--desde`, `--json` e esquema

**Dono:** Rivet · slice. **Depende:** tarefa 4; pode andar em paralelo com F2.

**Arquivos:** `crates/clipsync-cli/src/verificar.rs`, `main.rs`, `Cargo.toml`; `crates/clipsync-cli/tests/verificar_travessia.rs`.

**Interfaces:** as de `Entrada`/`Saida`; os onze passos; `TipoFalha::{Arquivo, Esquema}`; `--desde` absoluto e duração relativa ao maior `ts` da origem.

- [ ] **1. Escrever o RED.** Em `verificar_travessia.rs`, um helper `escrever(dir, linhas)` e casos:
  - `cadeia_feliz`: inventário dos dois lados, rotas, captura, tentativa, sessão, entrada, emulação, datagrama, retorno e fechamento; onze `[ok]`, sem quebra, código 0.
  - `quebra_no_primeiro_ponto`: para cada passo, remover/alterar o evento e afirmar que a quebra é aquele passo, não o seguinte; `[sem dado]` depois da quebra não é segunda quebra; em `captura`, `capture.failed{portal_denied}` é falha com motivo.
  - `tentativa_sem_ativacao`: `attempt.begin` de `test` e `attempt.filtered{no_activation}`; a cadeia para em `tentativa` com o motivo.
  - `ultima_e_sessao`: dois `attempt.begin`; `--ultima` escolhe o maior `ts`; `--sessao` escolhe a sessão e ignora a outra.
  - `desde_absoluto_e_duracao`: `--desde 2026-11-04T14:00:00Z` filtra; `--desde 10m` corta a partir do maior `ts` da origem, não do relógio; o mesmo corte vale para o destino.
  - `esquema_futuro_para`: `v:2` devolve `FalhaVerificar::Esquema` (código 3); `v:0` avisa e segue.
  - `arquivo_ausente_rotacionado`: origem ausente → código 3; destino ausente → `[sem dado]`; `clipsync.jsonl.1` com a sessão → o verificador lê `.1` quando o atual não tem a sessão (ou declara `[sem dado]` com motivo).
  - `json_igual_ao_texto`: o JSON tem `v`, `tentativa`, `sessao`, `passos`, `quebra`; os estados batem com o texto.
- [ ] **2. Rodar e ver falhar.** `cargo test -p clipsync-cli --locked --test verificar_travessia`; esperado: casos novos falham.
- [ ] **3. Implementar.** Parser tolerante a eventos desconhecidos; filtro por sessão/tentativa/janela; montagem dos onze passos na ordem do contrato; primeiro `[falhou]` como quebra; `[sem dado]` com motivo; leitura de `.1`/`.2` quando necessário; `--json`; `--desde` com `humantime` e `time::OffsetDateTime`; `main` com os códigos 0/1/3.
- [ ] **4. Rodar e ver passar.** `cargo test -p clipsync-cli --locked --test verificar_travessia`; `cargo test -p clipsync-cli --locked`; `cargo clippy -p clipsync-cli --all-targets --locked -- -D warnings`.
- [ ] **5. Commit.** `completa o verificador de travessia`.

## F4 · Fatia 4: pacote de suporte e tela

### Tarefa 10: cauda do log no pacote e caminho no Diagnóstico

**Dono:** Pixel · interface. **Depende:** F2 (para a cauda real).

**Arquivos:** `crates/clipsync-ui/src/diag.rs`; `crates/clipsync-app/ui/js/telas/diagnostico.js`; `crates/clipsync-app/ui-dev/mock/*`; criar `crates/clipsync-app/ui-dev/testes/diagnostico-log.test.mjs`.

**Interfaces:** `Diagnostics.log_path: String` com `#[serde(default)]`; `fn log_tail(profile_dir: &Path, max: usize) -> Vec<String>`; cauda de 400 linhas no `raw_text` antes de `redact`; a tela mostra o caminho em `.mono`, com quebra, sem `innerHTML`.

- [ ] **1. Escrever o RED.**
  - Rust em `diag.rs`: perfil temporário com `logs/clipsync.jsonl` de 410 linhas; `log_tail` devolve as 400 últimas na ordem; `bundle_preview` contém a cauda e o texto redigido (id/nome/IP do `g` não aparecem); sem arquivo, o pacote não ganha seção vazia.
  - Node: `diagnostico-log.test.mjs` com o mock devolvendo `log_path`; a tela renderiza o caminho e não quebra quando o campo falta (mock antigo).
- [ ] **2. Rodar e ver falhar.** `cargo test -p clipsync-ui --locked diag`; `node --max-old-space-size=512 --test --test-concurrency=2 crates/clipsync-app/ui-dev/testes/diagnostico-log.test.mjs`.
- [ ] **3. Implementar.** `Diagnostics.log_path`; `log_tail` lendo `logging::log_path`, sem `unwrap`, com erro → vazio; em `bundle_preview`, `let mut raw = raw_text(&g); raw.push_str("\n[log]\n"); ...` antes de `redact`; `diagnostico.js` mostra `dados.log_path` (ou o caminho do `app_state`, se o payload não tiver) em um `dd` mono; o mock preenche o campo.
- [ ] **4. Rodar e ver passar.** `cargo test -p clipsync-ui --locked diag`; o teste Node; `node --test ...` completo; `cargo clippy -p clipsync-ui --all-targets --locked -- -D warnings`.
- [ ] **5. Commit.** `inclui a cauda do log no pacote de suporte`.

## F5 · Fatia 5: documento

### Tarefa 11: `docs/verificar-por-log.md` e fechamento do contrato

**Dono:** Maestro.

**Arquivos:** criar `docs/verificar-por-log.md`; revisar `docs/contrato.md` §17 (já escrito); pedir ao DOC o link em `docs/uso.md`/`README.md` (fora da posse).

**Conteúdo:** onde fica o arquivo no Linux e no Windows; como copiar os dois arquivos (e `.1`/`.2`); o comando com e sem `--destino`; `--ultima`, `--sessao`, `--desde`, `--json`; como ler `[ok]`/`[falhou]`/`[sem dado]`; o exemplo do sintoma atual (notebook sem inventário); as duas mudanças de comportamento; privacidade e redação; rotação e perda possível; códigos de saída; o que não é prova (fake, CI, simulador).

- [ ] **1. Escrever o documento** com um exemplo de saída real gerado pelos testes da F3 e outro do caso `no_remote_inventory`.
- [ ] **2. Conferir links e comandos.** `cargo run -p clipsync-cli -- --help` mostra `verificar-travessia` e `log caminho`; os caminhos do texto batem com `logging::log_path`.
- [ ] **3. Pedir ao DOC o link** em `docs/uso.md` e `README.md`, sem editar esses arquivos.
- [ ] **4. Commit.** `documenta a verificação de travessia por log`.

## Autorrevisão do plano

- **Cobertura:** escritor/rotação/permissões e caminho puro na tarefa 1; inicialização, status e CLI na 2; inventário/rotas/captura/entrada na 3; esqueleto do verificador na 4; tentativa/barreira na 5; sessão/datagrama/retorno na 6; recusa de monitor na 7; emulação/destino/portal na 8; cadeia completa na 9; pacote e tela na 10; documento na 11.
- **Consistência:** o esquema e o vocabulário vêm do contrato §17.2; as tarefas não inventam evento novo. `session` e `attempt` são os ids existentes, não campos de protocolo. O `log_path` é o mesmo em CLI, status e interface. A única mudança de comportamento é a recusa do monitor ausente e os erros de captura registrados.
- **Limites:** fake, cross-compilação e CI não provam o cursor físico; a prova é o par de arquivos do usuário. O app Tauri só compila no CI. O portal e os hooks do Windows só têm prova real nas máquinas do usuário/CI; os eventos não podem ser usados para dizer que a captura funcionou sem `capture.ready` e `barrier.activated`.
- **Riscos residuais:** rotação concorrente não existe por contrato (escritor único), mas o teste não cobre dois processos; o Windows pode recusar `rename` com o arquivo aberto, e o contrato manda continuar no arquivo atual; o `Layer` de teste usa `set_default` e exige `#[tokio::test]` current_thread; um log copiado sem `.1`/`.2` pode perder a sessão e o verificador diz `[sem dado]`, não inventa.
- **Não executado na escrita deste plano:** nenhum teste, build, runtime, portal, perfil ou commit. O plano foi escrito por leitura do código, do contrato §17 e do desenho aprovado.

## Decisões do Maestro durante a execução · 04/11/2026

- Equipe: os terminais em modelo gratuito (Vector, Keel, Bolt, Flux, Pixel) esgotaram a cota do OpenCode. O Maestro assumiu a tarefa 3 e as fatias F2a/F2b no `clipsync-input` (o RED da tarefa 3 escrito pelo Vector foi mantido) e as decisões de plano; Rivet segue F1 e F3; Argus e Sentinel revisam. Pareceres do Argus ficam em `docs/verdicts.md`.
- `portal.rs` (EventSink, ConnectToEIS único): reprovado pelo Argus com 3 Important; guardado no ramo `wip/portal-eis-link` (`505bcaa`), fora da v0.5.1. A árvore volta ao conteúdo da `main` quando a T2 fechar.
- Tarefa 3: `capture.failed{adaptador, motivo}` em vez de um segundo `capture.start`; `routes.*` emitidos em `build_targets` com deduplicação por resultado (`State.routes_logged`), cobrindo o caminho legado e o organizado; `routes.empty.motivo` ganhou `unsupported_topology` (contrato §17.2 ajustado). `enter.refused` sai de `refuse_on` com o nome da variante em snake_case; `refuse_on_with` só entra na tarefa 7, para não ficar código morto.
- Teste `entrada_registra_recebida_e_recusada`: a pausa do módulo é autoridade do host (`ModuleInput::Paused` sozinho é reconciliado com o host), então o caso pausa pelo `MemHub` e injeta a entrada com a visão do destino sobre a conexão (`b.host.peer_input_context(a)`), não com a da origem.
- Tarefa 4: tentativa ausente é `[sem dado]` com motivo e código 0, nunca `[ok]`; `inventario_destino` é `[sem dado]` sem arquivo ou evento do destino, `[falhou]` quando a origem registrou `inventory.missing` para o par ou quando o destino tem `inventory.prepared` sem `inventory.received` na origem.
- Tarefa 2: absorveu os achados de Sentinel (symlink, permissões em objetos existentes, teto de 64 KiB com `log.truncated`, contador único com teto 200) e de Argus (nunca desvincular o arquivo em uso, `evento=legado`, piso `info`), com RED antes.
- Tarefas 5 e 6 (commit `89aba99`, um commit só porque ambas vivem em `capture.rs`): `open_falho_registra_motivo` virou `barreira_desconhecida_e_filtrada`, porque o `MemHub` não produz `SendError` em `open()` (par sempre conectado); `erro_de_envio_conta_descartado` não é reproduzível com os falsos (`send_datagram` nunca falha) e ficou coberto pelos totais de `session.closed`. Quando o destino recusa, a origem registra `session.active` antes de `session.refused`, porque o envio em si tem êxito e a recusa chega depois; o log preserva essa ordem. A pausa pela autoridade do host fecha com `motivo=pause` (ou `stop`), e o loopback registra `attempt.begin{origem:test}`, `barrier.activated`, `session.*` e `attempt.filtered{no_activation}` quando o prazo vence sem ativação.
- Tarefas 7 e 8 (commit `239b715`, um commit só): `return.sent.totais` leva `recebidos`, `perdidos` e `fora_de_ordem`; `ignorados` ficou de fora porque o módulo só conta ignorados por módulo, não por sessão (o valor por módulo continua no `status`). Os eventos `hooks.installed`/`hooks.dropped` do Windows não entraram: o adaptador só compila no Windows e não há prova local; ficam para a rodada com CI Windows. Os eventos do portal usam o nome do contrato com `motivo=refused` e `erro` quando a chamada falha. O silêncio em `enter_routes` quando o inventário ou a autorização mudam no meio da entrada (sem recusa no fio) foi mantido como está; só o monitor ausente passou a recusar.
- Revisão do Argus nas tarefas 3, 5 e 6 (commit `188b10d`): os sete achados tratados (T3-ARG-01/02, T56-ARG-01 a 07). Contador de tentativas do processo (`capture::next_attempt`), motivo de fechamento consumido por fechamento, `datagram.first` no primeiro envio com êxito, `capture.ready` com o trecho instalado, loopback com um único `session.closed` por sinal e totais reais, `pos` com três casas.
- F4 (tarefa 10) passou de Pixel para Rivet, por falta de cota do Pixel.
- Revisão do Argus nas tarefas 7 e 8 (commit `b7b70aa`): seis achados tratados (T78-ARG-01 a 06). `return.sent` só após o envio com êxito e `return.failed` nos demais casos; `enter.refused` com `enviada`/`erro`; `emulate.start` por entrada com `reutilizado`; `emulate.error` após posicionar; portal RemoteDesktop com ramo de falha; `perdidos` como estimativa que a duplicata exata não zera (contrato §17.2).
- Tarefa 10 (commit `05c8f7e`, Rivet): cauda de 400 linhas antes da redação, prévia preservada na exportação, caminho do log na tela Diagnóstico; UI 180/180, Node 273/273; portal 1280 e 960 sem vazamento com caminho de 274 caracteres. Em revisão por Argus e Sentinel.
- Re-revisão do Argus (commit `4c705d6`): T356B-ARG-01/02/03 (Deactivated como stop, trava como lock via `LoopSignal::Locked`, erro terminal antes da trava) e T78B-ARG-01 (guarda de cancelamento `Pendente`: envio cancelado pelo prazo de rede registra `return.failed`/`enter.refused` com `erro=timeout`). T78B-ARG-02 (verificador consumir `return.failed`) está com o Rivet na T9-bis.
- T9-bis (commit `f853383`, Rivet): correlação do destino por par, sessão e execução (`agent.up.id`); janela da tentativa separada da vida da sessão; `return.failed` como quebra em `retorno`; sem `agent.up` os passos remotos ficam SemDado/parada. Em revisão pelo Argus.
- T10-bis (Rivet, em curso): redação estrutural da cauda por chaves conhecidas, identidades históricas sempre redigidas (inclusive `agent.up.id`), varredor próprio de IP e caminho sem `regex`, `symlink_metadata` com a corrida local aceita, cauda de 256 KiB / 400 linhas / 16 KiB por linha, seção final de 256 KiB, `spawn_blocking` com uma leitura por vez (contrato §17.2 item 4).
- T10-bis (commit `99044ac`, Rivet): entregue conforme §17.2 item 4; diag 19/19, UI 185/185. Em re-revisão por Sentinel e Argus.
- T9-ter (commit `d4cd0bb`, Rivet): identidade também no inventário remoto; inferência de sessão não herda fechamento de outra tentativa. Em revisão pelo Argus. Falta só o `--help` em português.
- `--help` do verificador em português (commit `c442380`, Rivet). Todas as tarefas do plano entregues em 04/11/2026; restam as re-revisões de Argus e Sentinel e o teste físico nas máquinas do usuário.
- Re-revisão do Argus (commit `7ae747a`): BIS2-ARG-01 (erro terminal antes da trava também no loopback) e BIS2-ARG-02 (guarda de cancelamento criada antes da admissão; fila cheia conferida com `has_capacity` antes de construir a tarefa).
- T10-ter (commit `5e27d84`, Rivet): cauda sem tokenização; texto livre sempre omitido; allowlist literal, `ts` estrito, uids hex, ids curtos redigidos (contrato §17.2 item 4, `152d2dd`). Em re-revisão por Sentinel e Argus.
- Sentinel PASSA a F4 em `5e27d84` (recorte de segurança; SEG-01 encerrado, aceites anteriores mantidos). Falta o parecer do Argus sobre a fatia 4 e as re-revisões finais.
- Re-revisão do Argus (commit `e0e549d`): BIS3-ARG-01, a rechecagem pós-evento do loopback também classifica o fim (stop, pause, error, lock).
- Argus PASSA bis-4 (`e0e549d`) e a fatia 4 (`5e27d84`) com um Minor, T10T-ARG-01: a lista de motivos da cauda do suporte omitia `stopped`, `emulator_start_failed`, `entry_positioning_failed`, `route_invalid` e `inventory_unconfirmed`. Corrigido pelo Maestro no `diag.rs` com teste (Rivet ocioso; mudança de uma lista). `erro=network_busy|timeout` fica omitido de propósito: o campo `erro` é texto livre no contrato.
- Push 4f975ac quebrou CI e build: `clipsync-app` chamava `clipsync_core::logging::init()` (T2) sem depender do core, e a checagem local (`cargo clippy --workspace`) não compila o app por estar fora de `default-members`. Corrigido em `7e5b1e7`: `clipsync-ui` reexporta `logging` e o app chama `clipsync_ui::logging::init()`. Regra nova: antes de qualquer push, `cargo clippy -p clipsync-app --all-targets --locked -- -D warnings` (os headers do WebKitGTK existem nesta máquina; leva uns 2 min).
- CI do `a2241ae` verde na reexecução. A falha anterior foi `a_server_without_the_peers_key_never_records_a_refusal` (`crates/clipsync-core/tests/mesh.rs:1157`), não tocado nesta rodada: espera 4 s fixos pela discagem e falhou no runner Windows carregado. Intermitente; trocar o sleep fixo por espera com prazo em rodada futura.
- Campo (04/11/2026, PC Windows origem -> notebook Linux destino, build `7e5b1e7`): o sintoma original era `attempt.filtered{peer_offline}` (notebook desconectado); depois do repareamento, seis sessões completas com `return`. Os logs reais expuseram dois defeitos do verificador: fim da janela do destino preso ao `session.closed` da origem (relógio do notebook 1,6 s adiantado) e `inventario_destino` cobrado em travessia legada (`enter_span`). T11 (Rivet, `b968bfd`): tolerância de 30 s, prova de um lado basta para retorno/datagramas, N/A legado. Contrato §17.3 e guia ajustados pelo Maestro. Teclado na primeira sessão continua aberto (pendências).
