# S9: delta técnico após revisão do Sentinel

Base inspecionada: `d41aa65`. Desenho adotado pelo Maestro após revisão Sentinel; implementação autorizada pelos itens 2/3/4 de João. Mantêm-se wire v3/mínimo 1, dependências, deadline total de 2.000 ms e limite de 256 eventos pendentes.

## 1. H-01: época local, autoridade vigente e commit da entrada

Um booleano não distingue `permitido -> revogado -> permitido`. Acrescentar **metadados locais de admissão**, não campos serializados:

```rust
// Nomes finais podem acompanhar a convenção do crate; sem Serialize/Deserialize.
struct ReceiveContext {
    connection: u64,       // geração LOCAL da conexão que recebeu a mensagem
    input_epoch: u64,      // época LOCAL de autorização no recebimento
    stream: Option<u64>,   // índice QUIC uni; None em datagrama
}
struct PeerInputContext {
    connection: u64,
    input_epoch: u64,
    input_in: bool,
    paused: bool,
    connected: bool,
}
```

- `ModuleInput::Reliable` e `Datagram` carregam `context: ReceiveContext`. Clipboard ignora o campo com `..`; mensagens wire e JSON não mudam.
- `ModuleHost::peer_input_context(peer) -> Option<PeerInputContext>` lê a autoridade atual sob o lock do host. Não usar `State.peers` para decidir autorização. `host.peers()` continua fonte da identidade/versão; `host.paused()` deve ser respeitado. A implementação conjunta do contexto precisa observar pausa e época de forma coerente.
- Core mantém `input_epoch` por slot; muda antes de publicar eventos quando `input_in` muda, pausa muda, conexão é substituída ou identidade é removida/reintroduzida. Atribuir identificador local novo, sem reutilizar o anterior. `set_paused` atualiza pausa/épocas sob o mesmo lock usado pelo snapshot. Atualização de nome não revoga uma entrada.
- Capturar conexão/época **ao aceitar o stream**, antes do `read_frame`/`deliver`; preservar até o consumidor. `uni_task`/`datagram_task` recebem a geração da conexão instalada. Não carimbar mensagem de conexão antiga com a geração da substituta. No datagrama, capturar sob a mesma leitura usada na autorização.
- MemHub reproduz a mesma fronteira: conexão lógica estável durante mudança de permissão; nova época em revogar/reautorizar/pausar; carimbo do destino capturado antes de aguardar a fila.
- `on_enter` valida conexão, época, permissão e pausa vigentes **antes de iniciar**, **depois de `emu.start().await`**, **antes e depois de `emu.enter/emit(...).await`**. Só instala `Incoming` ativo após a última validação. Se perdeu validade, solta o que tiver sido emulado, não instala sessão e notifica recusa.
- `Incoming` guarda conexão/época de admissão. `on_datagram` exige que contexto, sessão e autoridade atual coincidam; depois de um `emit().await`, conferir de novo e limpar se mudou. Não continuar uma sessão só porque `input_in` voltou a true.
- `PeerUp` dispara reconciliação com o host atual, e não uma aplicação cega do booleano contido no evento: o evento pode estar atrasado em relação a uma nova concessão. Remove sessão de época antiga e executa `release_all` antes de rede. Não derruba sessão de saída por permissão de entrada.

Limite preciso: época local identifica mensagens aceitas/enfileiradas localmente antes da revogação. Sem campo wire, não identifica quando o remetente criou um stream que só apareceu no receptor depois da nova concessão. Tampouco promete desfazer movimento síncrono que coincidiu com uma revogação; garante que a conclusão assíncrona inválida não deixa uma sessão/pressionados ativos nem libera os próximos eventos. Testes precisam bloquear `start` e `enter` separadamente, revogar/reautorizar durante a espera e então concluir a operação.

## 2. Entrada entregue antes do cancelamento e ordem entre streams

Reset não desfaz um Enter completo. Cada tentativa conserva `{attempt_id, peer, connection, session}` até concluir a limpeza, inclusive se o envio retornar erro ou for cancelado.

- Acrescentar `ModuleHost::send_reliable_on(to, connection, msg)` para o caminho de input: mesma autorização/prazo de `send_reliable`, mas só usa a geração de conexão indicada. Não enviar encerramento/recusa antigos na conexão de uma reconexão. A API atual permanece para os outros consumidores.
- Cancelar invalida `attempt_id`, retira sessão/fila e libera localmente. Depois aborta e **aguarda** a tarefa de abertura. Resultado já concluído é descartado pelo identificador da tentativa; nunca publica Active nem resolve Switch com sucesso.
- Se a conexão da tentativa ainda existe, emitir `Leave { session }` limitado mesmo quando não se sabe se Enter chegou. Leave é seguro para sessão desconhecida e solta tudo quando a sessão existia. Na saída por bloqueio, tentar também ReleaseAll dentro do orçamento do trabalho de encerramento; priorizar Leave, que já encerra e solta no destino.
- Encerramento de uma tentativa precede a próxima abertura para aquele par. Não abrir novo stream de Enter enquanto o envio de encerramento anterior ainda pode criar seu stream. `open_uni` cancelado/encerrado é aguardado antes de iniciar Leave.

**Proteção do receptor sem tombstones ilimitados:** aproveitar a ordem de criação dos streams QUIC, já disponível por `RecvStream::id().index()`. Por par/conexão, guardar `closed_through` e `latest_enter`, ambos opcionais e monotônicos:

1. Ao receber Leave, avançar `closed_through` para seu índice, **mesmo sem Incoming daquele session**. Só fechar Incoming se session corresponde e a entrada foi criada em stream anterior ao Leave.
2. Rejeitar Enter cujo índice seja `<= closed_through` ou `<= latest_enter`. Isso também impede Enter antigo de substituir uma entrada mais nova que chegou primeiro.
3. Só comparar índices da mesma conexão; mensagem de conexão substituída é rejeitada. Troca de época na mesma conexão não zera as fronteiras. Nova conexão inicia fronteiras próprias.
4. ReleaseAll não avança a fronteira: pelo contrato existente, solta pressionados sem equivaler a Leave.

Não depende de entrega FIFO nem guarda um ID por sessão para sempre. O source atual já cria Enter antes de Leave; o source corrigido mantém essa ordem mesmo no cancelamento. Reordenação de processamento não muda os índices.

Compatibilidade: receptor atualizado aplica a fronteira também a wire v1/v2/v3. Receptor antigo não conhece essa defesa; Leave ainda é enviado, mas **não há garantia de impedir Enter tardio no receptor antigo**. Não é possível detectar patch S9 por `remote_version`, que continua 3; a garantia completa exige ambos os executáveis atualizados, sem inventar versão remota.

## 3. Fila de input e fronteira de captura

- A fila de 256 armazena **somente Input da tentativa atual**, nunca Ctl nem eventos de ciclo de vida. O laço continua consumindo rx/ctl enquanto a rede espera; sinais têm prioridade sobre resultado de abertura e reprodução da fila. Reproduzir lotes limitados, conferindo sinais novamente entre lotes.
- Cancelar em: Paused(true), Stop, Lock(true), SessionLocked(true), PeerDown do alvo, Home, Refused/Rejected do alvo, Retarget que altera/remove a barreira da tentativa, Deactivated, Error e EOF do adaptador. Retarget deve invalidar antes de instalar outra geometria. Nova ativação não herda Switch pendente.
- Overflow cancela, libera e emite texto fixo. Switch cancelado resolve `Err`, inclusive substituição por outra tentativa e shutdown; grace timer nunca converte cancelamento em sucesso.
- **Não basta `try_recv` uma vez.** Na invalidação, retirar o rx do laço normal e invalidar sua fila; parar e aguardar o produtor com `cap.stop()`, consumir apenas os sinais de ciclo de vida ainda presentes nesse rx e então descartá-lo, antes de aceitar outro receptor. Input e Activated antigos nunca são reaproveitados. Se o estado permite continuar, `cap.start()` cria rx novo. Linux preserva a sessão de portal, mas desabilita o produtor anterior; Windows espera o worker anterior. Pausa/Stop/trava manual não rearmam automaticamente enquanto ativos. SessionLocked do SO tem a exceção do adendo 9 para conservar observação de unlock, sempre com gate de captura fechado. Essa fronteira usa a API existente e evita acrescentar generation a cada evento dos hooks nesta fatia.
- Enquanto o adaptador é parado/rearmado, não existe tentativa nova válida. Não aceitar resultado de rede antigo nesse intervalo. Falha ao parar/rearmar deixa captura indisponível, com erro, em vez de reutilizar receptor duvidoso.
- Loopback usa a mesma máquina de abertura/cancelamento e descarta rx antes de devolver o adaptador. Ao encerrar, `cap.stop()` e resposta ao comando não esperam sucesso de rede. O shutdown atual que apenas aborta a tarefa de loopback deve mudar para sinalizar, liberar, aguardar retorno do adaptador e só abortar com cleanup definido.

## 4. Dono e limite das tarefas de rede

- Implementar helper interno `input/src/network.rs` para envios com conexão fixada, timeout e ownership. É posse adicional do backend.
- Contínuo/loopback possuem seus handles: no máximo uma abertura e um encerramento pendentes por controlador; a próxima abertura espera a ordem do encerramento. Encerramento tem orçamento total próprio de 2 s. Cancelar/join da abertura ocorre depois da liberação local e antes de abrir o stream de Leave.
- `State` possui `JoinSet` limitado a 32 trabalhos para respostas/anúncios. Migrar `send_v3`, Stats e Zones para esse dono; nenhuma chamada nova a spawn descartando o handle. LayoutOffer e bridges de Switch/hotkey também precisam sair de awaits de rede no laço e ter proprietário/resultado no shutdown. Zones repetidas podem ser coalescidas; comando novo sem capacidade retorna erro fixo. Aviso best-effort sem capacidade é registrado como falha, sem reverter limpeza local. A vaga de limpeza do controlador não disputa essas vagas.
- Saída normal: invalidar tentativas, liberar captura e emulação local, resolver respostas pendentes; permitir limpeza limitada; `abort_all` no restante e drenar `join_next`. Não abortar o dono antes de recuperar o adaptador. Drop dos conjuntos aborta filhos se o core cancelar o módulo; encerramento normal faz join explícito.
- Em `core::uni_task`, substituir spawn destacado por conjunto de até 32 filhos da conexão; ao atingir o limite, esperar conclusão ou fechamento antes de aceitar outro stream. Cancelar/join ao fechar conexão. O envio de Frame::Rejected usa o mesmo deadline/reset. Não estender esta fatia a refatoração geral de dialers/runtime do mesh.
- `send_uni` mantém deadline único desde antes de open_uni; guarda reseta stream em erro/cancelamento antes do finish. Drop normal de SendStream não serve: Quinn tenta finish do prefixo. Não tratar finish como confirmação de entrega.

**Resposta antiga preservada:** adicionar método local `ModuleHost::reject_input_on(to, connection, reason)` que envia o `Frame::Rejected { kind: Input, ref_id: None, reason }` já existente. Core e MemHub implementam. H-01 usa Refused(session) em v3; v1/v2 recebem Rejected, sem suprimir a resposta que já entendem. O Rejected antigo não correlaciona sessão e pode cancelar conservadoramente a tentativa atual daquele par; não se atribui precisão que o wire antigo não tem. A recusa já feita no transporte continua funcionando em todas as versões.

## 5. Posse/API a ampliar no contrato e plano

- `clipsync-proto/src/module.rs`: tipos locais, campos de ModuleInput e três métodos locais de ModuleHost acima; não apenas documentação. `src/testing.rs`: épocas, conexão/ordem lógica, envios fixados e Rejected do MemHub. `src/lib.rs` somente se necessário para reexportar os tipos.
- `clipsync-core/src/transport.rs`: épocas/snapshot, carimbo no ingresso, geração das tarefas, envios fixados/rejeição, deadline/reset e dono dos leitores de streams.
- `clipsync-input/src/{module,capture,msg}.rs`, novo `network.rs` e registro em `lib.rs`: reconciliação, fronteiras, máquinas de tentativa/cancelamento, rede possuída e reset do receptor. Sem necessidade prevista de alterar adaptadores reais ou ffi. Incluir `adapter/fake.rs` na posse caso seja necessário modelar fielmente a parada do produtor e o receptor novo nos testes, sem API nova de hook.
- `clipsync-clipboard/src/module.rs`: apenas padrões `Reliable { ..., .. }` para aceitar metadados locais; nenhuma política de clipboard nesta fatia.
- Testes atuais que constroem/desestruturam ModuleInput: ajustar os padrões/fixtures nos crates proto/core/input/clipboard. A busca atual retorna 26 ocorrências entre produção e testes; revisão deve conferir todas. Acrescentar testes focados já previstos em core/input e testes do MemHub. Sem Cargo/lock/wire/JSON/capability.
- Retirar do plano a restrição de manter ModuleInput/API interna inalterados: ela conflita com a prova da época de autorização. `send_reliable` existente mantém assinatura; os métodos adicionais atendem a fixação de conexão e a recusa antiga.

## 6. RED obrigatório antes da implementação

1. Enter aceito antes de revogar/reautorizar, mas consumido depois; start bloqueado e enter bloqueado, com transição em cada await; conexão substituída; datagrama de época antiga. Reautorizar + Enter novo funciona. PeerUp atrasado não derruba sessão válida de época nova. Tecla/botão liberados; saída local permanece independente.
2. Permissões e pausa do host mudam antes de chegar PeerUp/Paused; mensagens antigas não atuam. v1/v2 recebem Rejected; v3 recebe Refused com session correto.
3. Entregar Leave(stream 2) antes de Enter(stream 1); entregar Enter(stream 3) antes de Enter(stream 1); Leave antigo não fecha sessão nova. Fronteiras persistem pela reautorização, isoladas por par/conexão.
4. Host entrega Enter e retém a conclusão; cancelar; conclusão tardia não ativa; Leave sai mesmo assim; destino não ressuscita após reordenação. Repetir com reconexão para garantir que cleanup não vai à conexão nova.
5. Todas as causas de cancelamento enumeradas, Switch pendente e timer de grace. Fila cheia não esconde sinal; Input e Activated antigos no rx anterior não passam após rearmar. Overflow não reenvia dados em tentativa posterior.
6. Prazo QUIC com crédito esgotado e escrita parcial/reset; cancelamento externo depois de prefixo. Jobs bloqueados de Refused/Returned/Stats/Leave terminam no desligamento; contadores de tarefas voltam a zero. Captura é liberada antes de liberar o fake que mantém o envio bloqueado.

Validação física, incidente reservado ao TESTES e defeitos de EIS/posicionamento continuam fora deste delta.

## 7. Adendo vinculante: admissão e progresso sob saturação

O conjunto de leitores de `uni_task` tem no máximo 32 filhos por conexão. Cada filho tem prazo TOTAL de 2.000 ms contado ao aceitar o stream, cobrindo leitura completa de `read_frame`, autorização e entrega à fila do módulo. Esperas de resposta/rejeição não renovam esse orçamento. Enquanto os 32 estão ocupados, não criar filhos ou tarefas de entrega adicionais; o laço continua observando término, cancelamento e encerramento da conexão.

Se o orçamento expirar em leitura OU entrega, cancelar o trabalho, resetar/parar o stream aplicável e FECHAR a conexão com razão fixa sem payload. Invalidar imediatamente a geração/autoridade no host e propagar PeerDown pelo caminho de desconexão; cancelar e aguardar os demais filhos. Não descartar silenciosamente uma mensagem Leave e continuar como se a conexão estivesse saudável. Essa política também vale para filas cheias de outros módulos: o prazo limita a retenção de recursos, não cria autorização alternativa.

Regressões determinísticas: saturar 32 leitores com frames incompletos; saturar 32 entregas com fila de módulo bloqueada. Em ambos, comprovar recuperação ou fechamento/PeerDown em até 2.000 ms mais tolerância explícita de agendamento, limpeza das tarefas e soltura local independente. Usar relógio controlado quando disponível; se tempo real for necessário, registrar tolerância. Não inferir desempenho de rede física desses testes.

## 8. Ajustes de integração autorizados
- Posse adicional backend: input/src/adapter/fake.rs, somente para stop aguardar produtor e não reaplicar fila de input/ativação da geração antiga; preservar sinais de ciclo de vida. Não alterar adaptadores físicos nesta extensão. Teste que depende da nova fronteira fake deve explicar a equivalência/limites com a API dos adaptadores, sem usar alteração do fake para esconder defeito de produção.
- Abertura/encerramento podem ser futures diretamente possuídas pelo laço em vez de tarefas spawnadas. Drop deve cancelar com cleanup/reset RAII; não há join de tarefa que não foi criada. Onde houver spawn, cancelamento e join continuam obrigatórios.
- Dependência C01 atribuída ao mesmo dono backend: ignorar ToggleLock/GoTo atrasados enquanto paused, preservando TogglePause para o core. O core (Rivet) recusa Resume sob reserva de configuração. Incluir regressão focal de atalho atrasado; não retomar captura nem alterar trava após pausa.

## 9. Decisões de integração após regressões
- A bridge LOCAL de Switch/hotkey admite 5 s totais para encerramento anterior (2 s), abertura (2 s) e grace (400 ms). Envios/replies/cleanup QUIC conservam 2 s cada. Controle/ciclo de vida continua cancelando imediatamente; timeout/drop da bridge invalida tentativa e nunca permite ativação sem resposta válida. O timeout de resposta do controle local existente é120 s; connect2 s não governa essa espera.
- O produtor dos adaptadores também observa bloqueio/desbloqueio do SO. Parar e proibir toda reativação após SessionLocked elimina a fonte do próprio unlock (regressão S7 reproduzida). Para ESSE caso, depois de stop/join e descarte do rx velho, criar produtor/rx novo mantendo session_locked=true: consumir ciclo de vida e descartar qualquer Input/Activated, sem Begin nem sessão de saída até unlock. Revalidar host/pausa/trava antes de reabrir. Isso não autoriza captura durante pausa global, Stop ou trava manual; não acrescenta diálogo de consentimento nem contorna SO.
- Aceites: lock solta imediatamente, Input/Activated enfileirados e do novo produtor durante lock não são enviados, unlock permite somente ativação nova; pausa/Stop prevalecem sobre unlock. Em Linux, stop mantém sessão do portal existente mas encerra produtor anterior; não apresentar fake como prova física. Não mudar fake para esconder falta do watcher real.

## 10. Snapshot inicial e fallback de autorização
Posse backend ampliada minimamente a adapter/session.rs e adapter/win/capture.rs: watcher entrega snapshot inicial tanto true quanto false. Linux não pode emitir somente true, pois unlock durante pausa ocorreu sem produtor; Windows consulta a API segura já existente clipsync_win::input_desktop_is_default, sem alterar ffi/unsafe/deps. Falha/estado desconhecido mantém gate fechado e relata motivo fixo; nunca inicializar false por conveniência. Confirmar ordem antes de ativação, preservar geração e impedir reenvio de dados antigos.

Aceite focal: lock -> pausa global/trava manual -> unlock com produtor parado -> retomada -> snapshot novo desbloqueado -> somente ativação nova; snapshot bloqueado ou consulta falha conserva gate, Stop prevalece. Teste fake de política não prova consulta Windows/DBus real. Adaptadores físicos passam por leitura/cross-clippy e prova manual posterior.

H01 ganhou regressão em que PeerUp fica retido e não há tráfego posterior. Além da reconciliação imediata no evento e dos checks em cada entrada, on_tick confere autoridade vigente a cada2 s e solta Incoming inválido antes de rede. Declarar limite do fallback, não afirmar soltura física instantânea quando evento foi retido.

Consistência do adendo10: backend pode ajustar somente o texto de input/src/diag.rs que anunciava fallback desbloqueado; agora falha de consulta conserva indisponibilidade/bloqueio. O teste antigo fail-open deve refletir a política fechada, sem suprimir teste de sucesso explícito.

## 11. Correção de semântica Windows após revisão
O texto anterior sobre consulta de desktop não equivale a WTS unlocked. Contrato§15.9 prevalece: nova consulta WTS segura no ffi (feature existente), mantendo estado de sessão separado do desktop; validar/liberar buffer e falhar fechado. Backend tem posse mínima win/lib.rs+ffi.rs e portal.rs apenas ordem watcher antes do produtor. Fonte primária: Microsoft WTSINFOEX_LEVEL1_W e WTSQuerySessionInformationW, consultadas pelo implementador; Win7 com flags invertidos é fora do suporte Win10+.
