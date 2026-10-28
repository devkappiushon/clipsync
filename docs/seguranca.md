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
