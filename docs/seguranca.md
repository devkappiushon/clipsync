# Segurança · auditoria da fatia S1 (Sentinel, 2026-10-27)

**Veredito atual (reverificação, 2026-10-27): LIBERADO PARA S1, sem BLOCKER aberto.** Os quatro achados que o time corrigiu (S1-01, S1-02, S1-08, S1-09) foram reverificados no checkout e passam; o ataque do S1-01 foi repetido com o Hello malicioso cru contra o `clipsync pair host` real e é recusado. Ficam abertos, por decisão registrada no contrato v1.10, cinco achados de severidade baixa (S1-03, S1-04, S1-05, S1-06, S1-07), que continuam reproduzíveis e não bloqueiam.

**S5 · Windows (2026-10-27): LIBERADO para a tag v0.1.0**, 0 BLOCKER (ver a última seção deste arquivo).

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

---

# S5 · Windows (2026-10-27)

**Veredito para a tag v0.1.0: LIBERADO.** 0 BLOCKER · 0 ALTO · 3 MÉDIO · 8 BAIXO · 3 informativos. Condição: o README traz os avisos da seção "O que o README precisa dizer". Recomendo corrigir o S5-02 antes da tag (três linhas, sem dep nova), mas ele não bloqueia: o repositório é privado e o clipboard do Windows já sai como "experimental".

**Como foi verificado.** Leitura de `control.rs`, `profile.rs`, `agent.rs`, `adapter/{win,poll,mod}.rs`, `probe.rs`, `release.yml`, `ci.yml`, `Cargo.toml` e do código-fonte de `tokio-1.54.1` e `arboard-3.7.0` no cache local. **Nada do que é específico do Windows foi executado**: esta máquina é Linux e o `cargo check --target x86_64-pc-windows-gnu` não chega ao fim (falta `x86_64-w64-mingw32-gcc` para compilar o `ring`). O código `cfg(windows)` só passa por compilador e clippy no job `windows` do CI. Rodei aqui o teste `pipe_name_is_random_and_matches_the_contract_shape` (1 passou). Onde o achado depende de comportamento do Windows (DACL padrão, enumeração de `\\.\pipe\`), a base é a documentação da Microsoft e está marcado "a confirmar em Windows". A receita de confirmação está no fim da seção.

## Respostas às quatro perguntas

**1. Pipe de controle.**
- Outro usuário lê o nome? **Sim.** O namespace `\\.\pipe\` é listável por qualquer usuário local. O arquivo `agent.pipe` é protegido pela ACL do perfil, mas o nome também aparece na listagem. O nome sorteado impede só quem tenta criar o pipe **antes** do agente. Não é segredo.
- Outro usuário conecta e manda requisição? **Não, pela DACL padrão** (a confirmar em Windows). `ServerOptions::create` passa `SECURITY_ATTRIBUTES` nulo (`tokio-1.54.1/src/net/windows/named_pipe.rs:2282`). A Microsoft documenta que o descritor padrão dá controle total a SYSTEM, administradores e dono, e só **leitura** a Everyone. Mandar requisição exige escrita. Abrir só para leitura é possível (ver S5-04).
- Squatting no cliente? **Sim, por um caminho que o desenho não fecha** (S5-01): o pipe velho some depois de uma saída suja, e o nome já foi visto na listagem. O que vaza é o texto do `send --stdin`. A identidade do usuário não vaza: o cliente abre com `SECURITY_IDENTIFICATION` (padrão do tokio, `named_pipe.rs:2411`), então o impostor não consegue se passar pelo usuário.
- Escrita atômica e limpeza: **corretas** (`profile.rs:291-308`: arquivo temporário `create_new`, `sync_all`, `rename`; `control.rs:256` só apaga se o nome ainda é o seu). A limpeza só roda em saída limpa (S5-01).
- Algum caminho sem `first_pipe_instance`? **Não.** Só `control.rs:248-251` cria a primeira instância e ele liga a flag; `next_instance` (`:269-273`) a omite de propósito. Mas a flag protege pouco (S5-06).
- Falha da próxima instância: `?` em `:282` e `:287` encerra o `accept_pipes` e o agente sai, fechando o arquivo. Nunca fica um instante sem instância viva com o agente de pé, então não abre janela de squatting enquanto roda.

**2. Perfil no Windows.** Risco residual e avisos: S5-05.

**3. Clipboard.** Conteúdo em log: **não** (evidência abaixo). Marcador de gerenciador de senha: **ignorado** (S5-03, pendência). Escrita sem exclusão de histórico e nuvem: S5-02.

**4. Workflows.** Nenhum vetor de envenenamento do binário por cache ou por action de terceiros no `release.yml`. Pontos abertos: S5-07 a S5-11.

## Achados

### MÉDIO

**S5-01 · Squatting do pipe depois de saída suja do agente.** `control.rs:119-133`, `:256`, `crates/clipsync-cli/src/agent.rs:34-35`, contrato §11 regra 5.
- Cenário: (1) o agente sobe e publica `agent.pipe` com o nome N; (2) um segundo usuário local lista `\\.\pipe\` e anota N; (3) o agente morre sem limpar; (4) o segundo usuário cria o pipe N; (5) o primeiro usuário roda `clipsync send --stdin`; (6) `connect` lê o arquivo velho, abre N e **manda o texto ao impostor**. Sem o pipe de outro usuário, o cliente teria recebido `NotFound` e `AgentNotRunning`.
- Saída suja é comum no Windows: o único gatilho de parada é `tokio::signal::ctrl_c()` (`cli/src/agent.rs:35`). Fechar a janela do console, logoff, desligar e `taskkill` não passam pela limpeza.
- Precondição: máquina com mais de um usuário (compartilhada, RDP, servidor de terminal). Em PC de um usuário só, não há atacante.
- O impostor pode também responder qualquer coisa ao `Status`, `Pause` etc. Não consegue assumir a identidade do usuário (ver a resposta 1).
- Recomendação (uma): autenticar o servidor antes de mandar a requisição. `agent.pipe` ganha uma segunda linha com um segredo aleatório de 16 bytes. O cliente manda primeiro um nonce, o servidor responde `HMAC-SHA256(segredo, nonce)`, só então segue o quadro de sempre. Quem só listou o nome não tem o segredo e o cliente aborta sem enviar texto. Muda o quadro da §11 regra 5: decisão do Keel. Sem mudança de contrato, o mínimo é registrar no README (abaixo).

**S5-02 · Texto recebido de outro dispositivo entra no histórico do Windows e na nuvem da Microsoft.** `crates/clipsync-clipboard/src/adapter/win.rs:64`.
- `write_text` faz `open()?.set_text(text)`. O texto vindo do par vai para o Win+V (histórico) e, se o usuário ligou o clipboard na nuvem, é enviado à conta Microsoft: sai da malha de pares pareados.
- Prova (leitura): `arboard-3.7.0/src/platform/windows.rs:846-878` oferece `SetExtWindows::exclude_from_cloud` e `exclude_from_history`, e o `grep` por esses nomes e por `CanUploadToCloudClipboard` em `crates/` não acha nada.
- Atenuante: histórico e nuvem vêm desligados no Windows.
- Recomendação: `open()?.set().exclude_from_cloud().exclude_from_history().text(text)`. Sem dep nova, sem unsafe.

**S5-03 · Senha copiada de gerenciador de senhas é lida e enviada (pendência, não bloqueio).** `win.rs:28-29`, `adapter/poll.rs:33,40`.
- O polling de 250 ms lê todo texto novo e o envia aos pares com `clipboard_out`. Os formatos `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory` e `CanUploadToCloudClipboard` não são consultados. Prova: `arboard` só os **grava** (`windows.rs:813-840`); a leitura não olha marcador, e o repositório não tem a string.
- Efeito: um gerenciador que marca a cópia como sensível tem a senha enviada a todos os pares autorizados.
- Recomendação: antes de ler, `clipboard_win::is_format_avail(register_format("ExcludeClipboardContentFromMonitorProcessing"))` (e `CanIncludeInClipboardHistory` valendo 0) e ignorar o tique. API segura (sem unsafe). `clipboard-win 5.4.3` já está no `Cargo.lock` como dependência do `arboard`, mas virar dep direta é "dep nova" pela regra 1 da §11: perguntar ao Keel. Até lá, o README diz que o clipboard do Windows copia também o que gerenciadores de senha marcam como sensível.

### BAIXO

**S5-04 · O nome do pipe não é segredo e a DACL padrão deixa abrir para leitura.** `control.rs:154-156`, `:283`, contrato §11 regra 5 (a frase "só o dono lê esse arquivo" dá falsa sensação de sigilo).
- Qualquer usuário local lista o nome e, pela DACL documentada, pode abrir uma instância só para leitura. O servidor aceita sem limite (`PIPE_UNLIMITED_INSTANCES`, `named_pipe.rs:1780`) e cada conexão vira uma tarefa que espera até `REQUEST_READ_TIMEOUT` (10 s, `control.rs:97`). Milhares de conexões esgotam handles e memória do agente e deixam `send`, `status` e `pause` sem resposta. A malha QUIC não é afetada.
- Recomendação: limitar tarefas simultâneas do controle (por exemplo 16; excedente é fechado na hora) e reduzir `REQUEST_READ_TIMEOUT` no pipe. Limitar a DACL de verdade exige `create_with_security_attributes_raw`, que é unsafe: fica fora (regra 1).
- Corrigir o texto do contrato: o nome é público; a proteção é a DACL.

**S5-05 · Perfil: a chave privada fica protegida só pela ACL herdada e nada confere isso.** `profile.rs:403-404`, `:480`, `:333-339`, contrato §11 regra 4.
- Risco residual aceito pelo contrato: `--profile` ou `CLIPSYNC_PROFILE` fora de `%LOCALAPPDATA%`. Exemplos: raiz do disco (`C:\clipsync`: novas pastas herdam "Authenticated Users: Modificar"), `C:\Users\Public`, `C:\ProgramData`, compartilhamento de rede, pasta sincronizada (OneDrive, Dropbox). Nesses casos qualquer usuário local ou o serviço de sincronização lê `identity.key.pem` e se passa pelo dispositivo. O `load_identity` não avisa.
- Mesmo no padrão: qualquer processo do **mesmo usuário** lê a chave (igual ao 0600 do Linux), e administradores e backup também. Não há DPAPI.
- O arquivo temporário `identity.key.tmp-<pid>` (`profile.rs:292`) fica com a mesma ACL; um corte de energia deixa uma cópia da chave com esse nome. Inofensivo no perfil padrão.
- Recomendação: README (lista abaixo). Para depois do 0.1.0: conferir a ACL do diretório do perfil no `agent::run` (exige Win32; Keel decide a exceção da regra 1).

**S5-06 · `first_pipe_instance(true)` não impede um segundo agente do mesmo perfil.** `control.rs:244-251`.
- O nome muda a cada `serve`, então a flag só protegeria contra um pipe de mesmo nome, que não existe. A exclusão mútua real vem de `probe` (`:244`, com janela entre checar e criar) e do bind da porta UDP em `Mesh::start`, que roda antes. Com `listen` em porta 0 dois agentes sobem, o segundo reescreve `agent.pipe` e o primeiro fica sem arquivo.
- Recomendação: nada para o 0.1.0. Registrar que a exclusão depende da porta fixa; no Unix o `hard_link` dá exclusão de verdade.

**S5-07 · `ci.yml` não declara `permissions`.** `.github/workflows/ci.yml:1-12`.
- O token herda o padrão do repositório, que pode ser leitura e escrita. O `release.yml` declara certo (`contents: read` no topo e no job de build, `contents: write` só no job que publica, `:7-8`, `:15-16`, `:97-98`).
- Recomendação: `permissions: contents: read` no topo do `ci.yml`.

**S5-08 · Qualquer tag `v*` publica, em qualquer commit, sem CI verde nem aprovação.** `release.yml:3-5`, `:94-98`.
- Quem tem escrita no repositório cria a tag num commit de branch não revisada e o workflow publica. A checagem tag = versão (`:44-52`) só compara string.
- Recomendação: ruleset de tag para `v*` (só o dono cria) e `environment: release` com revisor obrigatório no job `release`.

**S5-09 · `actions/checkout` deixa o token no `.git/config` enquanto o `cargo build` roda código de ~209 crates.** `release.yml:31`, `ci.yml:18,47`.
- O token é só de leitura e o repositório é privado, então o dano é baixo. Build scripts e proc-macros podiam ler esse token.
- Recomendação: `with: persist-credentials: false` nos três `checkout`.

**S5-10 · Actions fixadas por tag móvel; o CI usa `@master` de terceiros.** `release.yml:31` (`checkout@v7`), `:86` (`upload-artifact@v4`), `:101` (`download-artifact@v4`); `ci.yml:22,49` (`dtolnay/rust-toolchain@master`), `:27,54` (`Swatinem/rust-cache@v2`).
- O release só usa `actions/*`, como manda a §11 regra 9. A tag `v4` pode ser movida por quem controla a action. O `download-artifact` roda no job com `contents: write`.
- O CI está liberado a usar terceiros, mas `@master` é ramo, não versão. O cache do CI não alcança o release (sem `cache` no `release.yml`, conferido).
- Recomendação: fixar por SHA de commit nas três actions do release e na `rust-toolchain`. Não consegui confirmar se `checkout@v7` existe (sem rede aqui); se não existir, o workflow falha fechado.

**S5-11 · `SHA256SUMS` não prova origem.** `release.yml:108`.
- O arquivo é gerado no mesmo job que publica e vai na mesma página do release. Detecta corrupção, não troca de binário por quem tem acesso à página.
- Recomendação: para o primeiro release público, `actions/attest-build-provenance` (oficial; pede `id-token: write` e `attestations: write` só no job `release`). Para o 0.1.0 privado, pode ficar.

### Informativos

- **I-7.** As notas do release (`release.yml:112-119`) dizem "Windows: ... clipboard de texto" sem "experimental" e sem "não testado em máquina real", o que a §11 regras 6 e 11 exigem. O `README` pode estar certo; a nota não.
- **I-8.** CRT estática: o `RUSTFLAGS` de `release.yml:59` está certo na leitura. Ninguém confere o `.exe` pronto. Sugestão: um passo que falhe se `dumpbin /dependents clipsync.exe` listar `VCRUNTIME140`.
- **I-9.** `cargo audit` e `cargo deny` não existem aqui nem no CI. As dependências do Windows (`windows-sys`, `clipboard-win`, `arboard`) não foram auditadas por advisory. `--locked` está nos dois builds de release.

## Verificado, sem achado (com a evidência)

- **Conteúdo do clipboard nunca em log.** `grep` por `tracing::`, `println!`, `format!` com texto em `crates/clipsync-clipboard/src`: o único log do módulo é `module.rs:572` (par e erro, sem texto). `probe.rs:60,87` imprime só mime, tamanho e milissegundos. `ClipSnapshot` tem `Debug` redigido (`adapter/mod.rs:38-47`). `ClipError::Other` vem de `arboard::Error::to_string()`, que não embute conteúdo. O hash SHA-256 só fica em memória (`poll.rs`, `ledger.rs`).
- **Linha de base.** O texto que já está no clipboard quando o agente sobe é a linha de base e não gera evento (`poll.rs:33`); um erro de leitura não conta como mudança (`poll.rs:40`).
- **Nome do pipe.** `new_pipe_name` usa `rand::random::<[u8;16]>()` (`control.rs:154-156`); `is_pipe_name` exige o prefixo local, 32 hex minúsculos e rejeita `\n` e `\\servidor\pipe\` (teste passou aqui). O cliente nunca abre nome fora desse formato (`:166-176`), então um `agent.pipe` adulterado não leva a caminho remoto.
- **Só local.** `reject_remote_clients(true)` na primeira instância e em todas as seguintes (`:248-251`, `:269-273`).
- **Quadro limitado.** `read_message` confere o tamanho antes de alocar (`:46`), igual ao Unix; uma requisição por conexão; resposta de erro genérica ("invalid request").
- **Publicação.** O arquivo só aparece depois do pipe existir (`:252-253`); `serve` recusa subir sobre agente vivo (`:244`) e propaga qualquer erro que não seja `NotFound` (`:195`) sem apagar nada.
- **Release.** Sem `cache` e sem `rust-cache`; só `actions/*`; `permissions` mínimas e por job; `--locked`; tag lida por `env:` e nunca interpolada no shell (`:46,65,78,124`); `ubuntu-24.04` fixo no build Linux; versão da tag conferida contra o `[workspace.package]` (primeiro `version =` do `Cargo.toml` é a linha 6; todos os crates herdam com `version.workspace = true`); `unsafe_code = "forbid"` no workspace.
- **Impersonação.** O cliente do tokio abre com `SECURITY_IDENTIFICATION | SECURITY_SQOS_PRESENT` (`named_pipe.rs:2411`), o que impede o servidor impostor de agir como o usuário.

## O que o README precisa dizer (Windows)

1. O perfil fica em `%LOCALAPPDATA%\clipsync` e guarda a chave privada. Não ponha `--profile` nem `CLIPSYNC_PROFILE` em raiz de disco, `C:\Users\Public`, `C:\ProgramData`, compartilhamento de rede ou pasta sincronizada (OneDrive, Dropbox). Só pasta dentro do seu perfil de usuário.
2. A proteção da chave é a ACL do seu usuário. Qualquer programa que rode como você, e qualquer administrador, lê a chave.
3. Em máquina com mais de um usuário (compartilhada, RDP), o Windows ainda é experimental: pare o agente com Ctrl+C. Se fechar a janela, rode `clipsync run` de novo antes de usar `clipsync send`.
4. O clipboard do Windows lê tudo o que é copiado, inclusive o que gerenciadores de senha marcam como sensível (pendência S5-03). Pause (`clipsync pause`) ao copiar senhas.
5. O texto recebido de outro dispositivo pode entrar no histórico (Win+V) e na nuvem do Windows até o S5-02 ser corrigido.
6. O clipboard do Windows é "experimental (polling de 250 ms, só texto, não testado em máquina real)".

## Como confirmar em Windows (10 minutos, duas contas)

1. Conta A: `clipsync run`; anote o nome em `%LOCALAPPDATA%\clipsync\agent.pipe`.
2. Conta B, PowerShell: `Get-ChildItem \\.\pipe\ | ? Name -like 'clipsync-*'` deve **listar** o nome (confirma S5-04).
3. Conta B: `$p=[IO.Pipes.NamedPipeClientStream]::new('.','clipsync-<hex>',[IO.Pipes.PipeDirection]::InOut); $p.Connect(1000)` deve dar `UnauthorizedAccessException` (confirma que não há escrita).
4. Conta B: o mesmo com `PipeDirection]::In`. Se conectar, S5-04 está confirmado; se der erro, a DACL é mais fechada que a documentada e o S5-04 cai para informativo.
5. Conta A: `taskkill /f /im clipsync.exe`. Conta B: crie `[IO.Pipes.NamedPipeServerStream]::new('clipsync-<hex>')`. Conta A: `echo teste | clipsync send --stdin`. Se B recebe o quadro, S5-01 está confirmado.

---

# S6 · webview e superfície (parcial, 32d7916)

**Veredito parcial (critério 8 da §12.9): 0 BLOQUEANTE · 2 MÉDIO · 14 BAIXO · 10 informativos.** Nenhum achado bloqueia a S6 sozinho. Os dois MÉDIO devem ser corrigidos antes do release v0.2.0, porque ambos deixam com acesso um par que o usuário quis remover. **Não encerra o critério 8:** o layout e o input do Vector (`layout.rs`, `input.rs`, commit 32d7916) chegaram durante a revisão e ficam de fora, e a revogação avisada ao par (contrato v1.16) ainda não existe (ver "Pendente").

**Base.** Pedido em 5a55688, reaberto em 4c1eaed (com `devices.rs` e `pairing.rs`) e feito sobre a HEAD, que andou durante o trabalho: 4c1eaed, 039fd63, 3c614e8, 2aeb1c0, 32d7916. Todo arquivo revisado é idêntico em 2aeb1c0 e 32d7916; o último commit mexeu só em `layout.rs`, `input.rs` e `tests/layout.rs`. Entrou no caminho: a casca ligada ao `clipsync-ui` real (81b7e49), a tela Preferências (5b8fea1), o clipboard, o diagnóstico e o autoteste do clipboard (2aeb1c0).

**Como foi verificado.**
- Leitura integral, num export de `git archive` (não no checkout compartilhado), de `tauri.conf.json`, `capabilities/default.json`, `build.rs` e `src/*.rs` do app; de `clipsync-ui/src/{lib,api,agent,status,onboarding,devices,pairing,clipboard,diag,prefs,autostart,events,msg}.rs`; de `profile.rs` e `pairing.rs` do core; de `self_test.rs` do clipboard; e de `ui/js/{api,dom,estado,app,areas}.js` mais as telas de pareamento, dispositivos, diagnóstico e preferências. `clipboard.js` e `layout.js`: busca de sinks e leitura do trecho do preview.
- Código-fonte de `tauri-2.12.5` e `tauri-plugin-notification-2.7.0` no cache do Cargo, para saber o que `core:default` e `notification:default` liberam e se o corpo da notificação é escapado.
- **Executado:** uma sonda descartável (7 testes, `cargo test -p clipsync-ui --test sentinel_probe`, todos passando) com agentes reais em perfis temporários. Ela fica fora do repositório, em `~/.cache/sentinel-s6-039fd63/` (`sentinel_probe.rs` e `probe.log`). Os achados marcados "executado" saíram dela; os marcados "por leitura" não foram executados.
- **Não verificado:** o app Tauri não compila nesta máquina (sem headers do WebKitGTK), então CSP, ACL e bloqueio de navegação foram lidos, não vistos rodando. Windows (`reg.exe`, instalador) não foi exercitado. A suíte de testes do próprio projeto não foi rodada por mim.

## BLOQUEANTE

Nenhum no recorte revisado.

## MÉDIO

**S6-01 · Corrida em `peers.toml`: um par revogado pode voltar sozinho (executado).** `crates/clipsync-ui/src/devices.rs:183-208` (`set_perms`) e `:211-223` (`revoke`), `crates/clipsync-core/src/pairing.rs:248-257` (`save` do pareamento), `crates/clipsync-core/src/profile.rs:291-308` (`write_private`), `:483-497` (`set_clipboard_mode`) e `:556-559` (`save_peers`).
- Todo comando que muda `peers.toml` carrega, altera e grava sem trava. A gravação passa por `write_private` com `sync_all`, então a janela entre a leitura e o `rename` dura milissegundos. Duas chamadas concorrentes perdem uma das alterações, e o Tauri roda os comandos em paralelo.
- Prova: `device_revoke` de B e `device_set_perms` de C em paralelo, 150 rodadas, runtime multi-thread. O par B revogado reapareceu em `peers.toml` em **136** rodadas, com o `revoke` respondendo `removed: true`. A permissão de C se perdeu em 3 rodadas e 10 chamadas falharam com erro de gravação, porque o `tmp` tem nome fixo por processo (`tmp-<pid>`).
- Efeito: o usuário vê "revogado", mas o par continua com a chave fixada e as permissões antigas, inclusive `input_in`. O mesmo vale para um pareamento que termina enquanto o usuário mexe numa permissão. Em uso normal a janela é pequena, mas é uma falha de revogação sem aviso.
- Correção: (1) uma função única no `Profile`, `update_peers(|peers| ...)`, que pega um lock de arquivo (`File::lock`, estável no Rust 1.89, sem dependência nova) e um `tokio::sync::Mutex` no `Ctx`, e lê, aplica e grava dentro do lock; `devices.rs`, `pairing::save` do core e a CLI passam a usar só ela. (2) Nome de `tmp` único por gravação (contador ou aleatório). (3) Teste de regressão com o roteiro da sonda.

**S6-02 · Recriar o perfil mantém `peers.toml` com as permissões antigas, inclusive `input_in` (executado).** `crates/clipsync-ui/src/onboarding.rs:119-165`, `crates/clipsync-core/src/profile.rs:439-441`.
- `onboarding_create` com `recreate: true` troca chave e certificado, mas `init_with` só cria `peers.toml` se ele não existir. A mensagem e o contrato dizem que "os pares terão de parear de novo", o que sugere que eles somem.
- Prova: perfil A com o par B (`clipboard_in`, `clipboard_out` e `input_in` ligados), `onboarding_create` com `recreate: true`: a identidade mudou e B continuou em `peers.toml` com `input_in = true`.
- Efeito: um par honesto não conecta mais, porque fixou a chave antiga. Um par perdido ou comprometido, que ignora a fixação, continua aceito pela nova identidade com controle de teclado e mouse. Quem recria o perfil costuma estar reagindo a um problema, que é o caso em que isso mais pesa.
- Correção: com `recreate` verdadeiro, gravar um `peers.toml` vazio e dizer isso na tela de confirmação (guardar o antigo como `peers.toml.bak` com modo 0600 só se o usuário pedir). Teste: depois do recreate, `devices_list` vem vazio.

## BAIXO

**S6-03 · O mock entra no pacote e o app cai nele se `window.__TAURI__` faltar.** `crates/clipsync-app/ui/js/api.js:9-12` e `:69-72`, `crates/clipsync-app/tauri.conf.json:6` (`frontendDist: ./ui`), `ui/mock.js`, `ui/mock/*.js`.
- Respostas ao item 2: o mock só é importado por `import("../mock.js")` quando `window.__TAURI__` é falso. `?cenario=` só é lido dentro do próprio mock (`mock/nucleo.js:6`), nunca antes dele. Com `withGlobalTauri: true`, dentro do Tauri o objeto existe, então nenhum dado do mock é usado e o mock não está no caminho de nenhum comando real (o `invoke` do mock só mexe em estado em memória). Isso vale por leitura; não vi o app rodar.
- A trava é uma só. Se a injeção do `__TAURI__` falhar, a interface mostra pares e dados inventados sem aviso, e o usuário pode agir sobre eles achando que são reais (fail-open). Além disso `mock.js`, `mock/`, `componentes.html`, `js/galeria.js` vão no binário, porque o `frontendDist` embute a pasta inteira.
- **O mock precisa sair do pacote.** Correção: (1) mover mock, galeria e página de componentes para fora de `ui/` (por exemplo `ui-dev/`) ou gerar um `frontendDist` filtrado no build; (2) em `api.js`, se `location.protocol === "tauri:"` ou `location.hostname === "tauri.localhost"` e `__TAURI__` não existir, rejeitar com erro visível em vez de importar o mock.

**S6-04 · O filtro de navegação é mais largo que o necessário.** `crates/clipsync-app/src/window.rs:26-32`.
- `"tauri" => true` aceita qualquer host, e `http(s)://tauri.localhost` aceita qualquer porta e credencial na URL. Os testes (`:58-71`) só cobrem domínio externo, `file:` e `javascript:`. Não há `on_new_window`. O front não tem links externos nem `window.open` (busca feita) e a CSP bloqueia frames, então hoje não há caminho de uso.
- Correção: aceitar só `tauri://localhost` e `http(s)://tauri.localhost` com `port().is_none()` e `username().is_empty()`; registrar `on_new_window` negando tudo; ampliar os testes com porta, `user@host` e `tauri://outro`.

**S6-05 · Nome de par entra sem escape no corpo da notificação do sistema (por leitura; depende do daemon).** `crates/clipsync-ui/src/events.rs:328-349` e `:358-364`, `crates/clipsync-ui/src/msg.rs:46-63`, `crates/clipsync-app/src/events.rs:74-79`; `tauri-plugin-notification-2.7.0/src/desktop.rs:492-493` passa o corpo direto ao `notify-rust`.
- `clean` e `validate_device_name` aceitam `<`, `>` e `&`. No Linux, daemons que interpretam a marcação do freedesktop (o GNOME mostra `<b>`, `<i>`, `<a href>`) podem desenhar o nome de um par pareado como texto formatado ou link clicável nas notificações "Bloqueio", "está conectado" e "enviou N bytes". Não testei com um daemon real.
- Correção: no Linux, escapar `&`, `<` e `>` em título e corpo antes de `show()` (em `notify`, na casca, com `cfg(target_os = "linux")`). No Windows o texto não é interpretado, então não escapar lá.

**S6-06 · O `.desktop` do autostart aceita quebra de linha no caminho e `$APPIMAGE` relativo (executado).** `crates/clipsync-ui/src/autostart.rs:53-56`, `:92-108`, `:121-130`.
- `desktop_quote` escapa `"`, `` ` ``, `$`, `\` e `%`, mas não `\n` nem `\r`. Prova: `Launcher::new("/tmp/x\nExec=/tmp/evil\nX-Injected=1").desktop_entry()` gerou **duas** linhas `Exec=` e uma chave injetada. `from_env` aceita `$APPIMAGE=app.AppImage` (relativo) e grava `"app.AppImage" --hidden`, que o ambiente gráfico resolve a partir de outro diretório. Exige controle do ambiente do processo ou um diretório de instalação com nome malicioso, ou seja, quem já executa como o usuário. Persistência no login é justamente o que o autostart faz, por isso registro.
- Correção: recusar (erro `io` em português) caminho que não seja UTF-8, que tenha caractere de controle ou que não seja absoluto; checar que `$APPIMAGE` é arquivo regular antes de gravar.

**S6-07 · Gravação de `ui.toml` e do `.desktop` sem `create_new`, sem modo 0600 e sem `fsync` (executado).** `crates/clipsync-ui/src/prefs.rs:153-164`, `crates/clipsync-ui/src/autostart.rs:121-130`.
- Os dois usam `fs::write` num `tmp` de nome previsível, que segue symlink. Prova: com um symlink em `ui.toml.tmp-<pid>` apontando para um arquivo da vítima, `prefs_set` respondeu `ok` e sobrescreveu a vítima. Os arquivos saíram com modo 664 (umask 002 aqui). Não há segredo neles e o diretório do perfil é 0700, então só o mesmo usuário planta o link: baixo. O padrão certo já existe em `write_private` (`profile.rs:291`).
- Correção: exportar e reaproveitar `write_private` do core nos dois; no `.desktop`, abrir com `create_new` e modo 0644 explícito.

**S6-08 · `prefs_set` pode dar erro depois de já ter mudado o autostart (executado).** `crates/clipsync-ui/src/prefs.rs:196-213`.
- A ordem é autostart primeiro, `ui.toml` depois. Prova: com `ui.toml` impossível de gravar, `prefs_set {autostart: true, appearance}` devolveu erro **e** criou o `.desktop`. O usuário lê "não foi possível salvar" e fica com início automático ligado, que é o item de consentimento explícito (§12.5, "padrão desligado").
- Correção: se a gravação de `ui.toml` falhar, desfazer o autostart (voltar ao estado anterior) antes de responder o erro.

**S6-09 · Gravação feita, mas o `ReloadPeers` falha: erro na tela e agente com a regra antiga (por leitura).** `crates/clipsync-ui/src/devices.rs:198-199`, `:220-221` e `:131-141`.
- `set_perms` e `revoke` gravam `peers.toml` e só depois pedem o reload. Qualquer erro que não seja "agente parado" volta ao front como falha, mas o arquivo já mudou. No `revoke`, o par removido do arquivo segue conectado no agente até ele recarregar ou reiniciar.
- Correção: se o reload falhar depois da gravação, responder com código próprio (`reload_failed`, com `details.saved: true`) e a tela dizer "salvo, mas o agente ainda não aplicou: reinicie o agente"; tentar o reload mais uma vez antes de desistir.

**S6-10 · Cancelar em `waiting_peer` pode deixar o par gravado e o resultado dizer `cancelled` (por leitura).** `crates/clipsync-ui/src/pairing.rs:351-358` e `:407-438`, `crates/clipsync-core/src/pairing.rs:226-237`.
- O core grava `peers.toml` de forma síncrona e só então espera o `ReloadPeers`. `pair_cancel` usa `task.abort()`, que pode cair depois do `save` e antes do reload. O `watch` vê a tarefa cancelada e emite `cancelled`, mas o par já está no arquivo e o agente não o conhece até recarregar. O contrato diz "cancelled e não grava"; o teste cobre só o cancelamento antes do `finish`.
- Correção: depois do `abort`, o `watch` relê `peers.toml`; se o par apareceu, emitir `paired` e mandar `ReloadPeers`. Ou tornar o trecho gravar e recarregar não cancelável (`spawn` interno).

**S6-11 · A janela de pareamento fica aberta ao sair da tela, e o teto de 600 s não está no contrato (por leitura).** `crates/clipsync-ui/src/pairing.rs:32` e `:244`, `crates/clipsync-app/ui/js/telas/pareamento.js:411-416`.
- `desmontar` só cancela as assinaturas e o relógio, não chama `pair_cancel`. Quem sai da tela com o anfitrião ouvindo deixa a porta de pareamento aberta em todas as interfaces pelo tempo restante (até 10 minutos com `window_secs: 600`), sem nada na tela mostrando isso. Pareamento sem decisão do usuário não acontece (recusa em 120 s), mas a porta aceita conexões não autenticadas, e o S1-04 (sonda QUIC com ALPN errado encerra o anfitrião) segue aberto.
- Correção: ao sair da tela com tentativa em `listening`, `connecting` ou `code`, chamar `pair_cancel`, ou mostrar um aviso fixo na barra lateral enquanto houver tentativa. Pôr `MAX_WINDOW_SECS` no contrato ou baixá-lo para 180.

**S6-12 · `event_id` aceita `+` (executado).** `crates/clipsync-ui/src/api.rs:377-392`.
- O id do item da caixa usa `u8::from_str_radix`, que aceita o sinal `+`. Prova: `"+f"` repetido 16 vezes foi aceito e é igual a `"0f"` repetido 16 vezes. Já `PeerId::from_hex` (crate `hex`) recusa `+`. Aliasing inofensivo hoje, mas a regra "32 hex" deixa de valer, e `clipboard_preview`, `clipboard_copy_item` e `clipboard_discard_item` usam esse parser.
- Correção: usar `hex::decode_to_slice`, como o `PeerId`.

**S6-13 · As notificações do sistema não podem ser desligadas pela tela (por leitura).** `crates/clipsync-ui/src/status.rs:108-119` (`os_notifications` fica `false`), `crates/clipsync-app/ui/js/telas/preferencias.js:105-123`, `crates/clipsync-ui/src/prefs.rs:51-58`.
- O front desabilita os três interruptores quando `os_notifications` é falso. Mas o `Notifications::default()` liga "Item recebido" e "Bloqueio", e a casca mostra a notificação de qualquer jeito, com nome do par, motivo e tamanho, inclusive na tela bloqueada. Quem não quer não consegue desligar pelo app.
- Correção: ligar `os_notifications` (a casca já mostra notificações, decisão 10), ou fazer a casca respeitar a feature e não emitir enquanto ela for falsa.

**S6-14 · O pacote de suporte é gravado sem `create_new`, com modo 664, e não é o texto da prévia (executado).** `crates/clipsync-ui/src/diag.rs:534-545` e `:249`, `:529-532`.
- `fs::write` direto em `Downloads/clipsync-suporte-AAAAMMDD-HHMMSS.txt`: segue symlink, sobrescreve o arquivo do mesmo segundo e sai com o modo da umask (664 aqui, legível por outros usuários locais). Prova: o arquivo exportado tinha modo 664. O conteúdo é regerado na exportação (a linha `gerado em (unix ms)` muda a cada chamada), então o arquivo **não é igual** à prévia que o usuário aprovou (executado: `file == prévia` deu falso). O contrato diz "o mesmo texto". A redação roda nas duas vezes, então o risco é o texto aprovado diferir do gravado, não vazamento.
- Correção: `bundle_export` receber o texto da prévia (ou guardá-lo na `Ui` por alguns minutos) e gravar esse; abrir com `create_new` e modo 0600; se o nome existir, acrescentar `-2`.

**S6-15 · A redação do pacote de suporte só troca nome como palavra inteira (por leitura).** `crates/clipsync-ui/src/diag.rs:309-338`, teste que fixa o comportamento em `:551-561` (`"pc-da-ana e ana"` mantém o primeiro `ana`).
- `-`, `_` e `.` contam como parte da palavra, então um nome de par ou o nome desta máquina dentro de um texto composto (`ana-notebook`, `casa.local`) passa sem redação. Hoje os textos que entram no pacote são gerados pelo próprio crate e os nomes aparecem soltos, então não achei vazamento em execução (a sonda trocou `ana` e `casa` corretamente). O risco vem de qualquer texto novo de módulo ou de diagnóstico que cite o nome dentro de um identificador.
- Correção: redigir também nome dentro de palavra composta (comparar sem limite de palavra para nomes com 4 ou mais caracteres, aceitando o excesso de redação) e acrescentar ao teste de "nenhum nome sobra" uma verificação sobre o texto final inteiro, com `contains`, para todo nome conhecido.

**S6-16 · O autoteste do clipboard escreve no clipboard real e pode perder conteúdo (por leitura).** `crates/clipsync-ui/src/onboarding.rs:210-250`, `crates/clipsync-clipboard/src/self_test.rs:49-113`.
- O teste lê o texto atual, grava um texto sintético, lê de volta e devolve o anterior. Conteúdo que não é texto (imagem, arquivo) não volta: o clipboard fica vazio. Uma cópia feita pelo usuário durante a janela de até 1,5 s é sobrescrita. No Windows o texto devolvido entra no histórico (Win+V) e na nuvem, como no S5-02. O comando `onboarding_self_test` não é limitado ao onboarding, então qualquer tela pode chamá-lo a qualquer hora.
- O agente é pausado antes (certo: o texto sintético não vai para os pares) e o `Pause` é persistido. Se o `Resume` falhar, só há um `tracing::warn!` (`:248`) e o app segue pausado sem avisar o usuário.
- Correção: recusar `onboarding_self_test` quando o perfil já foi inicializado há mais de um minuto, ou pedir confirmação na tela; tratar falha do `Resume` devolvendo `UiError` visível ("o agente ficou pausado"); documentar a perda de conteúdo que não é texto.

## Informativos

- **I-S6-1 · `core:default` e `notification:default` dão mais do que o front usa.** `capabilities/default.json:6`. No código-fonte do `tauri-2.12.5` (`build.rs`), `core:default` liga `emit`/`emit_to`, criação e edição de `tray` e `menu`, `path:resolve*`, getters de janela e `internal_toggle_devtools`; `notification:default` liga `notify`, canais e ações. O front só usa `invoke` e `event.listen` (`api.js:10`, `:70`), e a notificação sai pelo Rust (`events.rs` da casca), que não passa pela ACL. Mínimo suficiente: `allow-clipsync`, `core:event:allow-listen` e `core:event:allow-unlisten`. É mudança do contrato §12.7 regra 3 ("exatamente" os três), por isso fica como sugestão; hoje o dano é contido por CSP e ausência de conteúdo remoto.
- **I-S6-2 · Dureza opcional do Tauri:** `app.security.freezePrototype: true` não está em `tauri.conf.json`. Sem custo, reduz poluição de protótipo.
- **I-S6-3 · Os portões de confirmação são do front.** `acknowledge_full_trust`, `recreate` e o nome digitado no revogar são argumentos que o próprio front manda. O diálogo exige digitar o nome (`componentes.js:272-277`), mas o revogar envia `dispositivo.name` e não o texto digitado (`dispositivos.js:189`), então `name_mismatch` do backend nunca é a barreira real. Aceitável com CSP e sem conteúdo remoto; registro para ninguém contar com o backend como segunda trava.
- **I-S6-4 · `clean` não cobre todos os caracteres de formatação** (mesmo caso do I-6 da S1: U+00AD, U+180E, U+206A a U+206F, tags). E ele não limita tamanho: `unknown_command` ecoou 200.022 bytes e `bad_request` ecoou 50.058 bytes do valor recebido em `details` (executado, `api.rs:401-418`). Só o próprio front chama, então é ruído; um teto de 256 caracteres no eco resolve.
- **I-S6-5 · A chave privada fica em memória sem limpeza** em `pending_identity` entre `onboarding_start` e `onboarding_create` (`lib.rs:85`, `onboarding.rs:91-99`, clonada em `:150`). O `Debug` de `Identity` é redigido (`profile.rs:55-59`), então não vaza em log. Sem `zeroize` por decisão de dependência.
- **I-S6-6 · Travas seguradas durante E/S.** O `Mutex` de preferências fica preso durante `autostart::set` (que no Windows espera o `reg.exe`) e a gravação (`prefs.rs:198-214`). O `tokio::Mutex` do agente fica preso até 15 s em `launch` (`agent.rs:153`) e `app_state` espera por ele (`agent.rs:221`). Só travamento de tela, sem risco de dados.
- **I-S6-7 · `reg.exe` é chamado pelo nome.** `autostart.rs:195` e `:224`. O Windows procura primeiro na pasta do executável. Usar `%SystemRoot%\System32\reg.exe`. Não pude exercitar no Windows.
- **I-S6-8 · O S1-03 e o S1-04 agora são alcançáveis pela tela.** Parear com um endereço já conhecido substitui o par existente sem o `pair_result` avisar (`core/pairing.rs:248-257`), e a sonda QUIC com ALPN errado encerra o anfitrião (a UI mostra `failed`). Continuam abertos por decisão do contrato v1.10.
- **I-S6-9 · O texto de clipboard dos pares passa pelo processo da UI a cada segundo.** A consulta de eventos chama `clipboard::inbox` (`events.rs:233`), que traz os itens com preview; `InboxItem::new` descarta o texto na hora e nenhum evento o leva (conferido). Fica em memória só durante a consulta. Se o front ou um log novo passar a ver o item bruto, o conteúdo vaza; um teste que serialize o evento e procure um texto sintético trava isso.
- **I-S6-10 · O que está certo e foi conferido:**
  - A CSP de `tauri.conf.json:23` é idêntica à do contrato, sem `unsafe-inline` nem `unsafe-eval`, e `default-src 'none'` cobre frames, objetos e workers.
  - `capabilities/default.json` tem só a janela `main` e exatamente `core:default`, `notification:default` e `allow-clipsync`. `build.rs` declara só o comando `clipsync` no `app_manifest`.
  - O `tauri` do `Cargo.toml` raiz tem só a feature `tray-icon`: sem `devtools`, plugin único de notificação, versões fixadas com `=`.
  - Nenhuma URL remota na interface: o único `http` do front é o `xmlns` de um SVG em `data:`; as fontes são locais.
  - Busca no front por `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`, `eval`, `new Function`, `srcdoc`, `DOMParser`, `style=`, `href`/`src` dinâmicos, `fetch`, `WebSocket` e `window.open`: nenhum achado. `h()` (`dom.js:78-91`) põe todo filho como nó de texto; `style` só usa o CSSOM (permitido pela CSP); o `import()` de telas (`app.js:86`) usa um nome de uma tabela fixa; `localStorage` guarda só o tema.
  - Texto do par e do clipboard na tela (nome, motivo, endereço, preview, prévia do pacote, `agent_error`, detalhes do diagnóstico) entra por `h()` ou `textContent` em pareamento, dispositivos, clipboard, diagnóstico e preferências.
  - `UiRequest` recusa campo desconhecido (executado: `app_state` com `args: {x:1}` dá `bad_request`). `peer_id` usa `hex` estrito. Os grants de módulo desligado são cortados no core (`core/pairing.rs:221-223`). `pair_finish` antes de `pair_code_matches` dá `wrong_step`. O `confirm` recusa por tempo, cancelamento e ausência da `Ui`. A chave não aparece em `UiError`, evento nem log.
  - Clipboard: o preview é limpo e só então cortado em 200 caracteres (`clipboard.rs:263-274`), a lista nunca leva preview, `set_mode` grava `[clipboard] mode` por `write_private` e valida `auto` ou `manual` (`profile.rs:483-497`), e as mensagens do módulo mapeiam para códigos fixos, com o resto virando `internal` já limpo.
  - Pacote de suporte: não lê a caixa do clipboard, não inclui chave, `peers.toml` cru nem `portal-tokens.toml`; id, nome, IP e nome da máquina saem (executado: nenhum `ana` nem `casa` soltos no arquivo).

## Pendente (revisar quando o Maestro avisar)

- **Layout e input do Vector** (`layout.rs`, `input.rs`, commit 32d7916): validação de `layout_set_link`, limites de `x`, `y` e `seconds`, `layout.toml` por tmp + rename, texto de par em `Zones` e `EnterAt` passando por `clean`, `layout_copy_to_peers`, `layout_offer_answer` (aceitar grava uma oferta de par) e `input_test`.
- **Revogação avisada ao par** (contrato v1.16): ainda não implementada no `device_revoke`.
- **Input no autoteste:** a parte do input só entra quando o Vector entregar o `self_test` dele.
- **Windows** (`reg.exe`, instalador sem assinatura, `http://tauri.localhost`): nada exercitado.
- **Front de layout:** só busca de sinks; leitura completa junto com o backend.
- **Reverificação dos achados:** S6-01 e S6-02 pedem teste de regressão do Rivet; reabro quando o Maestro avisar da correção.

---

# S6 · final (415caf2 e 7c770ef)

**Critério 8 da §12.9 (Sentinel): PASSA.** 0 BLOQUEANTE · 0 MÉDIO · 6 BAIXO abertos · 11 informativos. Os dois MÉDIO da seção anterior (S6-01 e S6-02) estão fechados e provados por execução. Dos 16 achados da seção "S6 · webview e superfície (parcial, 32d7916)", 15 estão fechados e 1 (S6-16) ficou parcial, como BAIXO. A revisão das partes novas (layout e input do Vector, revogação avisada, autoteste do input, recusa registrada do lado recusado) achou 5 BAIXO novos. Nenhum bloqueia o release.

**Base.** Código revisado: 415caf2 (todas as correções do Flux, Pixel e Rivet, mais o Vector e a revogação avisada) e 7c770ef (só core). A HEAD hoje é 512f116: depois de 415caf2 só entraram README, versão 0.2.0, um teste do input, contrato v1.18 e o 7c770ef. `clipsync-ui` e `clipsync-app` ficaram idênticos desde 415caf2.

**Como foi verificado.**
- Export de `git archive` (não o checkout compartilhado). Cada correção foi conferida no código e, quando dava, repetida com a mesma sonda da rodada anterior (`~/.cache/sentinel-s6-039fd63/`, agora também `sentinel_probe2.rs`), contra o código novo.
- **Executado:** 9 testes da sonda (7 da rodada anterior, 2 novos) e a suíte do `clipsync-ui` inteira no export de 415caf2: todos os binários de teste passaram, 0 falhas (inclui `tests/layout.rs` com 13 testes e `tests/seguranca_s6.rs`). Os achados marcados "executado" saíram desses testes; os marcados "por leitura" não foram executados.
- **Não verificado:** o app Tauri continua sem compilar aqui (sem headers do WebKitGTK), então CSP, ACL e bloqueio de navegação foram lidos, não vistos rodando. Windows não foi exercitado. Não testei com um daemon de notificação real. Não forjei pacotes QUIC para o S6-21.

## Conferência das correções

| Achado | Estado | Evidência |
| --- | --- | --- |
| S6-01 corrida em `peers.toml` | **Fechado** (executado) | A mesma sonda de 150 rodadas deu 0 pares revogados de volta, 0 permissões perdidas, 0 erros (antes: 136, 3 e 10). `Profile::update_peers` (`profile.rs:572-584`) trava um arquivo e todos os escritores passam por ela: `devices.rs:205` e `:237`, `onboarding.rs:145`, `core/pairing.rs:251`, `core/agent.rs:158`. `write_private` ganhou nome de `tmp` único (`profile.rs:293-313`). |
| S6-02 recriar perfil | **Fechado** (executado) | Depois de `recreate: true` o par antigo some de `peers.toml` e a identidade muda (`onboarding.rs:136-146`). A lista é apagada antes de gravar a identidade nova, então uma falha deixa o lado seguro. |
| S6-03 mock no pacote | **Fechado** (leitura) | `ui/` não tem mais `mock*`, `galeria*` nem `componentes.html` (busca no export); eles estão em `ui-dev/`, e `tauri.conf.json`, `build.rs` e `Cargo.toml` não o citam. `api.js:9-35` só importa o simulador em `http://127.0.0.1` ou `localhost` e, fora disso, rejeita com erro visível. O servidor de dev (`scripts/servir-ui-dev.py`) escuta só em 127.0.0.1 e confere o caminho com `realpath`. |
| S6-04 navegação | **Fechado** (leitura e testes do módulo) | `window.rs:26-35`: sem porta, usuário ou senha; `tauri` só com host `localhost`; `on_new_window` nega tudo. |
| S6-05 marcação na notificação | **Fechado** (leitura) | `notify_text.rs` escapa `&`, `<` e `>` no Linux, `&` primeiro; título e corpo passam por `for_system` (`app/src/events.rs`). |
| S6-06 `.desktop` | **Fechado** (executado) | `Launcher::check` recusa caractere de controle e caminho que não seja UTF-8, e `enable_in` chama `check` (`autostart.rs:147-150`). `$APPIMAGE` relativo é ignorado (a sonda caiu no executável). Resíduo: `Launcher::new(...).desktop_entry()` chamado direto ainda monta o texto sem checar, mas nenhum caminho de gravação o usa sem passar por `check`. |
| S6-07 gravação de arquivos | **Fechado** (executado) | `ui.toml` sai com modo 600, e o symlink plantado em `ui.toml.tmp-*` não foi seguido (a vítima ficou `ORIGINAL`). `fsio::write_atomic` cria o `tmp` com `O_EXCL`, fixa o modo e faz `fsync`. O `.desktop` sai 644 de propósito. |
| S6-08 `prefs_set` parcial | **Fechado** (executado) | Com `ui.toml` impossível de gravar, o erro volta e o `.desktop` não é criado (`prefs.rs:196-250`, `undo_autostart`). |
| S6-09 reload falho | **Fechado** (leitura e teste) | `reload_after_save` tenta duas vezes e responde `config` com `details.saved: true` e o texto "salvo, mas o agente ainda não aplicou" (`devices.rs:147-155`). |
| S6-10 cancelar em `waiting_peer` | **Fechado** (leitura) | `pair_cancel` nessa etapa responde `wrong_step` (`pairing.rs`, `PAIR_ALREADY_DECIDED`); o fim fica com a outra máquina. |
| S6-11 janela de pareamento | **Fechado** (leitura) | `window_secs` agora vai de 1 a 120. `pareamento.js` cancela a tentativa ao sair da tela em `listening`, `connecting`, `code` e `permissions`, e não em `waiting_peer`. |
| S6-12 `event_id` | **Fechado** (executado) | `"+f"` repetido 16 vezes agora é recusado (`api.rs`, dígito a dígito). |
| S6-13 notificações | **Fechado** (leitura) | `features.os_notifications` é verdadeiro (`status.rs`), então os interruptores funcionam. A casca só mostra o tipo ligado e, sem preferências lidas, mostra nada (`notify_prefs.rs`, falha fechada). |
| S6-14 pacote de suporte | **Fechado** (executado) | Arquivo com modo 600, `create_new`, nome com sufixo `-2` a `-9` se existir, e o texto gravado é igual ao da prévia (a sonda deu `true`; antes `false`). A prévia fica guardada por 10 minutos por perfil. |
| S6-15 redação | **Fechado** (leitura e execução) | Nomes e ids com 4 ou mais caracteres saem em qualquer lugar do texto; os curtos saem como palavra isolada, e `-`, `_`, `.` contam como separador (`diag.rs:309-345`). A sonda não achou `ana` nem `casa` no arquivo. |
| S6-16 autoteste | **Parcial** (leitura e teste) | Um `Resume` que falha agora aparece no resultado da tela (`onboarding.rs`, `SELF_TEST_STILL_PAUSED`). Continuam abertos: perda de conteúdo que não é texto, e o comando poder ser chamado a qualquer hora. Vira BAIXO residual (abaixo). |
| I-S6-1 capabilities | **Fechado** (leitura) | `capabilities/default.json` agora tem exatamente `allow-clipsync`, `core:event:allow-listen` e `core:event:allow-unlisten`. O front só usa `invoke` e `event.listen`. |

## Partes novas revisadas

- **Layout e input do Vector (`layout.rs`, `input.rs`, `clipsync-input/src/{layout,self_test}.rs`).** Estado: **sem bloqueante e sem médio**; achei três BAIXO (S6-17, S6-18, S6-19).
  - Certo e conferido: `UiRequest` recusa campo desconhecido; ids de par passam por `known_peer` (64 hex e par do perfil); monitor com no máximo `MAX_NAME_BYTES`; `from` e `to` finitos e dentro de 0..1; ordem de validação da §12.5; `hotkey` de 1 a 9; `dead_corner_px` até 32; teto de ligações (`MAX_LAYOUT_LINKS`); `x` e `y` com teto de 1.000.000 em `set_device`; `base_rev` conferido dentro do lock (`WRITE`, um escritor por vez); `layout.validate()` e `ensure_no_manual_barriers` antes de gravar; `reload` falho vira `applied_but_not_loaded` com o arquivo salvo; todo nome e monitor de par passa por `clean`; `input_test` limita a 5 a 60 s, exige par pareado e conectado e vira `denied`, `offline` ou `not_linked` por texto fixo; `lock_to_screen` e `switch_to_device` só falam com o módulo. A oferta de layout de um par chega como `pending_offer` e só é gravada por `layout_offer_answer` com `accept: true`; `LayoutOffer` e `Zones` exigem `input_in`, como na tabela já verificada pelo Argus.
  - O autoteste do input só pergunta aos portais, sem diálogo, sem sessão, sem mexer em cursor ou tecla (`self_test.rs:1-18`, `:112-135`).
  - Front de layout: texto de par só por `textContent` ou `aria-label`; sem `innerHTML`, `style=`, `fetch` nem `window.open` (busca nos 3 sinks de novo, nada).
- **Revogação avisada (`transport.rs`, `agent.rs`, `control.rs`): sem achado.** Estado: **ok**. O par removido sai do id autenticado da conexão e não do conteúdo da mensagem (`control_task(peer, ...)`, `revoked_by(peer)`), então um par só consegue esquecer a si mesmo. `Bye{Revoked}` só vai a par com versão 2. Se `peers.toml` não grava, a revogação vale até reiniciar (falha para o lado seguro). Executado (`sentinel_probe2`): B muda a permissão que dá a A e A **não** o esquece; B revoga A e, em até 4 s, A e B não têm mais um ao outro em `peers.toml`. O canal de controle agora abre antes da malha, e um agente que falha ao subir desfaz a publicação do socket (`Published::drop`). `wait_port_released` só tenta fazer `bind` e solta a porta na hora.
- **Recusa registrada do lado recusado (7c770ef).** Estado: **um BAIXO** (S6-21). O texto vem de `PEER_REFUSED_US` (fixo, sem dado do par), uma entrada por par, e o nome sai por `clean` e pelo escape de notificação.

## BAIXO abertos

**S6-16 (residual) · O autoteste do clipboard escreve no clipboard real.** `crates/clipsync-ui/src/onboarding.rs:240-322`, `crates/clipsync-clipboard/src/self_test.rs:49-113`.
- O texto anterior volta; imagem ou arquivo não voltam, e uma cópia feita durante a janela de até 1,5 s é perdida. No Windows o texto devolvido entra no histórico e na nuvem (S5-02). `onboarding_self_test` ainda pode ser chamado a qualquer hora. O agente fica pausado durante o teste, então nada vai para os pares.
- Correção: pedir confirmação na tela quando o clipboard tiver conteúdo, ou recusar fora do onboarding; escrever a perda de conteúdo que não é texto no `detail` do resultado.

**S6-17 · `layout.toml` é gravado sem `create_new`, com modo 664 e segue symlink (executado).** `crates/clipsync-input/src/layout.rs:465-479`.
- `Layout::write` usa `File::create` num `layout.toml.tmp` de nome fixo. Prova: com um symlink em `layout.toml.tmp` apontando para um arquivo da vítima, `layout_set_device` respondeu `ok` e sobrescreveu a vítima; o arquivo saiu com modo 664. Mesmo padrão do S6-07, só que no crate do Vector. O diretório do perfil é 0700, então só o mesmo usuário planta o link: baixo.
- Correção: gravar por `create_new` com modo 0600 e nome único, como `write_private` do core ou `fsio::write_atomic` do crate da UI (mover para um crate comum).

**S6-18 · `layout_offer_answer` aceita "a oferta pendente", sem dizer qual (por leitura).** `crates/clipsync-ui/src/layout.rs:1155-1180`, `crates/clipsync-input/src/module.rs:853-857`.
- O módulo guarda uma só oferta e a substitui por qualquer `LayoutOffer` novo (de qualquer par com `input_in`). O comando não leva o par nem o `rev` da oferta que a tela mostrou. Se outra oferta chegar entre a tela e o clique, o usuário aceita a oferta nova sem ter visto. Só pares a quem o usuário já deu `input_in` (confiança total) podem ofertar, por isso é baixo.
- Correção: `layout_offer_answer {accept, from_peer_id, rev}` e responder `no_offer` ou `stale_offer` quando a pendente for outra.

**S6-19 · Aceitar uma oferta contorna o limite de posição do `set_device` (por leitura).** `crates/clipsync-ui/src/layout.rs:1127-1136` contra `:986-989`; `clipsync-input/src/layout.rs:364-381` (`validate` não confere `x` e `y`).
- `apply_offer` copia `x` e `y` do remetente (só encaixa na malha de 16 px), sem o teto de 1.000.000. Coordenadas absurdas deixam um dispositivo fora da tela do layout, difícil de achar e de mover de volta. Sem efeito sobre o input.
- Correção: aplicar o mesmo teto em `apply_offer`, ou melhor, em `Layout::validate`, que serve aos dois caminhos.

**S6-20 · A casca ignora o tipo `device_removed` (por leitura).** `crates/clipsync-app/src/notify_prefs.rs` (`allows`), `crates/clipsync-ui/src/events.rs:338-346`.
- A UI emite a notificação "Dispositivo removido" (segue a preferência `block`), mas `NotifyPrefs::allows` só conhece `item_received`, `device_connected` e `block`; o resto é negado. Resultado: ninguém é avisado de que um par o removeu. Falha para o lado seguro, mas a função não chega ao usuário.
- Correção: `"device_removed" => self.block` em `allows`, com teste.

**S6-21 · O registro "recusou a conexão" não exige que o certificado do par tenha sido verificado (por leitura, não executado).** `crates/clipsync-core/src/transport.rs:683-688` (ramo de erro de `dial_once`), `:720-725`, `crates/clipsync-core/src/tls.rs:331-334`.
- `refused_our_certificate` compara só o texto fixo do `CONNECTION_CLOSE`. No ramo de erro de discagem a conexão ainda não passou pelo fim do handshake, e o texto não é autenticado. Quem responder no endereço de um par (IP reaproveitado) ou injetar um pacote de fechamento consegue gerar o bloqueio falso "NOME recusou a conexão: este dispositivo não está mais pareado lá" e, com `block` ligado, a notificação de sistema. Efeito: informação falsa que empurra o usuário a reparear; não dá acesso a nada. Não forjei pacote para confirmar; a leitura mostra que o ramo não consulta se o verificador aceitou o certificado do par.
- Correção: registrar só no ramo pós-handshake (`:720-725`, com o par autenticado) ou exigir que o `PinVerifier` tenha aceitado o certificado antes; se não der, trocar o texto por "possível recusa" e não notificar.

## Informativos

- **I-S6-2 · `app.security.freezePrototype: true`** continua fora de `tauri.conf.json`. Sem custo.
- **I-S6-3 · Os portões de confirmação são do front** (`acknowledge_full_trust`, `recreate`, nome digitado ao revogar). O revogar envia `dispositivo.name`, não o texto digitado (`dispositivos.js:189`).
- **I-S6-4 · `clean` não cobre todo caractere de formatação e não limita tamanho.** Executado de novo: `unknown_command` ecoa 200.022 bytes e `bad_request` ecoa 50.058 bytes em `details`. Só o front chama.
- **I-S6-5 · A chave privada fica em memória sem limpeza** em `pending_identity` (`lib.rs`, `onboarding.rs:91-99`). O `Debug` é redigido.
- **I-S6-6 · Travas seguradas durante E/S.** O `Mutex` de preferências fica preso durante `autostart::set` e a gravação (`prefs.rs:196-220`); o do agente, até 15 s em `launch`. Somou-se o `File::lock` de `update_peers`, que bloqueia a thread do runtime enquanto outro processo grava `peers.toml` (curto).
- **I-S6-7 · `reg.exe` ainda é chamado pelo nome** (`autostart.rs`, Windows). Usar `%SystemRoot%\System32\reg.exe`. Não exercitado.
- **I-S6-8 · S1-03 e S1-04 continuam alcançáveis pela tela.** Parear com um endereço conhecido substitui o par sem avisar no `pair_result`, e a sonda QUIC com ALPN errado encerra o anfitrião (a janela agora dura no máximo 120 s).
- **I-S6-9 · O texto de clipboard dos pares passa pelo processo da UI a cada segundo** (`events.rs`, `clipboard::inbox`); é descartado na hora e nenhum evento o leva.
- **I-S6-11 · Nomes curtos na redação (S6-15, resíduo).** Nome com até 3 caracteres só sai como palavra isolada, então `ana1` ou `pc2` dentro de um texto passam. Os textos do pacote são gerados pelo crate e hoje não os trazem.
- **I-S6-12 · O servidor de dev da UI** (`scripts/servir-ui-dev.py`) escuta só em 127.0.0.1 e não entra no pacote, mas serve `ui-dev/` com `Cache-Control: no-store`; não o deixe rodando em máquina compartilhada.
- **I-S6-13 · `ui/README.md` vai no pacote** (o `frontendDist` embute a pasta inteira). Só texto, sem risco.

## Veredito do critério 8

**PASSA.** CSP idêntica à do contrato; capabilities no mínimo que o front usa; `build.rs` com um comando; navegação restrita à origem exata do app; nenhuma URL remota; sem `devtools`; o mock e a galeria fora do pacote com falha fechada; entrada validada, com campo desconhecido recusado e limites nos comandos do layout e do input; texto de par limpo no crate e só por `textContent` no front; gravações por arquivo novo com `fsync` e rename; nada de segredo em log, em `UiError` ou em evento. Nenhum BLOQUEANTE nem MÉDIO aberto. Recomendo corrigir S6-17, S6-20 e S6-21 antes da tag v0.2.0 (cada um é uma mudança pequena), mas nenhum dos seis BAIXO abertos impede o release.

**O que não cobre este veredito.** O app Tauri em execução (CSP, ACL e navegação lidas, não vistas), o Windows, o daemon de notificação real e o instalador. O critério 6 (app real) é do Maestro e do Argus.

## Reverificação dos BAIXO (af7b5f8)

**Resultado: os seis BAIXO abertos estão Fechados. Estado final da S6: 0 BLOQUEANTE · 0 MÉDIO · 0 BAIXO aberto · informativos I-S6-2 a I-S6-13 como acima.** O critério 8 segue **PASSA**. Revisado sobre a af7b5f8; o código de `crates/` não mudou até a 7b5bc13 (HEAD ao fechar).

Como foi conferido: leitura dos diffs de cada commit contra o export de `git archive`; as sondas `sentinel_probe2` e a nova `sentinel_probe3` (em `~/.cache/sentinel-s6-039fd63/`) contra o código novo; e os testes do próprio projeto que cobrem cada item (todos passaram: `clipsync-input` layout 33/33, `clipsync-core` testes `proven`, recusa e `a_server_without_the_peers_key_never_records_a_refusal`, `clipsync-ui` oferta absurda). O módulo `notify_prefs.rs` foi compilado e testado sozinho, fora do crate do app (5/5), porque o app não compila aqui.

| Item | Estado | Evidência |
| --- | --- | --- |
| S6-17 `layout.toml` (6b0e427) | **Fechado** (executado) | `Layout::write` usa `write_private` com nome de `tmp` único, `create_new`, modo 0600, `fsync` e rename (`clipsync-input/src/layout.rs`). A sonda: `layout.toml` saiu com modo **600** (antes 664) e o symlink plantado em `layout.toml.tmp` não foi seguido (a vítima ficou `ORIGINAL`). Testes do projeto: link no `tmp` e no próprio destino, escritas concorrentes sem `tmp` sobrando. |
| S6-18 oferta com par e rev (490335f, f66b595) | **Fechado** (executado) | Sonda com dois agentes reais: B ofertou o layout a A; resposta com par errado, rev errada e recusa com par errado deram `stale_offer` (com `details.from_peer_id` e `rev` da oferta pendente); sem os campos deu `bad_request`; a oferta seguiu pendente depois dos erros; a resposta certa foi aceita e gravou `layout.toml` com modo 600. O comando aplica a oferta lida na hora da checagem. O front manda `from_peer_id` e `rev` da oferta que mostrou e relê a tela em `stale_offer` ou `no_offer` (`layout.js`). |
| S6-19 teto de posição (e4e6308) | **Fechado** (leitura e teste) | `Layout::validate` recusa `x` ou `y` além de 1.000.000 com `unsigned_abs` (cobre `i32::MIN`), na validação, na escrita e na leitura de arquivo editado à mão. `apply_offer` agora satura em vez de cair para 0 (`saturating_i32`), então a posição absurda chega ao `validate` e é recusada. Teste `an_offer_with_absurd_positions_does_not_pass_validate` passou (limite exato aceito, `i32::MAX`/`MIN` recusados). |
| S6-20 `device_removed` (a84e102) | **Fechado** (leitura e teste) | `NotifyPrefs::allows` aceita `"device_removed"` junto com `"block"` e nada mais; sem preferências lidas continua tudo negado. O teste `a_removal_follows_the_block_switch_and_nothing_else` passou no módulo isolado. |
| S6-21 recusa registrada (d43a13b) | **Fechado** (leitura e teste; sem forjar pacote) | O ramo de erro de `dial_once` só registra se `verifier.proven() == Some(peer)`. `proven` só é marcado depois de o pino bater **e** a assinatura do handshake (`CertificateVerify`) validar com a chave do certificado (`tls.rs`, `verify_tls13_signature`); o verificador é novo a cada discagem, então nada vaza entre tentativas. O ramo pós-handshake já tinha o par provado. Testes: `proven_needs_the_pin_and_the_handshake_signature` (par real prova; outra chave não; o certificado do par mostrado por quem não tem a chave falha na assinatura e `proven` fica vazio) e `a_server_without_the_peers_key_never_records_a_refusal` (servidor com outra chave e servidor com o certificado do par e outra chave, ambos sem gerar entrada em `recent_blocks`). |
| S6-16 residual (0274626) | **Fechado** | O passo 3 do onboarding, único chamador de `onboarding_self_test` (`onboarding.js:278`), avisa antes do botão, ligado ao botão por `aria-describedby`, que o teste escreve um texto de teste na área de transferência, devolve o texto anterior e **não devolve imagem nem arquivo**; o aviso some se o clipboard está desligado. Resíduo aceito: o comando do backend segue chamável por quem falar com o agente, mas só o front do app o chama, e o agente fica pausado durante o teste. |

Informativos novos desta rodada:
- **I-S6-14 · Cópias do mesmo `write_private`.** Agora há três (core, `clipsync-ui/src/autostart/fsio.rs` e `clipsync-input/src/layout.rs`). Funcionam, mas a próxima correção tem que ser feita nos três; vale um crate comum quando houver espaço.
- **I-S6-15 · S6-21, resíduo teórico.** O close de nível Initial não é autenticado, e o cliente só descarta as chaves Initial ao enviar o primeiro pacote Handshake. Em teoria, um atacante no caminho que forjasse o fechamento depois de o servidor real provar a chave e antes desse descarte ainda produziria a entrada. A janela é de um ou dois pacotes e o efeito é só um aviso falso; não tentei reproduzir.
- **I-S6-16 · Licença mostrada em "Sobre"** agora é "Todos os direitos reservados" (constante `LICENSE`, 6454fe2). Nada de segurança; registro para o README e o release dizerem o mesmo.

## S7 · decisão unsafe (ed26b66)

**Veredito: APROVADA COM CONDIÇÕES.** Vale para a §13.3 (crate `clipsync-win` com `windows =0.62.2`, `unsafe` só em `src/ffi.rs`, lints próprios, greps no CI) e para a §13.2 regra 2 (privacidade dos hooks). O Flux pode começar o `clipsync-win` agora. As condições C1 a C4 e R1 a R4 são do código e do `ffi.rs`: eu as confiro na revisão bloco a bloco antes do v0.3.0, e uma condição aberta ali é BLOQUEANTE. Revisado só por leitura, pelo registro do crates.io e por provas em projeto de teste; **não há Windows nesta máquina**, então o comportamento real dos hooks (flag de injetado, `LowLevelHooksTimeout`, `RegisterHotKey`) é lido da documentação, não visto.

### O que conferi (evidência)

| Afirmação | Resultado |
| --- | --- |
| `windows 0.62.2` está no `Cargo.lock` e o Tauri já o puxa | **Sim.** Listado por `tao`, `tauri`, `tauri-runtime`, `tauri-runtime-wry`, `tauri-winrt-notification`, `webview2-com`, `webview2-com-sys` e `wry`. Os `windows-collections`, `-future`, `-numerics` e `-threading` do lock vêm dele e não são ativados pelas features da §13.3 (nenhuma é `Foundation_*`). |
| Versão confiável | crates.io: **não retirada** (`yanked: false`), publicada em 2025-10-06 por `kennykerr` (mantenedor do windows-rs, repositório `microsoft/windows-rs`), licença MIT OR Apache-2.0. Fixar com `=` e o checksum do lock cobrem a troca silenciosa. |
| As 10 features existem e bastam | Existem todas (lidas no `Cargo.toml` do crate no registro local). Achei 22 chamadas da §13.5 nos módulos dessas features, incluindo `SetWindowsHookExW`, `OpenInputDesktop`, `GetUserObjectInformationW`, `WTSRegisterSessionNotification`, `QueryDisplayConfig`, `GetDpiForMonitor`, `SendInput`. Nenhuma pede feature fora da lista. |
| `zbus =5.19.0` no lock | Sim (fora do meu pedido, só a conferência da §13.10). |
| `forbid` não aceita `allow` em módulo (E0453) | **Confirmado** num projeto de teste: `forbid` no workspace mais `#[allow(unsafe_code)]` num módulo dá `E0453`. |
| `[lints] workspace = true` junto de `[lints.rust]` | **Confirmado**: o Cargo recusa o manifesto. Logo o crate próprio da §13.3 (b) é mesmo o único caminho. |
| Os 7 crates de hoje herdam `forbid` | Sim: 7 de 7 têm `[lints] workspace = true`. Não há `unsafe` no código de hoje (3 menções só em comentário). |
| Com `deny`, quem garante "só no `ffi.rs`"? | **Só o grep.** No projeto de teste, `deny` mais `#![allow(unsafe_code)]` no `ffi.rs` compila; **um `#[allow(unsafe_code)]` em qualquer outro arquivo do mesmo crate também compila.** O clippy pegou o bloco sem `// SAFETY:`. Então a prova do CI é a única trava do isolamento dentro do `clipsync-win`, e ela tem furos (C1). |

### Condições da §13.3 (unsafe)

- **C1 · Fechar os furos do `scripts/conferir-unsafe.sh`.** Executei o script (7040a2f) num repositório de teste, com o `ffi.rs` válido e um segundo arquivo no `clipsync-win`. **Passam sem acusar:** `#[allow(dead_code, unsafe_code)]`, `#[expect(unsafe_code)]`, `#![allow(\n unsafe_code\n)]` em várias linhas, `unsafe` com a chave na linha de baixo e `#[unsafe(no_mangle)]`. Acusa só `allow(unsafe_code)` e `unsafe {` na mesma linha. Corrigir: fora do `ffi.rs`, falhar em **qualquer linha** com `unsafe_code` e em qualquer `\bunsafe\b` que não seja comentário de linha (hoje há 3 comentários que dizem "unsafe": `clipsync-cli/src/text.rs:100`, `clipsync-clipboard/src/adapter/win.rs:2` e `clipsync-ui/tests/autostart.rs:26`; reescrever como "inseguro" ou excluir linha que começa com `//`). Somar duas verificações: todo `crates/*/Cargo.toml` exceto o `clipsync-win` tem `[lints] workspace = true`, e o `clipsync-win` tem `unsafe_code = "deny"` e as duas lints de clippy da §13.3 item 2. Sem isso um crate novo sem `[lints]` nasce sem o `forbid`.
- **C2 · `ffi.rs` não exporta `unsafe`.** `mod ffi;` privado; toda função de lá é segura e fecha a invariante dentro do bloco (nenhum `pub` ou `pub(crate) unsafe fn`). Cada `// SAFETY:` diz por que ponteiro, tamanho de buffer e handle valem, não só repete a chamada.
- **C3 · Estado dos callbacks sem ponteiro cru.** O hook de baixo nível não recebe dado do usuário, então o `Box<dyn FnMut>` precisa de um lugar: `thread_local!` (`RefCell` ou `Cell`; a thread dos hooks é a mesma do laço de mensagens). Proibido `static mut`, `Box::into_raw` e `GWLP_USERDATA` com ponteiro cru (o caminho da janela oculta). Reentrada: `try_borrow_mut` e, se falhar, "passa". Cada `extern "system"` roda dentro de `catch_unwind` (o workspace é `panic = unwind`, `Cargo.toml:32`): pânico devolve "passa", marca o hook como morto e o `Notice` encerra a sessão. Pânico que cruza o limite C aborta o processo.
- **C4 · Handles com RAII.** `HHOOK`, `HWND`, `HDESK` e o registro WTS fecham no `Drop`. O `input_desktop_is_default` roda antes de cada lote do `WinEmulate`: sem guarda, cada chamada vaza um `HDESK`. Teste no CI Windows: 10 000 chamadas sem o `GetProcessHandleCount` subir.
- **Para o Maestro (§13.10).** Ao entrar o membro novo, o diff do `Cargo.lock` tem de mostrar só o pacote `clipsync-win` e a lista de dependências dele; nenhuma versão muda e nenhum pacote externo aparece. Builds com `--locked`.

### Condições da §13.2 regra 2 (privacidade dos hooks)

As alíneas (a) a (g) estão certas no desenho. Condições para elas valerem de fato:

- **R1 · A pausa contradiz o roteiro.** (g) diz que pausar tudo remove os hooks; o passo 6 da §13.12 retoma com Win+Alt+P, que só existe com o hook de teclado vivo. As duas não cabem. **Recomendo: a pausa remove os hooks** (hook de teclado vivo durante a pausa quebra a promessa que a pausa faz ao usuário); Win+Alt+P só pausa, retomar é pela bandeja ou pela CLI, e o passo 6 do roteiro muda. Decisão de produto, então vai ao Maestro. Até decidirem, vale (g) como está escrita.
- **R2 · (a) vale a cada mudança, não só na subida.** O hook cai na hora quando a última ligação local sai do `layout.toml`, quando o par da ligação é bloqueado ou revogado, e quando o módulo desliga. Ligação para par que não está pareado não conta.
- **R3 · Estado do teclado fora da captura é um conjunto de tamanho fixo.** O `winmath` fala em reconhecer atalho "a partir da sequência de teclas" (§13.6, `winmath.rs`): tem de ser função do conjunto pressionado agora mais a tecla que chegou, sem `Vec`, fila, janela de tempo ou contador por tecla. Teste: o tipo do estado é `Copy` e sem alocação. A alínea (c) diz "o pressionar do que está seguro" na ativação: fixar **só modificadores e botões**, como o item 4 do módulo, nunca letra.
- **R4 · O canal "sem limite" vira limitado.** O callback continua sem esperar: `try_send` num canal limitado (ordem de 8 000 eventos). Se encher, a captura termina (`Leave` e `ReleaseAll`), a fila é descartada e nada sai depois. Motivo: fila sem limite é histórico de teclas na memória e, com a rede travada, tecla digitada minutos antes chegando de uma vez ao destino. Na saída da sessão, o que sobrou na fila nunca é enviado.
- **R5 · Redação (e) por prova, não só por regra.** `impl Debug` manual de `RawEvent` e de `Key`, com teste de que `format!("{:?}")` não contém vk, scan nem extended. CI: nenhum `tracing::*!`, `println!`, `eprintln!` ou `dbg!` com argumento em `crates/clipsync-win/src/` e `adapter/win/`; `assert!` e `assert_eq!` com evento só em testes.

### Pontos aceitos e informativos

- **I-S7-1 · Injetado passa direto (b): aprovado.** Faz o que deve: programa local não dispara atalho nem abre sessão com tecla sintética, e o próprio ClipSync não entra em laço. Limite a documentar: o que o Windows marca como injetado inclui teclado na tela, sessão remota (RDP) e drivers de macro ou KVM. Com captura ativa, esse input vai para a máquina local e não para o destino; falha para o lado local, sem vazar tecla. Testar `LLKHF_INJECTED` e também `LLKHF_LOWER_IL_INJECTED`.
- **I-S7-2 · Hook de teclado ocioso e `RegisterHotKey`.** Fora da captura, o hook de teclado existe só para os atalhos Win+Alt+N, L e P, e vê toda tecla da máquina (restrito por (c)). Alternativa sem hook: `RegisterHotKey`, que só avisa quando o atalho exato é apertado, e o hook de teclado sobe só durante a captura. Win+Alt+dígito é do shell (jump list) e Win+L é reservado, então pode não registrar; **não testei, sem Windows**. Pedido ao Flux: testar no PC e registrar o resultado antes de escrever o hook ocioso. Se registrar, o hook ocioso some. Se não, fica como a §13.2 manda.
- **I-S7-3 · Hook removido em silêncio.** O Windows tira o hook que estoura o `LowLevelHooksTimeout`, sem aviso. Com o mouse em `CursorPark` o usuário ficaria sem cursor visível. Barato de cobrir: com captura ativa, a thread dos hooks confere a cada 1 s se o `GetCursorPos` ainda é o ponto estacionado; se não for, encerra a captura. O mesmo vale para o conjunto de teclas, que esvazia ao soltar: se o hook morrer com tecla segura, a tecla "fica presa" no conjunto; reconciliar com `GetAsyncKeyState` na ativação.
- **I-S7-4 · Teclas de senha na captura ativa.** Com a captura ativa, toda tecla vai ao destino, inclusive senha digitada nele. É o desenho (destino com `input_in` concedido pelo usuário). O limite de segurança é o §13.2 regra 1 (bloqueio e área segura), que o critério 7 da §13.11 manda eu rever no código.
- **I-S7-5 · Fora do escopo desta decisão e a rever no `ffi.rs`:** `CursorPark` (janela própria sob o cursor: some no `Drop` e com o processo; se o hook morrer, ver I-S7-3), o laço de mensagens e o `Drop` do `Hooks` (sem deadlock se rodar na thread dos hooks), e os buffers de `GetUserObjectInformationW`.

### Resumo para o Maestro

| Item | Veredito |
| --- | --- |
| §13.3 (b), crate próprio, `windows =0.62.2`, features, `ffi.rs` único | **Aprovada**, com C1 a C4 |
| §13.3 item 5, greps do CI | **Aprovada com C1** (furos executados) |
| §13.2 regra 2, hooks | **Aprovada**, com R1 a R5 |
| Bloqueante agora | **Nenhum.** O Flux pode começar o `clipsync-win`. |

## S7 · revisão do `clipsync-win/src/ffi.rs` (f80445d, c1047f6)

**Veredito parcial: SEM BLOQUEANTE · 0 MÉDIO · 4 BAIXO abertos.** Revisados os 53 blocos `unsafe` (1.144 linhas), o `lib.rs` e o `tests/windows.rs`, sobre o HEAD `bd52c10`. Esta rodada cobre só o crate; falta o adaptador (`adapter/win/`, hoje só `wincore.rs`) e os bloqueios de tela, que revisarei quando o Flux avisar. **Não há Windows aqui**: nenhum teste de hook rodou por mim. O CI Windows (run 40753074524) estava em andamento ao fechar; o resultado dele não está neste veredito.

Como foi conferido: leitura de todos os blocos contra a documentação do Win32 e o código do `windows 0.62.2` no registro local; `scripts/conferir-unsafe.sh` (passa); `cargo clippy -p clipsync-win --all-targets --target x86_64-pc-windows-gnu -- -D warnings` (**passa**, o que prova que os 53 blocos têm `// SAFETY:` e que nenhum bloco junta duas operações); `cargo test -p clipsync-win` no Linux (2 de 2, o do `Debug` redigido); `grep` por `static mut`, `into_raw`, `from_raw`, `GWLP_USERDATA`, `SetWindowLong`, `tracing`, `println!`, `dbg!` em `crates/clipsync-win` (só um `eprintln!` em teste, com a lista de atalhos recusados, sem tecla).

### Condições da decisão (7cd67a3) no código

| Condição | Estado | Evidência |
| --- | --- | --- |
| C1 script | **Fechada** | Reexecutei os 6 casos que passavam em branco (`allow(a, unsafe_code)`, `expect`, multilinha, `unsafe` com chave embaixo, `#[unsafe(no_mangle)]`) mais os 2 de controle: o script de hoje (2bd35a9) acusa todos. Ficou mais estrito: acusa até comentário que cita "unsafe" (os 3 de hoje foram reescritos). |
| C2 sem `unsafe` exportado | **Cumprida** | `mod ffi;` privado. Os 6 `unsafe extern "system" fn` (`enum_monitor`, `keyboard_proc`, `mouse_proc`, `watcher_proc`, `park_proc` e o tipo do parâmetro de `WindowClass::register`) são privados. Nenhum `pub unsafe`. 53 blocos, 53 `SAFETY`. |
| C3 callbacks | **Cumprida no código, sem teste** | `thread_local!` (`CALLBACKS`, `DESKTOP_WAS_DEFAULT`, `TICKS`, `ENUMERATED`, `CHECK_CURSOR`); nada de `static mut`, `into_raw` ou `USERDATA`. `try_borrow_mut` e, se reentrar, "passa" (`dispatch_input`, `notice`). `catch_unwind` em `keyboard_proc`, `mouse_proc`, `watcher_proc`, `enum_monitor`, `notice` e no `on_press` do atalho; nada que possa entrar em pânico fica fora (`finish` e `mark_lost` só usam `try_with` e `try_borrow_mut`). `Block` devolve `LRESULT(1)` sem `CallNextHookEx`; `code < 0` passa. O pânico sai pelo workspace `panic = unwind` (`Cargo.toml:32`). Ver S7-02: o caminho do pânico não tem teste. |
| C4 handles | **Cumprida** | RAII para `HHOOK` (`WatcherResources::drop`, teclado, mouse, timer, registro WTS, janela, classe, nessa ordem), `HWND` (`Window`), `HDESK` (`DesktopGuard`), cursor (`CursorResource`), classe (`WindowClass`), atalho (`HotkeyRegistration`) e o contexto de DPI (`DpiGuard`). As falhas no meio do `setup` soltam o que já foi aberto. Os dois testes de vazamento existem; o resultado no runner eu não vi. |
| R2 hook só enquanto há motivo | Do adaptador | O `Hooks` cai no `Drop`. Quem decide quando criar e largar é o `adapter/win/`, ainda por fazer. |
| R5 `Debug` redigido | **Cumprida** | `Debug` manual de `RawEvent::Key`, `RawEvent::Button` e `WinKey`; teste passou aqui. **Falta o grep no CI** (S7-04). |

### BAIXO

- **S7-01 · `async_key_down(vk: u16)` é público e lê qualquer tecla** (`ffi.rs:144`, `lib.rs:220`). Quem importar o crate consegue consultar a tecla que quiser a cada 10 ms, o que é um registrador de teclas sem hook, e a regra da §13.2 limita o uso a modificadores e botões na ativação. Hoje ninguém o chama fora do crate (conferi com `grep`). **Correção:** trocar o `vk` livre por um tipo fechado (`enum Held { Shift, Ctrl, Alt, Win, LeftButton, … }`) ou uma função `held_modifiers() -> ModifierSet`. Fechar antes de o adaptador usar.
- **S7-02 · `HooksLost` é permanente, ninguém o trata e o caminho não tem teste.** `mark_lost` põe `lost = true` e a flag nunca volta (`ffi.rs:650-665`); depois disso `dispatch_input` devolve "passa" para sempre. Fora do crate não há uso de `HooksLost` (conferi). Dois efeitos: (a) se o adaptador ignorar o aviso, o Input fica morto sem o usuário saber e o estado na tela diz "ligado"; (b) um falso positivo (outro programa chama `SetCursorPos`, ou o `SetCursorPos` do `CursorPark` é recortado pelo Windows e o ponto não bate) mata a captura até o módulo reiniciar. Falha para o lado seguro, sem vazar tecla. **Correção:** o `adapter/win/` trata `HooksLost` como `Leave` + `ReleaseAll` + recriar o `Hooks`, e o `ffi.rs` ganha um teste que roda `keyboard_proc` com um `KBDLLHOOKSTRUCT` montado à mão (flags 0) e um `on_input` que entra em pânico, conferindo "passa", um único `HooksLost` e hook morto. Hoje `tests/windows.rs` tem 0 menções a `HooksLost`.
- **S7-03 · Ponteiro com procedência só do cabeçalho** (`ffi.rs:260` e `269`). `DisplayConfigGetDeviceInfo(&mut source.header)` entrega à API um ponteiro derivado do campo `header`, e a API escreve a estrutura inteira. Na prática funciona (é o exemplo corrente do windows-rs e o compilador não reordena em volta de uma chamada externa), mas é formalmente fora do que o ponteiro cobre. **Correção:** como já foi feito com `MONITORINFOEXW` na linha 303: `std::ptr::from_mut(&mut source).cast::<DISPLAYCONFIG_DEVICE_INFO_HEADER>()`, e igual para `target`, ajustando o `SAFETY`.
- **S7-04 · O grep do R5 não existe no CI.** Pedi `tracing`, `println!`, `eprintln!` e `dbg!` com argumento barrados em `crates/clipsync-win/src/` e `adapter/win/`. Hoje o código está limpo (só o `eprintln!` do teste), mas nada impede o próximo commit. **Correção (Maestro):** um passo em `scripts/conferir-unsafe.sh` ou parecido; vale também para `keymap.rs` e `winmath.rs`, que tratam tecla.

### As duas perguntas do Flux

- **Janela 5x5 com alfa 1 e cursor vazio: aprovada.** Com a captura ativa o hook bloqueia todo movimento, então o cursor fica no ponto e a janela só serve para escondê-lo. Se o hook morrer, o cursor se mexe, sai de cima da janela e reaparece sozinho, e o relógio de 500 ms ainda acusa `HooksLost`: a falha é visível e sai pelo lado do usuário. Pontos conferidos: `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW` não rouba o foco nem aparece na barra; `WS_EX_TOPMOST` cobre só 25 pixels; um clique que caia nela nesse intervalo é engolido (aceito); a classe, o cursor e a janela são destruídos na ordem certa nas duas saídas (erro no `setup` e `Drop`).
- **`PARKED` como `AtomicU64` estático: aprovado.** Guarda só um ponto, não tem dado de tecla, e `Release`/`Acquire` bastam para um valor único. Dois resíduos, ambos sem efeito de segurança: o valor "sem parada" (`u64::MAX`) é igual ao ponto `(-1, -1)`, que existe num desktop com monitor acima ou à esquerda do principal (só desliga a conferência se o centro da barreira for exatamente esse pixel); e como é único no processo, duas `CursorPark` ao mesmo tempo se pisariam (hoje só há uma por sessão).

### Informativos

- **I-S7-6 · O atalho de retomada reage a input sintético.** `RegisterHotKey` aceita Win+Alt+P injetado por qualquer programa do mesmo nível, ao contrário do §13.2 (b) para os hooks. O efeito é só tirar o módulo da pausa (religar os hooks do próprio ClipSync); sem vazamento. Registrar no roteiro.
- **I-S7-7 · O teste de injetado não tem controle positivo.** `injected_input_never_reaches_the_callback` passa também se o hook nunca disparar. Não há como gerar evento físico no CI, então o controle fica no roteiro manual (digitar de verdade e ver o hook reagir).
- **I-S7-8 · `hotkey_registrable` usa o mesmo atalho do `ResumeHotkey`.** Chamado em pausa, ele acusa Win+Alt+P como recusado porque o próprio processo já o registrou. É só diagnóstico.
- **I-S7-9 · `ThreadHandle::drop` manda `WM_QUIT` por id de thread** (`ffi.rs:493`). Se a thread já tivesse saído, o id poderia ter sido reaproveitado por outra do processo (num app Tauri, a do WebView). Hoje a thread só sai pelo `WM_QUIT`, então a janela de risco não existe; uma flag "terminei" posta pela própria thread fecharia a porta. O `PostThreadMessageW` também depende de a fila de mensagens já existir; o `init.send` vem depois de criar janela ou registrar o atalho, que a criam.
- **I-S7-10 · `keyboard_verdict` e `mouse_verdict` são funções seguras que desreferenciam um `lparam` cru.** São privadas e só o `*_proc` as chama, então é aceitável; mais limpo seria o `*_proc` copiar a estrutura e passar o valor.
- **I-S7-11 · `send` devolve `Ok(n)` com `n` menor que o lote** (o UIPI recusa sem erro). O adaptador precisa tratar `n < len` como falha e soltar o que injetou, senão fica tecla presa no destino. Conferir no `adapter/win/`.
- **I-S7-12 · `process_handle_count` e `hotkey_registrable` fazem parte da API pública** só para os testes. Sem risco; poderiam ficar atrás de uma feature `test-support`.

### Falta para o veredito final do critério 7 (§13.11)

Adaptador `adapter/win/` contra a §13.2 regra 2 e a §13.6 (hook só com ligação local, sem histórico de teclas, canal limitado, `HooksLost`, `n < len` do `send`, `async_key_down` só para modificadores); o estado do bloqueio de tela nos dois sistemas; `Returned` e `Refused` só para sessão do mesmo par; o resultado do CI Windows e do roteiro manual.

## S7 · final (8cc14c4)

**Veredito do critério 7 (§13.11): PASSA, sem BLOQUEANTE.** Estado final da S7: **0 BLOQUEANTE · 1 MÉDIO · 2 BAIXO abertos**, mais os informativos abaixo. Recomendo corrigir o **S7-05** antes da tag v0.3.0 (é uma linha de reset e uma troca de fonte); os dois BAIXO podem ir depois. Revisado por leitura sobre o HEAD 8cc14c4 (`clipsync-win`, `adapter/win/{capture,emulate,mod}.rs`, `wincore.rs`, `winmath.rs`, `adapter/session.rs`, `module.rs`, `capture.rs`, `clipsync-proto/src/{input,module}.rs`).

**O que não cobre este veredito.** Não há Windows aqui: nenhum hook, `SendInput`, bloqueio de tela ou `RegisterHotKey` rodou por mim, e o CI Windows da base (run 40757283200) estava **em andamento** ao fechar, então os testes que só rodam lá (`ffi.rs`, `tests/windows.rs`, vazamento de handle) não estão neste veredito. O roteiro manual (critério 8) é do usuário.

### Evidência executada

- `scripts/conferir-unsafe.sh` e `scripts/conferir-privacidade.sh`: passam (o segundo olha 8 arquivos e já roda nos dois runners, `ci.yml:50` e `:89`).
- `cargo test --locked -p clipsync-input -p clipsync-win` no Linux: **verde** (47 + 10 + 10 + 10 + 21 + 27 + 21 + 8 e os de doc, 0 falha), incluindo `a_failed_open_does_not_begin_the_capture`.
- `cargo clippy --locked -p clipsync-win -p clipsync-input -p clipsync-cli -p clipsync-core --all-targets --target x86_64-pc-windows-gnu -- -D warnings`: **limpo**, o que compila e lint-a o `cfg(windows)` inteiro (adaptador e `ffi.rs`, com `undocumented_unsafe_blocks` e `multiple_unsafe_ops_per_block` em `deny`). O `clipsync-app` não entra: o script de build do Tauri não roda aqui, como já era.
- `loginctl show-session` desta máquina devolve `LockedHint=no`, e o teste do `session.rs` leu o logind real.

### O `ffi.rs` depois do aac844e (reconfirmação)

Diff de `c8cf755..HEAD` no `ffi.rs` e no `lib.rs`; tudo o que mudou fora de teste:
- **S7-01 fechado.** `async_key_down(vk)` saiu. A API pública agora é `modifier_down(Modifier)` e `button_down(WinButton)`, tipos fechados (8 modificadores, 5 botões); o `async_down(vk)` é privado. Nenhuma letra pode ser lida por esse caminho.
- **S7-03 fechado.** As duas chamadas de `DisplayConfigGetDeviceInfo` passam `ptr::from_mut(&mut source).cast()` e o `SAFETY` foi reescrito para dizer isso.
- **I-S7-11 fechado.** `send` devolve `Err` quando o sistema aceita menos entradas que o lote.
- **S7-02, parte do teste, fechada.** Três testes novos dentro do `ffi.rs` chamam `keyboard_proc` e `mouse_proc` com estruturas montadas à mão: pânico no teclado e no mouse passa o evento e manda `HooksLost` uma única vez, o hook morto não chama mais o fechamento, e `Block` bloqueia. Li o código dos testes; eles só rodam no Windows (esperam o CI).
- Os novos blocos `unsafe` (`async_down`, os dois `from_mut`, os dos testes) têm `SAFETY` coerente; o clippy acima confirma. Nenhum `unsafe` fora do arquivo, nenhum `pub unsafe`.

### As condições da decisão no adaptador

| Condição | Estado | Onde |
| --- | --- | --- |
| **R2** hook só com motivo | **Cumprida** | `start_continuous` não instala nada sem `derived_links()` (ligação para par confiável); `retarget` e `sync_links` (a cada 2 s e a cada par que cai ou é revogado) derrubam a captura quando a última ligação some; `Paused(true)` com `holds_hooks` chama `stop_continuous`, e o `start_continuous` recusa subir pausado (`module.rs:704`). Um loopback em andamento na hora da pausa também termina e não deixa hook. |
| **R1** pausa remove os hooks, Win+Alt+P retoma | **Cumprida** | Em pausa só existe o `ResumeHotkey` (`RegisterHotKey`), que o módulo arma em `arm_resume` e solta em `disarm_resume`; o `WinCapture::stop` derruba os hooks antes de tudo. Falha do sistema ao registrar vira o texto fixo `RESUME_HOTKEY_REFUSED` e a retomada pela bandeja. |
| **R3** estado fixo, só modificador e botão | **Cumprida** | `Held` é `Copy` de 4 palavras mais um byte (`wincore.rs:99`), `HotkeyTracker` é `Copy` com `u8`, `u8`, `[u32; N]` e um `bool`, sem histórico; `press_batch` carrega só `modifiers()` e `buttons()`; `reconcile` só consulta os 8 modificadores e os 5 botões. O `Debug` de `Held`, `Out`, `HotkeyTracker` é redigido. |
| **R4** fila limitada, sem tecla velha | **Cumprida** | `sync_channel(8000)` com `try_send` (o callback não espera); cheia, a captura termina no próprio hook (`core.end_capture()`), o worker esvazia a fila (`while try_recv`) e manda `Deactivated`. O canal do módulo é limitado (4096) e o worker usa `blocking_send`: se travar, a fila de 8000 enche e o ciclo acima corre. Na saída normal da sessão, o que sobrar na fila chega ao engine, que descarta (`CaptureEvent::Input` com `session.is_none()` retorna, `capture.rs:461`). |
| **R5** sem log de tecla | **Cumprida** | Script no CI dos dois sistemas; `Debug` redigido em `RawEvent`, `WinKey`, `Held`, `Out`, `HotkeyTracker`. O `probe` do Vector só conta tipos de evento e compara o código do Esc sem imprimi-lo. |
| **§13.2 2(b)** injetado passa sem ler | **Cumprida** | Os dois flags (`INJECTED` e `LOWER_IL_INJECTED`) são testados antes de ler `vkCode`, no teclado e no mouse. |
| **§13.2 2(f)** callback sem espera | **Cumprida** | No `hook_closure` só há atômicos, `try_lock` e `try_send`; todo o resto (estacionar o cursor, `SendInput`, canal do módulo) é do worker. |
| **§13.2 2(g)** hooks saem no `Drop` | **Cumprida** | `WinCapture::drop` tira o `Hooks` primeiro; `stop` também. Ver S7-07 para a thread que pode sobrar. |

### HooksLost

Tratado como decidido: `recreate_hooks` encerra a captura (`end_locally`), derruba o `Hooks` morto, manda `Deactivated` (o módulo responde `Leave` e o destino solta tudo), e a política (`RestartPolicy`, `Copy`, 3 vezes em 10 s) recria; a 4ª vez desliga (`locked = true`, sem hooks, `Error(HOOKS_GAVE_UP)` com texto fixo e sem código de tecla) até o módulo reiniciar. O `Drop` do hook morto roda na thread do worker, nunca na do hook, então não há junção da thread com ela mesma. A política tem testes na máquina de decisão pura. Falso positivo continua possível (outro programa chama `SetCursorPos` com captura ativa): o efeito é encerrar a sessão, para o lado seguro.

### Achado

- **S7-05 · MÉDIO · O conjunto de teclas segura valor velho entre uma instância de hook e a seguinte** (`adapter/win/capture.rs:76`, `:187-193`, `:296-314`). `Shared.held` é publicado só quando o `core.held()` do hook muda (`held != stored`), e `stored` e o `core` nascem vazios a cada `Hooks::start`. Nada zera `Shared.held` em `start()` nem em `recreate_hooks` (conferi todas as escritas: só a linha 189, dentro do callback). O `end_locally` lê esse valor e **aperta de novo localmente** todo modificador e botão dele (`:306-314`), em vez de consultar o que está fisicamente seguro como a §13.6 manda. Cenário, no fluxo normal do próprio produto: Win+Alt+P pausa (o hook viu Win e Alt descerem, `held` = {Win, Alt}); os hooks caem; o usuário solta as teclas sem hook nenhum; Win+Alt+P retoma e o hook novo sobe vazio; o usuário atravessa a borda **só com o mouse** e volta sem tocar numa tecla. Como o `core.held()` continua vazio, o publicado não é sobrescrito, e o `end_locally` da volta injeta **Win e Alt presos** na máquina local (o menu Iniciar ou o menu Alt na próxima tecla, ou Ctrl preso e os atalhos que ele dispara). O mesmo acontece com `HooksLost` com tecla segura. O critério de pronto pede que "nada fique preso no PC". Lido e raciocinado, **não executado** (o arquivo é `cfg(windows)`). **Correção, pequena:** (1) zerar `Shared.held` em `start()` e em `recreate_hooks`; (2) no `end_locally`, apertar de novo só o que `reconcile(Held::default())` diz que está fisicamente seguro (`modifier_down` e `button_down`), que é o que a §13.6 já descreve; (3) um teste do `wincore` ou do `capture` que cubra "pausa, retoma, atravessa só com mouse, volta" e confira que o lote de volta não traz modificador.
- **S7-06 · BAIXO · Resto de depuração em código de produção do Linux** (`adapter/session.rs:29`). `eprintln!("DEBUG logind: {e}")` roda quando o logind não responde. O erro é do D-Bus e não traz tecla nem segredo, mas é um `eprintln!` num caminho de produção e o `conferir-privacidade.sh` não cobre esse arquivo. Já existe o `tracing::debug!` na linha de cima: remover o `eprintln!`. (O da linha 146 é de teste.)
- **S7-07 · BAIXO · A thread do worker pode sobrar depois do `Drop` do `WinCapture`** (`capture.rs:662-670`). O worker guarda um `SyncSender` da própria fila (`queue`), então `inbox.recv()` nunca acusa fim; o `Drop` só tenta `try_send(Msg::Stop)`, que falha com a fila cheia. Os hooks já foram removidos (a regra 2g está cumprida), então sobra uma thread parada com o `Shared`. Só acontece em queda sem passar por `stop()`. Correção: o worker não guarda o sender, ou o `Drop` marca uma flag de fim.

### Bloqueio de tela, nos dois sistemas

- **Windows, destino:** `ensure_unlocked` roda antes de cada lote e no `enter`: sessão bloqueada (WTS) **ou** área de entrada diferente de `Default` dá `InputError::Locked`, e a falha da chamada (a área segura recusa o `OpenInputDesktop`) também conta como bloqueio. O módulo responde `Refused{Locked}`, esvazia a sessão e solta tudo, tanto na entrada quanto no meio da sessão (`module.rs:1198` e `:1283`). O `release_all` segue permitido: só solta.
- **Windows, origem:** `Notice::SessionLocked`, `SessionUnlocked` e `DesktopSwitched` (relógio de 500 ms) viram `SessionLocked(bool)`; o `CaptureCore` não ativa barreira bloqueado, e o engine fecha a sessão com `ReleaseAll` e depois `Leave` (`close_for_lock`), e recusa `Activated` e `switch` bloqueado.
- **Linux, destino:** `PortalEmulate` devolve `Locked` com o `LockedHint` ligado (`portal.rs:850`); **origem:** `PortalCapture` emite `SessionLocked` (inclusive lock já ligado na subida). Sessão do logind por `XDG_SESSION_ID`, depois por PID, depois pela sessão gráfica do usuário.
- **Aceito:** sem logind o Linux trata como desbloqueado (decisão da §13.6 item 8, com aviso no diagnóstico). Quem usa o ClipSync sem logind injeta na tela de bloqueio do destino se ela existir.

### `Returned`, `Refused`, `begin` e `can_begin`

- **`Returned` e `Refused` só valem para sessão do mesmo par.** O módulo só repassa quando há captura contínua; o engine exige `s.peer == from && s.id == session` nos dois (`capture.rs:545-548` e `:575-578`), e o loopback exige `c.to == from` mais a sessão igual (`module.rs:1068-1073`); senão `ignored += 1`. `Returned` é ignorado com a trava ligada. `pos` é validado no proto (finito, 0 a 1) antes de chegar lá, e o `k` ainda é preso a 0..1. O texto da recusa sai de `msg::refused_by` com o motivo da enumeração fixa, nunca do par.
- **Autorização no proto:** `EnterSpan` como `EnterAt` (`input_in` mais pausa), `Returned` e `Refused` sem `input_in` e `min_version` 3 (`module.rs:60,148-183`); para par v2 o módulo não manda `Refused` (`send_v3`). `EnterSpan` valida `pos`, `back_from < back_to` e o nome do monitor.
- **`can_begin`/`begin`:** o `switch` recusa trava, pausa, sessão bloqueada, par fora do layout, par offline e `can_begin()` falso (o portal do Linux só começa por travessia, então recusa); depois abre a sessão e chama `begin`; se o `begin` falhar a sessão fecha com `Leave`. No Windows o `begin` estaciona o cursor e leva só modificadores e botões; falha de estacionar desfaz a captura.

### Informativos novos

- **I-S7-13 · Win+L com captura ativa vai para os dois lados.** O Windows não deixa o hook segurar o Win+L, então a máquina de origem bloqueia e o destino também recebe Win e L e pode bloquear. O `SessionLocked` fecha a sessão logo depois. Efeito de uso, sem vazamento; para o roteiro do passo 7.
- **I-S7-14 · Janela entre `begin` e `ready`.** O hook lê `begin` e depois `ready` em duas leituras; se a thread do hook for suspensa por alguns milissegundos entre elas e o worker gravar os dois nesse intervalo, o cursor fica em "armando" (movimento bloqueado). Sai pelo Win+Alt+N do próprio dispositivo ou pela bandeja. Probabilidade desprezível; um único atômico com estado resolveria.
- **I-S7-15 · O falso `HooksLost` por recorte do `SetCursorPos`** (herdado do I-S7-3): se o centro estacionado cair fora da área virtual, o ponto não bate e o relógio de 500 ms acusa perda a cada captura; nos três primeiros o hook é recriado, no quarto o controle desliga com o aviso. O centro do monitor da barreira está sempre dentro, então só acontece com a área virtual mudando no meio da captura.

### Para o Maestro

| Item | Estado |
| --- | --- |
| §13.2 regra 2, hooks (a a g) | **Cumprida**, com S7-05 e S7-07 |
| `ffi.rs`, bloco a bloco, depois do aac844e | **Aprovado**; S7-01, S7-02, S7-03 e I-S7-11 fechados |
| `HooksLost` | **Aprovado** |
| Bloqueio de tela, Linux e Windows | **Aprovado** (resíduo aceito: Linux sem logind) |
| `Returned`/`Refused` só do mesmo par; `begin`/`can_begin` | **Aprovado** |
| Bloqueante | **Nenhum.** Critério 7: **PASSA** |
| Antes da tag | Corrigir **S7-05**; conferir o CI Windows 40757283200 e o resultado dos testes de `ffi.rs` |

### Reverificação das correções (6880b7f)

**Resultado: S7-05 Fechado, S7-06 Fechado, S7-07 Fechado com um resíduo novo (S7-08, BAIXO, Aberto).** Estado da S7 depois disto: **0 BLOQUEANTE · 0 MÉDIO · 1 BAIXO aberto (S7-08)**. O critério 7 segue **PASSA**. Revisado sobre o HEAD 6880b7f (diff `a38d25c..HEAD` de `capture.rs`, `wincore.rs`, `session.rs` e dos testes), com `conferir-unsafe.sh` e `conferir-privacidade.sh` passando, `cargo test -p clipsync-input --test wincore` verde no Linux (22 de 22, inclusive `coming_back_presses_only_what_is_physically_down`) e `cargo clippy -p clipsync-win -p clipsync-input --all-targets --target x86_64-pc-windows-gnu -- -D warnings` limpo (isso compila os testes `cfg(windows)` do `capture.rs`, mas não os executa).

| Item | Estado | Evidência |
| --- | --- | --- |
| **S7-05** conjunto de teclas velho | **Fechado** | `Shared::reset_input` zera as 5 palavras de `held` e as flags `begin` e `ready`, e é a primeira linha de `start_hooks`, que `start()` e `recreate_hooks` chamam (conferi os dois chamadores). O `end_locally` não lê mais o publicado do hook: aperta de novo `repress_synths(&reconcile(Held::default()))`, isto é, só os 8 modificadores e 5 botões que `modifier_down` e `button_down` dizem apertados. `repress_synths` é pura e testada. O teste do cenário (`a_new_hook_instance_forgets_the_keys_the_old_one_held`) monta o caso Win+Alt, pausa, retoma, cruzamento só com mouse, e confere `held` vazio e volta sem reaperto; ele chama o `reset_input` à mão (não dá para subir hook real num teste), então que o `start_hooks` o chama eu li, não vi rodar. Os testes de `capture.rs` só rodam no Windows. |
| **S7-06** `eprintln!` de depuração | **Fechado** | `grep` por `eprintln!` e `DEBUG` em `clipsync-input/src` e `clipsync-win/src` (fora do `probe/`) não acha nada; os dois `eprintln!` do `session.rs` saíram (c9a6579 e ad2b5f0). |
| **S7-07** thread do worker | **Fechado** (com o S7-08 abaixo) | Saiu o `Msg::Stop`. O worker lê `Shared.stop` a cada 100 ms (`recv_timeout`) e no laço do `emit`, que não bloqueia mais no canal cheio (`try_send` e espera de 2 ms checando a flag). `stop()` liga a flag, derruba os hooks e faz `join`; o `Drop` faz o mesmo e também larga o `CursorPark`. Conferi que ninguém junta a thread dela mesma (o worker só tem `Arc<Shared>`, não o `WinCapture`) e que o `Drop` solta a trava dos hooks antes do `join`. Teste `the_worker_leaves_by_the_flag_even_with_everything_full` (Windows). |

**S7-08 · BAIXO · Aberto · `stop()` pode deixar hooks vivos se um `HooksLost` estiver sendo tratado ao mesmo tempo** (`adapter/win/capture.rs`, `stop()` e `recreate_hooks`). O `stop()` liga a flag, tira os hooks da trava, e junta o worker; mas o `recreate_hooks` não confere a flag antes de `start_hooks`, e depois do `join` o `stop()` não olha a trava de novo. Se o worker já tinha tirado o hook morto e está prestes a criar o novo quando o `stop()` roda (pausa apertada no mesmo milissegundo de um `HooksLost`), o hook novo é gravado em `Shared.hooks` depois que o `stop()` passou, e fica instalado até o próximo `start()` ou `Drop`. É o mesmo desenho de antes (a corrida já existia com o `Msg::Stop`), só estreito, mas o efeito é um hook de teclado e mouse ocioso durante a pausa. Não executado: raciocinado sobre o código. **Correção, duas linhas:** `recreate_hooks` retorna sem criar se `shared.stop` estiver ligada, e o `stop()` tira `shared.hooks` mais uma vez depois do `join`.

**Ressalva do Flux sobre o reaperto (I-S7-16, informativo, não verificado).** O `end_locally` agora depende de o `GetAsyncKeyState` refletir a tecla fisicamente apertada. Pode ser menos do que parece: na ativação o próprio adaptador injeta o soltar de tudo o que estava apertado (`release_locally`), e entrada injetada também atualiza a tabela do sistema; e uma tecla que o hook bloqueou durante a captura pode nem ter entrado nela. Se for assim, o reaperto na volta sai vazio, e um Shift segurado de ida e volta não é restaurado. Isso falha para o lado seguro (nunca fica preso, que era o defeito do S7-05), mas pode não cumprir "pressiona de novo o que ainda está fisicamente seguro". Não tenho Windows para ver. O roteiro §13.12 passo 5 solta o Shift antes de voltar, então **não** cobre este caso: sugiro acrescentar ao passo "segurar Shift, atravessar, voltar sem soltar, digitar uma letra no PC: maiúscula ou minúscula, e nada preso depois de soltar".

### Estado do CI da base (8cc14c4, run 40757283200)

O job Linux passou. O job Windows **falhou**, e só em `clipsync-ui --test diag` (`diagnostics_show_the_keyboard_layout_and_on_windows_the_shortcuts`): o diagnóstico mostrou "atalhos ativos" sem ligação local, porque o item "atalhos" do próprio crate de input (texto fixo, sempre `ok`, `clipsync-input/src/diag.rs:101`) vinha antes do item que carrega o estado do agente. Não envolve hook nenhum e não é falha de segurança do que o produto faz; é o diagnóstico dizendo "ativo" quando os hooks não existem, para o lado errado de informar, e a correção está no diff **não commitado** de `diag.rs` e `tests/diag.rs` (`with_agent_state`, que troca os itens de mesmo nome). Li o diff e ele faz o que o teste novo pede, mas **não conferi a versão commitada nem um CI verde com ela**. No mesmo run, tudo o que é do `clipsync-win` passou no runner Windows: os 3 testes de callback do `ffi.rs` (pânico no teclado e no mouse passa o evento e reporta a perda uma vez; `Block` bloqueia), `hooks_and_watcher_start_and_stop_without_leaking`, `ten_thousand_desktop_checks_do_not_leak_handles`, `injected_input_never_reaches_the_callback`, `which_win_alt_hotkeys_the_system_accepts` (o `Win+Alt+P` registrou), `the_desktop_and_the_monitors_can_be_read`, `a_notice_clock_runs_and_stops`, e os dois scripts de CI (`unsafe isolado` e `privacidade dos hooks: ok (8 arquivos)`). Esses resultados são de antes das correções desta rodada; os testes novos de `capture.rs` ainda não rodaram no Windows.

**Informativo novo, I-S7-17.** `clipsync diagnose` (CLI) imprime o item "atalhos" do crate de input como `ok` no Windows sempre, descrevendo o que existe e não se os hooks estão de pé. Não afirma hook ativo, mas combina mal com a tela da UI depois da correção; vale reescrever o texto como "disponíveis" ou receber o estado do agente.
