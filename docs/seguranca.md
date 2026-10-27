# Segurança · auditoria da fatia S1 (Sentinel, 2026-10-27)

**Veredito atual (reverificação, 2026-10-27): LIBERADO PARA S1, sem BLOCKER aberto.** Os quatro achados que o time corrigiu (S1-01, S1-02, S1-08, S1-09) foram reverificados no checkout e passam; o ataque do S1-01 foi repetido com o Hello malicioso cru contra o `clipsync pair host` real e é recusado. Ficam abertos, por decisão registrada no contrato v1.10, cinco achados de severidade baixa (S1-03, S1-04, S1-05, S1-06, S1-07), que continuam reproduzíveis e não bloqueiam.

Veredito da primeira rodada (histórico): BLOQUEADO por S1-01.

Contagem da primeira rodada: 1 BLOCKER · 0 alta · 1 média · 7 baixas · 5 informativos.
Estado agora: **0 BLOCKER · 0 alta · 0 média · 5 baixas abertas · 6 informativos** (4 achados corrigidos e verificados).

## Reverificação (commit c1ed65d)

- **S1-01 · CORRIGIDO, verificado.** `clipsync_proto::validate_device_name` (`proto/src/wire.rs:201`) é a regra única: vazio, > 64 bytes, `is_control` (C0, DEL, C1 incluindo U+0085 e U+009B) e caracteres de formatação (U+061C, U+200B-200F, U+2028/2029, U+202A-202E, U+2060-2064, U+2066-2069, U+FEFF). Usada em `Hello::validate` e em `profile::valid_name`; teste `validate_device_name_rejects_control_and_bidi` passa. CLI: `text::clean` em `pair.rs:35/82/96`, `cmds.rs` (status, peers, inbox) e `text::delivery`; `Código:` é a última linha antes de `[s/N]`.
  - **Repetição da reprodução:** Hello com os 26 bytes do ataque, montado em bytes brutos (nome trocado direto no corpo postcard, sem passar pelo `validate()` do encoder), contra o `clipsync pair host` real. Resultado: o host recusa o frame, stdout só tem `Aguardando em ...`, sem ESC, sem `123 456`, sem prompt; stderr `erro: pareamento falhou: bad frame`; saída 3; `peers.toml` vazio.
  - **Pareamento legítimo continua funcionando.** Tela: `Par: notebook-legit (id ...)` / `Código: 240 594` / `O código acima é o mesmo no outro computador? [s/N]`; o código é a penúltima linha, a pergunta a última; par gravado.
  - **Teste de matriz:** aceitos ainda (invisíveis, não movem cursor nem reordenam texto): U+00AD, U+180E, U+206A-206F, U+FFF9-FFFB, U+FE0F, caracteres de tag U+E00xx. Ver I-6.
- **S1-02 · CORRIGIDO, verificado.** `pairing::save` chama `validate_device_name` antes de gravar (`pairing.rs:248-250`). `load_peers` põe a entrada de nome inválido em quarentena (ignora e loga só o id curto) e carrega o resto; reproduzido com um `peers.toml` com uma entrada boa e uma com `ESC[2J` no nome: carrega 1 par, e o pino ruim não entra em `pins()`. Observação: a entrada em quarentena some do arquivo no próximo `save_peers` (o arquivo é reescrito só com as boas). Aceitável; o log avisa.
- **S1-08 · CORRIGIDO, verificado.** `agent.rs:208-219` aplica `mesh.set_paused` primeiro e depois persiste; falha ao gravar devolve erro mas continua pausado. Teste `pause_fails_closed_when_state_cannot_be_saved` passa.
- **S1-09 · CORRIGIDO, verificado por leitura e teste, com uma ressalva.** `control.rs`: `bind_private` binda em nome temporário, `chmod 0600`, `hard_link` para `agent.sock` e apaga o temporário; `AlreadyExists` vira `AgentAlreadyRunning`. Socket velho só é removido em `NotFound`/`ConnectionRefused`; qualquer outro erro de `connect` aborta sem apagar. Testes `control_` (2) passam, incluindo modo 0600. **Ressalva:** a terceira recomendação, `agent::run`/`load_identity` recusarem diretório de perfil com modo diferente de 0700, não foi feita (só `init` aplica 0700). Com o bind atômico o risco caiu para quase nada; fica como melhoria.
- **S1-03, S1-04, S1-05, S1-06, S1-07 · ABERTOS por decisão (contrato v1.10, pós-MVP).** Reproduzidos de novo hoje contra o código novo: S1-03 (nome repetido apaga o par: sobrou 1 em 2), S1-04 (sonda com ALPN errado encerra o host na hora), S1-05 (`Debug` de `PairMsg` ainda imprime o nonce; `Key{code:u32::MAX}` e `Motion{f32::MAX}` ainda aceitos), S1-06 (Blocked continua depois de 8 s e de `reload`). S1-07 não reproduzido (só leitura), código inalterado. Nenhum depende dos outros para bloquear o release. O S1-04 tem agora o texto de contrato no v1.10 §5 regra 3; falta implementar.

Testes da reverificação (alvo próprio, checkout atual): `cargo test --locked --offline -p clipsync-proto -p clipsync-core -p clipsync-cli` verde: proto 41, core 36 (unit 14, control 2, diag 1, mesh 7, profile 12), cli 12 (unit 3, roteiro 8, smoke 1); 0 falhas. Provas adversariais: `r_s1_01_raw_malicious_hello_is_refused_by_the_real_host`, `r_s1_01_legit_pairing_shows_code_last_before_prompt`, `r_s1_02_poisoned_entry_is_quarantined_and_agent_starts` passam.

---

## Primeira rodada (texto original; os itens S1-01, S1-02, S1-08 e S1-09 estão corrigidos, ver acima)

Escopo lido por inteiro: `core/src/{tls,pairing,transport,agent}.rs`, `proto/src/{wire,limits,module,control}.rs`. Lidos também, porque o foco pedia: `core/src/{control,profile,lib,msg}.rs`, `proto/src/{clip,input,ids,redact}.rs`, `cli/src/{pair,prompt,msg,text}.rs` e o tratamento de `Ack`/`Stats`/`Leave` nos módulos.

Método: leitura linha a linha contra o contrato (§4, §5 regras 1 a 10, §9 S1) e o prompt mestre (§2.1, §8, §9); depois provas adversariais locais, fora do repositório (nenhum arquivo do código foi editado, nenhum git rodado). Evidência de cada "reproduzido" abaixo: saída real do teste adversarial (apêndice A).

## Achados

### S1-01 · BLOCKER · `device_name` do par aceita caracteres de controle e o CLI imprime cru: o código de pareamento pode ser forjado na tela

- **Onde:** `proto/src/wire.rs:197-206` (`validate_name` só checa vazio e 64 bytes); `cli/src/msg.rs:117-119` e `cli/src/pair.rs:34` (`format!("Código: {code}\nPar: {name} ...")` sem filtro). Contraste: `core/src/profile.rs:277-284` (`valid_name`) já recusa `char::is_control` para o nome LOCAL; o nome que vem do fio não passa por essa regra.
- **O que acontece:** quem fala com o host durante a janela escolhe o `device_name` do `Hello`. Um nome de 26 bytes (`ESC[1A`, `\r`, `ESC[2K`, `Código: 123 456`, `\n`) sobe uma linha, apaga e reescreve a linha `Código:`. O usuário vê um código que não é o da sessão dele.
- **Por que derrota o pareamento:** o MITM (B ↔ M ↔ A) conhece o código que B mostra (ele é o ponto final TLS da sessão com B, calcula o mesmo exporter). Ele conecta em A depois de ter esse código e manda o `Hello` com o nome acima. A vê o código de B na tela, confirma, B confirma: os dois gravam a chave do M como "par" e o usuário concede as permissões que o prompt perguntar (inclusive `input-in`, controle total do teclado). O compromisso H(nB)/nA/nB está correto (ver "Verificado"), mas protege só o código computado, não o que a tela mostra.
- **Reproduzido (CLI real, `clipsync pair host`):**
  - bytes impressos: `"Código: 761 467\nPar: \u{1b}[1A\r\u{1b}[2KCódigo: 123 456\n (id 817a-ab9c-d3b6-b138) · módulos: clipboard, input\nO código acima é o mesmo no outro computador? [s/N] "`
  - tela do usuário depois de aplicar CR, LF, CSI A e CSI K: `Código: 123 456` / ` (id 817a-ab9c-d3b6-b138) · módulos: clipboard, input` / `O código acima é o mesmo ... [s/N]`. O código real (761 467) sumiu da tela.
  - `encode_frame(Hello{device_name:"\x1b[2J\x07\u{202e}x"})` retorna `Ok`: o próprio `validate()` deixa passar.
- **Recomendação:**
  1. Uma função só para nome de dispositivo, em `clipsync-proto` (`validate_name`), que recusa `char::is_control` (C0, DEL e C1, inclusive o CSI de 8 bits U+009B) e os caracteres de formatação Unicode (U+200B a U+200F, U+2028/2029, U+202A a U+202E, U+2066 a U+2069, U+FEFF). `profile::valid_name` passa a chamá-la, para as duas regras não divergirem.
  2. Defesa em profundidade no CLI: tudo que vem do par (nome, e depois motivos de bloqueio) passa por `escape_debug` ou filtro antes de imprimir. Imprimir `Código:` na última linha antes da pergunta, depois do nome.
  3. Testes de regressão: `validate_device_name_rejects_control_and_bidi` (proto) e um `pairing_` que faz o join com nome malicioso e espera `PairFailed`.

### S1-02 · MÉDIA · pareamento grava um nome que `load_peers` recusa: `peers.toml` envenenado, agente não sobe e nenhum novo pareamento funciona

- **Onde:** `core/src/pairing.rs:215-218` e `:244-254` (`save` grava `peer_hello.device_name` sem validar); `core/src/profile.rs:487-491` (`load_peers` recusa nome com controle).
- **Reproduzido:** depois do pareamento do S1-01, `load_peers()` → `Err("profile: peers.toml: nome inválido em 817a-ab9c-d3b6-b138")` e `agent::run(...)` → o mesmo erro. O `peers.toml` ficou com `name = """\n\u001B[1A\r\u001B[2KCódigo: 123 456\n"""`. Como `save` chama `load_peers` primeiro, todo pareamento seguinte também falha. Só edição manual do arquivo conserta.
- **Recomendação:** `save` valida o nome (a correção do S1-01 já fecha a entrada; manter a checagem aqui mesmo assim) e devolve `PairFailed` antes de gravar. `load_peers` deveria colocar a entrada ruim em quarentena e carregar o resto, em vez de derrubar o perfil inteiro.

### S1-03 · BAIXA · nome repetido apaga o par existente em silêncio e a busca por nome é ambígua

- **Onde:** `core/src/profile.rs:163-167` (`Peers::upsert` remove também quem tem o mesmo `name`); `core/src/pairing.rs:249-250` (`save` remove quem tem o mesmo `addr`); `profile.rs` `Peers::find` casa nome, id curto ou hex no mesmo `find`, na ordem do arquivo.
- **Reproduzido:** parear `b1` e depois `c1`, ambos chamados `notebook-b`: sobra 1 par em A (o `c1`); o `b1`, com as permissões que tinha, sumiu sem aviso.
- **Impacto:** quem consegue ser pareado com o nome de um par legítimo (usuário digitou o endereço errado, host malicioso) apaga a entrada real e herda tudo que referencia por nome (`send --to`, `probe input --to`, e a futura `[[input.barrier]] peer = "..."`). Hoje exige a confirmação do usuário nos dois lados, por isso BAIXA; sobe para MÉDIA quando a barreira por nome entrar (2B).
- **Recomendação:** o prompt mostra "substitui o par X (id ...)" e exige resposta explícita; nome igual com id diferente é recusado por padrão. `find` prefere id exato e devolve erro se o nome for ambíguo.

### S1-04 · BAIXA · uma sonda QUIC não autenticada encerra o `pair host` e queima a janela

- **Onde:** `core/src/pairing.rs:291-316`. O primeiro `Incoming` é tomado como "a tentativa"; erro de handshake ou timeout de 10 s sobe por `?` e o host termina.
- **Reproduzido:** um cliente com ALPN `clipsync/1` na porta do pareamento: o host terminou com `Err("pairing: the cryptographic handshake failed: error 120: peer doesn't support any known protocol")` imediatamente, apesar da janela de 60 s; o join legítimo seguinte falhou.
- **Impacto:** negação de serviço do pareamento por qualquer host da LAN que ache a porta UDP (efêmera, mas impressa na tela do host). Não afeta segurança do par. É consequência literal do contrato §5.3 ("qualquer falha fecha").
- **Recomendação:** mudança de contrato (Keel): só falha **autenticada** (primeiro `Commit` válido em diante) consome a janela; erro de handshake antes disso volta a `accept()` até o prazo. Alternativa mínima: `incoming.retry()` quando `!remote_address_validated()`.

### S1-05 · BAIXA · `validate()` deixa passar valores fora de qualquer faixa útil

- **Onde:** `proto/src/input.rs:101-137`, `proto/src/wire.rs:170-177`.
- **Reproduzido (aceitos por `encode_datagram`/`encode_frame`):** `Motion{dx:f32::MAX}`, `Key{code:u32::MAX}`, `ScrollDiscrete{dx:i32::MIN, dy:i32::MAX}`, `Hello{listen_port:0}`.
- **Impacto:** o prompt mestre §8.6 pede "validação de coordenadas". Quem já tem `input_in` tem controle total de qualquer jeito, então o ganho é robustez: valor absurdo chega ao emulador (libei/evdev) como entrada, e o limite de taxa conta datagramas, não eventos (2000 × 32 = 64 mil eventos/s).
- **Recomendação:** `|dx|,|dy|` ≤ um teto lógico (ex.: 16 384 px), `code` de tecla/botão ≤ `KEY_MAX` (0x2ff), `ScrollDiscrete` dentro de ±(120 × 64), `listen_port != 0`; contar eventos no limite de taxa (módulo input).

### S1-06 · BAIXA · bloqueio por "chave mudou" é pegajoso: o dialer termina e nem `reload` revive

- **Onde:** `core/src/transport.rs:500-508` (`return` depois de marcar `Blocked`), `:275-323` (`reload` não reinicia o dialer de um id que já existe).
- **Reproduzido:** um impostor com outra chave no endereço do par B → A marca `Blocked { reason: "chave do par mudou; repareie" }` em poucos segundos. O impostor sai, o B legítimo (chave pinada) passa a escutar no mesmo endereço, `reload` com a mesma lista: 8 s depois continua `Blocked`. Só volta se B discar A (o `install` inbound limpa o estado) ou se o agente reiniciar.
- **Impacto:** DHCP reaproveitando o IP, ou alguém na LAN respondendo no endereço do par, desliga a malha de A→B até reiniciar. Falha fechada (nada vaza), mas sem recuperação.
- **Recomendação:** manter o alerta e o estado, mas continuar tentando com backoff longo (5 min) e limpar `Blocked` quando a chave esperada reaparecer; ou fazer `reload` reiniciar o dialer.

### S1-07 · BAIXA · listener da malha sem validação de endereço nem teto de handshakes simultâneos (não reproduzido)

- **Onde:** `core/src/transport.rs:388-397` (`accept_loop` cria uma task por `Incoming`), `:399-404`. Padrão `0.0.0.0:47800`.
- **Impacto:** cada Initial com origem forjada custa um handshake TLS (assinatura ECDSA do servidor) antes de a chave do cliente ser checada. Só leitura; sem medição.
- **Recomendação:** `if !incoming.remote_address_validated() && incoming.may_retry() { incoming.retry() }` e um `Semaphore` para handshakes em andamento.

### S1-08 · BAIXA · pausa falha aberta se `state.toml` não grava

- **Onde:** `core/src/agent.rs:205-213`. Grava o estado primeiro e só então chama `mesh.set_paused`. Disco cheio ou somente leitura: devolve erro e o tráfego **continua**.
- **Recomendação:** aplicar a pausa na memória primeiro e depois persistir; se a gravação falhar, continua pausado e o erro avisa que não sobrevive a reinício.

### S1-09 · BAIXA · socket de controle: janela entre `bind` e `chmod`, e `remove_file` em qualquer erro de `connect` (leitura)

- **Onde:** `core/src/control.rs:105-111`. `UnixListener::bind` cria o socket com o modo da `umask`; `set_permissions(0o600)` vem depois. Com `umask 000` e diretório de perfil permissivo, há uma janela em que outro usuário local conecta. O diretório é 0700 quando criado por `init` (e `init` reaplica 0700), mas `agent::run` não confere o modo do diretório como confere o da chave. Além disso, `Err(_) => remove_file(socket)` apaga o socket de um agente **vivo** se o `connect` falhar por `EAGAIN` (fila cheia) ou outro erro que não seja "ninguém escuta".
- **Recomendação:** criar em nome temporário dentro do diretório, `chmod 0600`, `rename` para `agent.sock` (atômico, sem `unsafe`); remover o socket velho só em `ConnectionRefused`/`NotFound`; `load_identity` ou `run` recusam diretório de perfil com modo diferente de 0700.
- **Verificado OK (teste do Flux + leitura):** socket nasce 0600, segundo `serve` com o primeiro vivo dá `AgentAlreadyRunning`, quadro acima de `MAX_FRAME_BYTES` é recusado antes de alocar, `agent::run` falha antes de abrir a porta QUIC.

### Informativos

- **I-1** `PairMsg` deriva `Debug` (`proto/src/wire.rs:38`) e imprime os nonces inteiros (`Nonce { na: [9, 9, ...] }`, reproduzido). Nenhum caminho atual loga esse tipo (o `Frame::Pair` na malha fecha com `Protocol` antes do log), mas basta um `{:?}` futuro. Fazer `Debug` manual redigido, como os outros tipos sensíveis do §2.3.
- **I-2** `read_frame` aloca o tamanho declarado (até 1,1 MiB, zerado) antes de o corpo chegar, e as tasks por fluxo uni (`transport.rs:712-727`) não têm timeout. Um par já autenticado e pinado pode manter ~100 fluxos × 1,1 MiB virtuais por conexão. Atende à regra 10 (checa antes de alocar), só não é mínimo. Aceitável hoje.
- **I-3** Pareamento assimétrico possível: `finish` grava quando os dois `Confirm` são `accept`, mas cada lado grava por conta própria. Se a conexão cair depois de A gravar e antes de B receber o `Confirm` de A, A confia em B e B não confia em A. Falha fechada para B. Só registrar.
- **I-4** Chave privada em arquivo 0600 sem rotação nem revogação (já em §10 Aberto 2). Remover um par é editar `peers.toml` + `ReloadPeers`.
- **I-6** (reverificação) A lista de caracteres de formatação do `validate_device_name` não cobre U+00AD, U+180E, U+206A-206F, U+FFF9-FFFB, U+FE0F e tags U+E00xx. Nenhum move o cursor nem reordena texto, então não permitem forjar a linha do código; só tornam possível um nome visualmente igual a outro (confundível). Considerar recusar a categoria Unicode Cf inteira e normalizar espaços.
- **I-5** **Não verificado:** auditoria de dependências. `cargo audit` e `cargo deny` não estão instalados nesta máquina (`error: no such command`). O que deu para checar: 209 crates do registry, 0 de git, versões fixadas com `=` no workspace, `ring` como único provedor TLS. Instalar `cargo-audit` (ou rodar no CI) antes do release. A allow-list de reasons do teste `tls_` também não inclui `peer doesn't support any known protocol` (ALPN errado; texto constante do rustls, sem dado interno), observado no S1-04.

## Verificado, sem achado (com a evidência)

1. **Pinagem.** `PinVerifier` único implementa `ClientCertVerifier` e `ServerCertVerifier` (`tls.rs:198-270`). Valida o X.509 com o próprio cert como âncora, verifica a assinatura TLS 1.3 com ring, recusa cadeia com intermediários, recusa TLS 1.2. `VerifyMode::Pairing` só é construído em `pairing.rs:284` e `:419`, ambos com `ALPN_PAIR`; a malha usa `Mesh(pins)` no servidor e `Expect(id)` em cada discagem. ALPN errado é recusado nos dois sentidos. Retomada desligada: servidor `NoServerSessionStorage` + `send_tls13_tickets = 0`, cliente `Resumption::disabled()`; sem ticket não existe 0-RTT. Teste `tls_verifier_runs_on_every_connection_resumption_is_off`: contador do servidor 1, 2, 3 e do cliente 1 a cada conexão. Par desconhecido, chave trocada e cliente sem pino: `tls_unknown_client_refused_with_fixed_reason`, `tls_changed_server_key_detected_by_dialer`, `mesh_unknown_identity_never_connects`, `mesh_changed_key_is_blocked_with_constant_reason`. `install` fecha com `UnknownPeer` se o par saiu do `PinSet` no meio do caminho.
2. **Pareamento.** Commit/Nonce/Reveal na ordem do contrato; `commitment` com separação de domínio; o host só calcula o código depois de conferir H(nB); o código vem do exporter com contexto nB‖nA. Análise: o MITM compromete nB' antes de ver nA, e escolhe nA2 antes de ver nB; nenhum dos dois códigos fica previsível, sobra 1 em 10⁶ por janela, e a janela morre na primeira falha. Recusa não grava (`pairing_declined_on_one_side_saves_nothing`, `pairing_bad_reveal_fails_saves_nothing_and_host_ends`). Segunda conexão durante a tentativa é recusada (`extra.refuse()`, `pairing_window_takes_one_attempt_only`). Depois de falha, a porta não responde (`pairing_after_a_failure_the_window_is_closed`). O código depende dos dois nonces (`pairing_code_depends_on_both_nonces`). `Confirm` forjado pelo par não substitui a decisão local. Janela expira por `timeout_at(deadline, accept)`. Ressalva: S1-01 (a tela) e S1-04 (janela).
3. **Protocolo.** Tamanho checado antes de alocar em `read_frame` e em `control::read_message`; reproduzido: cabeçalho `u32::MAX` sem corpo devolve `TooLarge` em < 1 s sem esperar bytes, e 64 KiB + 1 também. Fluxo acima do limite recebe `stop`/`reset` com `Protocol` (`read_control`, `uni_task`). `validate` roda em todo `decode_frame_body` e `decode_datagram`, e antes de todo `encode_*`. Datagrama > 1200 bytes recusado nos dois sentidos; bytes sobrando são `Decode`. `Unreachable` é recusado no encode e no decode. Origem sempre é o `PeerId` do certificado da conexão (`uni_task(peer)`, `datagram_task(peer)`); nenhum campo de payload carrega origem. Ressalva: S1-05.
4. **Autorização.** `authorize_*` é chamado em `handle_frame`, `datagram_task`, `send_reliable` e `send_datagram`, e nada mais decide permissão. Ordem outbound e inbound confere com o contrato; módulo desligado → `ModuleDisabled` antes de tudo; pausa respeitada (e relida a cada datagrama); permissões por direção (`clipboard_in`/`clipboard_out`/`input_in`) e `reload` aplica mudança de permissão na hora, sem reconectar. As mensagens "abertas" (`Ack`, `Stats`, `StatsRequest`, `Leave`, `ReleaseAll`) passam sem permissão por contrato; confirmei que os módulos conferem o dono: input por `incoming[from].session`, `ctl.to == from` e `session` fixada; clipboard por `pending[(id, from)]`.
5. **Vazamentos.** Varri todo `tracing::*` do core e do proto: só id curto, contagens, tamanhos, estado. Nenhum texto de clipboard, keycode, chave, nonce ou pino completo em log, erro ou close. `CONNECTION_CLOSE`/`Bye` de aplicação usam só `CloseCode::reason()`; de handshake, só as strings constantes do rustls (`ApplicationVerificationFailure`, `UnknownIssuer`, e observado `peer doesn't support any known protocol`). Debug redigido de `ClipOffer`, `InputEvent`, `InputDatagram`, `ControlRequest`, `ModuleCommand`, `InboxItem` com testes (`redact`). Ressalva: I-1.
6. **IPC.** Ver S1-09 (Verificado OK).

## Testes executados

- `cargo test --locked --offline -p clipsync-proto -p clipsync-core` (alvo próprio, árvore de trabalho de 2026-10-27 ~15h50): proto 40 passed (authorize 4, ids 7, memhub 7, redact 5, wire 17), core 31 passed (unit 12, control 1, diag 1, mesh 6, profile 10, smoke 1), 0 failed.
- Provas adversariais (apêndice A), `cargo test --test s1_adversarial`: `s1_01`, `s1_03`, `s1_04`, `s1_05`, `s1_06` passam, ou seja, **cada falha descrita acima foi reproduzida**.

## Não coberto

Captura de pacotes (`tshark`/`tcpdump`: não usado), Windows/macOS, spoof de endereço em rede real (S1-07 só por leitura), fuzzing de `postcard`, os módulos `clipboard` e `input` além do tratamento das mensagens "abertas", e a UI. O módulo input (S3) tem pontos de segurança próprios (§8) que esta auditoria não revisou.

## Apêndice A · como reproduzir

Scratch fora do repositório: `/tmp/claude-1000/-home-kappxs-Documentos-ANTIGOS-clipsync--maestri-roles-8c9321a6-7070-9729-e695-64e2d83b7b0c/ea2f9fda-41f4-4311-87de-1e3f92651ef2/scratchpad/poc/` (crate `sentinel-poc`, só usa a API pública do core e do proto; `tests/s1_adversarial.rs`). Efêmero: se o diretório sumir, peça e eu converto em testes de regressão em `crates/clipsync-core/tests/` depois que as correções entrarem.

```
source ~/.cargo/env
cd /home/kappxs/Documentos/ANTIGOS/clipsync
CARGO_TARGET_DIR=$SP/target cargo build --locked --offline -p clipsync-cli      # binário da árvore atual
cd $SP/poc
SENTINEL_CLI=$SP/target/debug/clipsync CARGO_TARGET_DIR=$SP/target \
  cargo test --offline --test s1_adversarial -- --nocapture --test-threads=1
```

Núcleo da prova S1-01 (o resto do arquivo é o cliente `evil_join` que segue o protocolo à risca e o mini-terminal que aplica CR, LF, CSI A e CSI K):

```rust
let inj = format!("\x1b[1A\r\x1b[2KCódigo: 123 456\n");           // 26 bytes
let (real_code, _c) = evil_join(addr, &inj).await;                // Hello{device_name: inj}, Commit, Reveal corretos
// host = `clipsync pair host --allow clipboard-in` (processo filho, stdin "s\n")
let screen = render(&stdout);                                     // o que o usuário vê
assert!(screen.iter().any(|l| l.contains("Código: 123 456")));
assert!(!screen.iter().any(|l| l.contains(&real_code)));          // o código real sumiu
assert!(a.load_peers().is_err());                                 // S1-02
```

## Pergunta em aberto para o Conductor

S1-04 pede mudança de contrato (§5.3: o que conta como "tentativa"). Decisão do Keel antes de corrigir.
