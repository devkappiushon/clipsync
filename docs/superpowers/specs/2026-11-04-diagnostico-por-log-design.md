# Diagnóstico verificável por log · desenho · 04/11/2026

Rascunho do Maestro · scaffold para aprovação do usuário. Itens marcados **(meu palpite)** são escolhas minhas, não do usuário; o resto vem do pedido dele e do levantamento do código em 04/11/2026.

## 1. Intenção e porquê

O usuário pediu: "faça o ClipSync ser totalmente verificável via log pela IA; ainda estou com o problema do cursor não passar para o notebook". Hoje a IA não consegue verificar nada: os logs vão só para o stderr (no Windows o app de release os descarta), não há arquivo, o caminho da travessia quase não registra nada, e há três pontos em que a passagem falha em silêncio. O id de sessão existe em toda mensagem de entrada, mas nunca aparece em log.

Sucesso: depois de uma tentativa de passagem com o mouse físico, o usuário copia um arquivo de cada máquina e roda um comando que diz, passo a passo, até onde a cadeia chegou e qual foi o primeiro ponto que falhou, com a linha de log como prova. A IA lê a mesma saída e chega à mesma conclusão sem adivinhar.

## 2. Metas, não metas e anti-metas

Metas:

1. Arquivo de log por máquina, em formato de linhas JSON, sempre ligado, com rotação por tamanho, que o usuário encontra sem procurar.
2. Cada passo da travessia registrado com um vocabulário fixo de eventos e com o mesmo id nas duas máquinas, inclusive os três pontos que hoje falham em silêncio.
3. Um comando da CLI que lê os logs da origem e do destino e imprime a cadeia conferida, em português, com veredito por passo e o primeiro ponto de quebra; também em JSON para a IA.
4. O pacote de suporte do app passa a incluir a cauda do log, já redigida, para o lado Windows não depender de terminal.
5. Documento curto que diz como coletar nos dois sistemas e como ler, com a lista dos eventos.

Não metas (nesta rodada): buscar o log do par pela rede; mudar o protocolo da malha; trocar a lógica do estado de travessia da interface; registrar métricas contínuas de latência.

Anti-metas: qualquer conteúdo de clipboard, tecla ou keycode em log (contrato §2.2 continua vinculante); log que cresce sem limite; dependência nova sem revisão de segurança.

## 3. Restrições

- Contrato §2.2: `tracing` para stderr com `CLIPSYNC_LOG` continua como está. O arquivo é uma camada a mais, não uma troca.
- Sem conteúdo, preview, keycode ou tecla; ids de par sempre curtos; nomes de host só os já expostos pelo `status`.
- Sem crate novo no workspace **(meu palpite)**: a camada de arquivo é um `Layer` próprio do `tracing-subscriber` (crate já pinado; `clipsync-core` passa a depender dele, aresta nova que Sentinel revisa), serializando com o `serde_json` já presente. Escritor com `Mutex`, descarga a cada linha (o volume é de eventos, não de datagramas), modo 0600 no Linux. Evita `tracing-appender` e o recurso `json` do `tracing-subscriber`, que puxaria `tracing-serde`.
- Sem mudança de protocolo: a correlação entre máquinas usa o `session` (`u32`) que já viaja nas mensagens de sessão (`Enter*`, `Leave`, `Returned*`, `Refused`) e em todo datagrama. Zones, LayoutOffer, InventoryV4 e LayoutOfferV4 não têm sessão: os eventos de inventário correlacionam por par e horário. Tentativas barradas antes de a sessão nascer recebem um `attempt` local e aparecem só no log da origem, o que por si só é uma conclusão ("a origem barrou antes de enviar").
- Rust com os lints do workspace (sem `unwrap`, sem `unsafe`); testes com os adaptadores falsos existentes.

## 4. O que foi decidido

### 4.1 Arquivo de log

- Caminho: `<perfil>/logs/clipsync.jsonl` (Linux: perfil atual em `~/.config/clipsync`, Windows: `%LOCALAPPDATA%\clipsync`). Rotação por tamanho **(meu palpite)**: 5 MB, mantendo `clipsync.jsonl.1` e `.2`.
- Cada linha abre com `"v":1` (versão do esquema). Uma linha por evento: `{"ts":"2026-11-04T14:03:21.417Z","nivel":"info","host":"notebook-a","alvo":"clipsync_input::capture","evento":"session.open","session":2890173,"peer":"aa31-537a",...}`. Campos extras vêm dos `fields` do `tracing`.
- Nível do arquivo: `info` para os crates `clipsync_*`, `warn` para o resto, independente de `CLIPSYNC_LOG`. `CLIPSYNC_LOG_ARQUIVO` sobrescreve o filtro **(meu palpite)**.
- Inicialização num módulo único de `clipsync-core` (`logging.rs`): a camada de stderr sobe como hoje, antes de resolver o perfil; a camada de arquivo é instalada junto, mas só começa a escrever quando `logging::open_file(profile_dir, device_name)` é chamada depois de `--profile` resolvido e do `config.toml` lido (eventos anteriores são descartados). Só o processo do agente escreve o arquivo (app com agente no processo, ou `clipsync run`); os demais comandos da CLI ficam só no stderr. Como o perfil tem trava de agente único, há um escritor por arquivo. O caminho vem de uma função pura `logging::log_path(profile_dir)`, usada pela interface mesmo com o agente parado ou falho, e também entra em `AgentStatus.log_path` e no `AppState`.

### 4.2 Vocabulário de eventos da travessia

Nomes fixos no campo `evento`, um por passo, todos com `peer` (id curto) e, quando existir, `session`. Origem: `capture.start`, `capture.ready` (lista das barreiras instaladas: par, monitor, borda, trecho), `barrier.activated` (barreira, posição, `attempt`), `attempt.filtered` (motivo em `snake_case` do nome da variante, registrado no ponto em que o motivo ainda é tipo e não `String`: paused, locked, screen_locked, unknown_barrier, cleanup_pending, inventory_changed, peer_offline e as variantes de `SendError`), `session.open` (tipo da mensagem: enter_routes, enter_span, enter_at, enter), `session.sent`, `session.refused` (motivo do par), `session.active`, `datagram.first` (primeiro datagrama enviado), `return.received`, `session.closed` (motivo e totais: enviados, descartados). Destino: `enter.received`, `enter.refused` (motivo, inclusive monitor não encontrado, que hoje some), `emulate.start`, `emulate.positioned`, `emulate.error`, `datagram.first` (primeiro recebido), `return.sent` (com totais: recebidos, ignorados, perdidos, fora de ordem). Inventário (antes de tudo, nas duas máquinas): `inventory.prepared` (monitores locais, contagem e uids), `inventory.sent` (para qual par), `inventory.received` (de qual par, contagem), `inventory.missing` (par sem monitores conhecidos: a origem não monta rotas), `routes.built` (rotas e barreiras derivadas, contagem) e `routes.empty` (motivo). Infraestrutura: `agent.up`, `peer.connected`, `peer.disconnected`, `module.input.start`, `module.input.stop`, `portal.session.created`, `portal.connect_eis`, `hooks.installed`, `hooks.dropped`.

Os três silêncios de hoje viram eventos: erro descartado ao abrir a sessão (`capture.rs`, três motivos), monitor não achado no destino (`organization.rs`, passa a recusar com motivo) e ausência de sinal do portal antes de `Activated` (`capture.ready` com as barreiras instaladas, para provar que o compositor as aceitou).

### 4.3 Comando de verificação

`clipsync verificar-travessia --origem A.jsonl --destino B.jsonl [--sessao N | --ultima]`. Sem `--sessao`, usa a última tentativa da origem. Saída em português, uma linha por passo: `[ok]`, `[falhou]` ou `[sem dado]`, com a linha de log resumida ao lado, e ao fim "primeiro ponto de quebra: ...". `--json` devolve a mesma cadeia como objeto. Só lê arquivos; não precisa do agente. Teste com logs sintéticos para cada ponto de quebra. Também `clipsync log caminho` (imprime o caminho do arquivo) **(meu palpite)**.

### 4.4 Coleta sem terminal

O pacote de suporte ("Exportar pacote de suporte" na tela Diagnóstico) passa a incluir as últimas 400 linhas do log, após a mesma redação de ids e nomes **(meu palpite sobre o número)**. A tela Diagnóstico mostra o caminho do arquivo de log.

### 4.5 Documento

`docs/verificar-por-log.md`: onde fica o arquivo em cada sistema, como coletar (pacote de suporte no Windows, caminho no Linux), o comando, a leitura da saída e a tabela dos eventos. Uma seção "para a IA" com o procedimento em três passos.

### 4.6 Esquema por evento (v1)

Campos comuns em toda linha: `v` (1), `ts` (UTC, RFC 3339 com milissegundos), `nivel`, `host` (nome deste dispositivo, quando o arquivo já estava aberto), `alvo`, `evento`. Campos por evento, todos opcionais quando não se aplicam:

| Evento | Campos |
|---|---|
| `inventory.prepared` | `monitores` (contagem), `uids` (lista) |
| `inventory.sent` / `inventory.received` | `peer`, `monitores`, `revisao` |
| `inventory.missing` | `peer` |
| `routes.built` | `rotas`, `barreiras` (contagem), `pares` (lista) |
| `routes.empty` | `motivo` |
| `capture.start` / `capture.ready` | `adaptador`; em `ready`, `barreiras`: lista de `{peer, monitor, borda, trecho}` |
| `attempt.begin` | `attempt`, `origem` (`test` quando vem de Testar passagem; `barrier` quando vem de `barrier.activated`) |
| `barrier.activated` | `attempt`, `barreira` (índice), `peer`, `pos` (x, y inteiros) |
| `attempt.filtered` | `attempt`, `motivo` |
| `session.open` | `attempt`, `session`, `peer`, `mensagem` (`enter_routes`, `enter_span`, `enter_at`, `enter`), `rota` |
| `session.sent` / `session.active` | `session`, `peer` |
| `session.refused` | `session`, `peer`, `motivo` (o do par) |
| `datagram.first` | `session` |
| `return.received` / `return.sent` | `session`, `peer`, `pos`; em `sent`, totais |
| `session.closed` | `session`, `motivo`, totais |
| `enter.received` | `session`, `peer`, `mensagem` |
| `enter.refused` | `session`, `peer`, `motivo` (fino, inclusive `monitor_not_found`; no fio continua `ApplyFailed`) |
| `emulate.start` / `emulate.positioned` / `emulate.error` | `session`, `adaptador`, `pos`, `erro` (texto curto, sem conteúdo) |
| `agent.up`, `peer.connected`, `peer.disconnected`, `module.input.start`, `module.input.stop`, `portal.session.created`, `portal.connect_eis`, `hooks.installed`, `hooks.dropped` | `peer`, `nome`, `motivo`, `contagem`, conforme o caso |

Ordem canônica da cadeia, para o verificador: inventário (as duas máquinas) → rotas → captura pronta → tentativa → sessão aberta e enviada → recebida no destino → emulação posicionada → primeiro datagrama nos dois lados → retorno → sessão fechada.

### 4.7 O que é uma tentativa

Uma tentativa começa num `attempt.begin` (Testar passagem na interface) ou num `barrier.activated` (cursor físico) e termina em `attempt.filtered`, `session.refused` ou `session.closed`. O `attempt` é um contador local da origem. `--ultima` escolhe a tentativa de maior `ts` na origem. `--desde <horário>` limita a janela nos dois arquivos. Sem tentativa na janela, o verificador não inventa: imprime o estado dos pré-requisitos (inventário dos dois lados, rotas, captura pronta) e conclui "nenhuma tentativa registrada: a origem nunca viu o cursor na barreira", que é exatamente o sintoma atual do usuário. Destino ausente (`--destino` não informado) ou arquivo rotacionado sem a sessão: os passos do destino saem como `[sem dado]`, com o motivo. Versão do esquema maior que a conhecida: o verificador para e avisa; menor: avisa e segue com os eventos que conhece.

### 4.8 Silêncios que mudam de comportamento

- `capture.rs`: os erros hoje descartados em `let _ = self.open(..)` e em `opened(Err)` passam a registrar `attempt.filtered` ou `session.closed` com o motivo, sem mudar o que a interface mostra.
- `organization.rs`: monitor não encontrado no destino passa a recusar com `ApplyFailed` no fio e `enter.refused{motivo: monitor_not_found}` no log. A origem vê a recusa (hoje acredita que a sessão abriu). Mudança de comportamento registrada no contrato.

## 5. Fatias e posse (proposta para o contrato §17)

Ordem decidida: a fatia 1 é vertical e curta, para provar o `Layer` próprio e a correlação num caso real nas máquinas do usuário antes de espalhar eventos por `capture.rs`, `module.rs` e `organization.rs`.

1. Vertical mínimo: `logging.rs` (escritor, rotação, 0600, `log_path`, `open_file`), inicialização na CLI e no app, `AgentStatus.log_path`, `AppState.log_path`; os eventos `inventory.*`, `routes.*`, `capture.ready` (com barreiras) e `enter.received|refused` reais; esqueleto de `verificar-travessia` que lê esses eventos e imprime os pré-requisitos. Posse: `clipsync-core/src/logging.rs` (novo) e `clipsync-core/src/msg.rs`, `clipsync-proto/src/control.rs`, `clipsync-cli/src/main.rs` e `clipsync-cli/src/verificar.rs` (novo), `clipsync-app/src/main.rs` e `clipsync-app/src/state.rs`, `clipsync-ui/src/status.rs`; em `clipsync-input`, só os pontos de inventário, rotas, `capture.ready` e `on_enter`, combinados com o dono da fatia 2.
2a. Origem: `attempt.*`, `barrier.activated`, `session.*`, `datagram.first`, `return.received`, com os erros de `open`/`opened` registrados por variante (`clipsync-input/src/capture.rs`, `route_session.rs`, `msg.rs`). 2b. Destino: `emulate.*`, `datagram.first`, `return.sent`, `enter.refused{monitor_not_found}` (`module.rs`, `module/organization.rs`, `adapter/portal.rs`, `adapter/win/*`). Testes com falsos capturam os eventos com um `Layer` de teste.
3. `verificar-travessia` completo (cadeia, `--ultima`, `--desde`, `--json`, esquema) e `clipsync log caminho`, com testes de logs sintéticos por ponto de quebra.
4. Pacote de suporte com a cauda do log (400 linhas, redigidas) e caminho na tela Diagnóstico. Posse: `clipsync-ui/src/diag.rs`, `ui/js/telas/diagnostico.js`, mock e testes Node.
5. `docs/verificar-por-log.md` e contrato §17. Posse: Maestro.

Revisão: Argus em cada fatia; Sentinel na fatia 1 (permissões do arquivo, redação) e na 4.

## 6. O que fica para o construtor

Como o `Layer` lida com `span` (não há spans hoje; pode ficar sem), a forma de capturar eventos nos testes (um `Layer` de teste que acumula em `Vec`), e o limite de 200 eventos descartados antes de `open_file` (contados em `agent.up`).

## 7. Respostas do usuário · 04/11/2026

1. Estado observado: o notebook (destino) mostra "sem monitores conhecidos"; o PC (origem) mostra o desenho das telas, mas o usuário não consegue mais mover nada. Leitura do Maestro: o inventário de monitores não está chegando ou não foi preparado no notebook, e sem ele a origem não monta rotas nem barreiras, então o cursor nunca sai. A troca de inventário entra no vocabulário de eventos (§4.2) como passo obrigatório da cadeia, antes da captura.
2. O notebook é só destino. A correção do `portal.rs` (captura no Linux) entra na v0.5.1 por já estar pronta e revisada, sem prioridade.
3. Rotação de 5 MB com duas cópias aprovada por omissão ("faça").

Desenho aprovado pelo usuário em 04/11/2026 ("faça"). Próximo: contrato §17 e fatias no workboard pelo Keel.
