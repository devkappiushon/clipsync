# Complemento SEG S9: cinco causas confirmadas

29/10/2026. Arquiteto /root/contrato. Desenho registrado pelo Maestro; B09 liberado após aprovação Sentinel. B01/B03/B05/B07 aguardam atribuição de posse e integração conforme adendos abaixo. Fonte: /tmp/clipsync-s9-seg-cobertura.md e código atual dos cinco pontos. Não é nova auditoria nem evidência de correção. Nenhum código, build, teste, runtime ou perfil real foi alterado nesta entrega.

## B-09: pausa volátil e erro da bandeja — liberar Rivet primeiro

**Fonte atual:** ui/clipboard.rs::set_mode grava Profile::set_clipboard_mode e chama ui/agent.rs::restart; restart ainda faz stop/start sem confirmar persistência da pausa. app/tray.rs::toggle_pause só registra tracing::warn no erro. C01 já tem ConfigEdit, Effects/Observed, reserve_operation, retain_operation, Startup::with_listener e start_reserved.

**Decisão:** compartilhar a transação já implementada de C01 com a edição de modo de clipboard, sem criar outro mecanismo de restart. Extrair helper privado limitado a duas edições tipadas (Modules e ClipboardMode), mantendo validações específicas de cada comando. Não usar callback genérico de arquivos nem restaurar snapshot antigo do perfil.

- Mesmo modo já configurado: no-op, sem pausa/gravação/restart.
- In_app: adquirir exclusão e ConfigEdit; observar posse; Pause precisa ser confirmada E persistida ANTES da escrita do modo; ler InputState depois da pausa; reter guard no Running; parar com join confirmado; reservar listener; salvar o modo; iniciar com Startup::with_listener/start_reserved e trava anterior. Volta pausado, exigindo Retomar explícito. Nenhuma escrita paused=false pela transação.
- Pausa volátil anterior: repetir Pause para tentar persistir o estado atual. Se falhar, não salvar o modo nem parar/reiniciar. Relatar a pausa que já vale em memória, a persistência falha e config_saved=false. Se a persistência for restabelecida, seguir e continuar pausado depois do restart.
- Parado comprovado: gravar e continuar parado. Unknown/timeout: recusar. External: preservar o contrato existente de modo, configuração salva para próxima inicialização e restart_needed=true; não enviar Pause, Stop, Lock ou Resume ao externo. O novo modo não é apresentado como aplicado naquele processo. Esse caminho config-only é específico de clipboard_set_mode; modules_set continua recusando externo.
- ConfigEdit ganha operação reservada set_clipboard_mode(mode), usando a mesma leitura validada/lock e verificação de conflito integral. Profile::set_clipboard_mode segue wrapper do mesmo escritor, não uma segunda leitura/escrita dentro da reserva.
- Falhas de pausa, parada, commit, startup e trava recebem Effects/Observed do C01, inclusive antes de salvar. Não usar restart como sinônimo de stop/start quando ele não preserva essas condições. O helper legado deve deixar de ser usado nesse caminho.
- ModeReply conserva mode e restart_needed; acrescentar effects/observed opcionais com serde(default). Front mostra a configuração salva, a pausa/necessidade de restart e qualquer resultado parcial. Mensagem explícita: "Modo salvo. O ClipSync continua pausado; clique em Retomar quando quiser." Sem retomada escondida.

**Erro acionado pela bandeja:** em pause_set, guardar em Ctx um aviso de pausa e emitir resync quando a operação falhar; oferecer app_state.pause_warning: Option<String> com serde(default). Texto específico para STATE_NOT_SAVED: o estado foi alterado em memória, mas não sobreviverá ao encerramento; para comunicação incerta, dizer que a pausa não foi confirmada. Não supor que todo erro significa paused=true. Limpar o aviso somente após confirmação de persistência de uma ação de pausa/retomada ou da pausa preparatória do C01, nunca por mero poll bem-sucedido.

A bandeja, no erro, reconsulta app_state para sincronizar seu rótulo, abre a janela e deixa a UI apresentar esse aviso persistente. O aviso vem de app_state, então não se perde se o evento foi emitido antes de a janela ouvir. Notificação do sistema é opcional e respeita preferências; não substitui esse feedback. Não habilitar capability nova. Front não usa innerHTML nem detalhes sensíveis.

**Posse:** Rivet: ui/{modules,agent,clipboard,status,lib,api,msg}.rs, core/profile.rs no método de edição, respectivos testes. Coordenar app/{tray,events,msg}.rs e ui/js/{app,estado}.js com seus donos; não editar simultaneamente. Um dono mantém o schema de Effects/Observed. A11y preserva foco/anúncio único do aviso.

**Aceites:** perfil fake com state.toml anterior false/ausente, config gravável e falha controlada de escrita de estado; Pause aplicada em memória -> troca de modo recusada, sem Stop/Start, modo anterior intacto. Reparar persistência -> troca/restart termina pausado, trava preservada; pausa concorrente nunca é desfeita. No-op, externo, ausência e Unknown mantêm as condições acima. Bandeja recebe STATE_NOT_SAVED -> rótulo sincronizado + aviso visível ao abrir a janela, mesmo sem notificações do SO. Falhas das demais etapas preservam relato parcial e posse da tarefa.

## B-01: retenção e agrupamento de negativas

**Fonte:** core/transport.rs::push_block adiciona uma entrada por ocorrência e trunca para MAX_RECENT_BLOCKS=20; ui/events.rs::new_blocks usa fresh por igualdade do registro inteiro e cria uma notificação por entrada; app/events.rs::notify mostra todos os candidatos permitidos.

**Decisão:** o anel guarda até 20 grupos distintos por (peer, module, reason), e não 20 repetições. Nomes são só apresentação; nunca fazem parte da chave. Repetição atualiza contagem e último instante no grupo, sem consumir outro slot. Registrar primeiro instante e último instante; expulsão é de grupos conforme política determinística, com memória limitada. Não prometer conservar diversidade ilimitada diante de mais de 20 causas distintas.

- BlockRecord mantém at_unix_ms como primeiro instante do grupo e ganha occurrences: u64 com serde(default=one), last_unix_ms: u64 com serde(default), normalizando o último para at quando faltar. São campos do controle JSON local, sem variante/versão nova no fio QUIC. Contagem satura em u64::MAX e isso significa "pelo menos" no extremo, nunca wrap.
- Grupo repetido não expulsa outro grupo por si; uma causa nova pode substituir a menos recente. Reordenação e empate seguem regra explícita e testada. Chaves usam os motivos locais já produzidos; não armazenar conteúdo de payload/tecla/clipboard.
- UI compara chave + início + contagem, não igualdade total com fresh: aumentar contagem não faz o anel inteiro parecer novo. Reinício/recriação de grupo trata contagem reiniciada sem subtração negativa. Diagnóstico leva contagem/última ocorrência e apresenta repetições; não infere taxa física de notificações a partir disso.
- Agregador de notificações no poll: primeira negativa de uma janela gera aviso; novas ocorrências acumulam por chave. No máximo um aviso de bloqueios a cada 30 segundos, relógio monotônico, com resumo das repetições e quantidade de causas/pares distintos ocorridos no intervalo. Não multiplicar aviso por peer_blocked. Política de os_notifications/block continua obedecida.
- A casca aplica a mesma cadência máxima para kind=block como proteção contra candidato duplicado/resync/agente antigo. Preferir suprimir candidatos repetidos, sem perder o histórico em Diagnóstico. Remoção real de dispositivo e outros kinds seguem política própria; não agrupá-los como negativa genérica.

**Posse:** dono backend atual em core/transport.rs e proto/control.rs; Rivet/guardião de UI em ui/{events,diag,msg}.rs; dono da casca em app/{events,state}.rs. UI do Diagnóstico consome contagem quando disponível; defaults preservam leitura de agente antigo.

**Aceites:** mil negativas iguais ocupam um grupo com count=1000; inserir causas A/B e repetir A não elimina B; 21 causas respeitam limite/política; contador extremo não volta a zero. Snapshot repetido não notifica; count aumentado gera delta do grupo, sem 20 eventos falsos. Relógio fake cobre janela/primeiro/resumo, polling vazio e preferência desligada. Confirmar só quantidade de candidatos/chamadas ao notificante fake; entrega real do daemon Windows/Linux é manual.

## B-03: política explícita de relógio e ordenação de clipboard

**Fonte:** clipboard/module.rs::new_offer usa +=1; on_offer atualiza lamport_max antes de pausa/TTL/ordenação; apply só muda last depois de Written. proto/clip.rs mantém lamport u64 e a validação atual não limita seu valor. Saturar o contador deixaria empates repetidos e não define o que fazer com oferta remota extrema.

**Decisão de contrato:** manter ordenação lexicográfica (lamport, PeerId) e o contador por instância, com admissão limitada de avanço remoto e incremento local verificado. Não mudar formato de Offer/Ack, enum de rejeição ou dependência.

1. MAX_REMOTE_AHEAD = 1_000_000 passos por oferta em relação ao maior contador local já admitido. É tolerância de relógio lógico, não milissegundos nem taxa de rede. Oferta com lamport=0, lamport=u64::MAX ou salto maior é Invalid usando RejectReason existente. Comparação usa subtração verificada/saturante apenas para medir diferença, nunca para produzir novo timestamp.
2. Validar conteúdo/tamanho, pausa, TTL e duplicação ANTES de incorporar relógio. Oferta inválida, duplicada, expirada ou recebida pausado não avança lamport_max nem last. Rejeição por relógio não aplica nem armazena conteúdo na caixa; diagnóstico usa texto fixo sem conteúdo.
3. Oferta admitida atualiza lamport_max=max(local, remoto). Preservar last como maior par realmente aplicado/copied localmente; auto-aplicação só avança last após Written. InInbox, conflito de escrita ou falha não fingem aplicação. ClipApply explícito pode aplicar item stale como hoje, mas last nunca retrocede.
4. new_offer passa a devolver Result e usa checked_add(1), reservando MAX como sentinela também no emissor. Se não houver próximo u64 OU o próximo for MAX, não enviar, não usar 0 nem repetir MAX; registrar bloqueio fixo e devolver erro nas ações explícitas. Captura local não derruba o laço nem impede pause/revoke/diagnóstico. Não reiniciar/resetar contador automaticamente.
5. A janela evita que uma única oferta extrema envenene a instância. Um par legítimo cuja instância esteja mais de 1.000.000 passos adiante também será recusado: essa é uma restrição deliberada e precisa constar no contrato/diagnóstico, não ser chamada de falha de rede. A política não afirma tolerância a par autorizado malicioso, que já pode propor conteúdo; apenas limita salto e garante comportamento definido.
6. Desempate por PeerId continua igual entre máquinas atualizadas, inclusive timestamp igual. Explicitar que o contador não é persistido hoje; reinício não recebe garantia nova de ordenação global. Todos os pontos que criam oferta (cópia e ClipSend/ClipSendCurrent) tratam Result.

**Posse:** dono backend atual em clipboard/{module,msg}.rs e testes de módulo; helper puro clock.rs opcional no mesmo crate. proto/clip.rs só documentação se necessária, sem nova validação de wire que altere compatibilidade por surpresa.

**Aceites:** relógios 0/MAX/janela+1 rejeitados sem mexer current/last/clock/caixa; limite exato admitido; oferta pausada/expirada/duplicada extrema não contamina a próxima oferta local. Copiar depois de remoto válido produz timestamp maior. Forçar relógio local MAX em teste -> erro explícito, nenhum wrap/oferta, módulo continua respondendo. Testar dois PeerId com mesmo timestamp, ordem invertida de entrega, falha/conflict de escrita, modo Manual e ClipApply stale para provar a política de last. Não testar só ausência de panic.

## B-05: pendências conservadoras de soltura no Windows

**Fonte:** input/adapter/win/emulate.rs::emit remove ups dos sets antes de send; release_all drena os sets antes da confirmação. clipsync-win/ffi.rs::send devolve Err quando SendInput inseriu menos que o lote, sem revelar qual parte teve efeito.

**Decisão:** manter ffi e Result atuais. keys/buttons passam a representar "possivelmente pressionados", nunca uma afirmação sobre quais eventos de um lote parcial foram injetados.

- Preparar desired (estado se TODO o lote funcionar) e possible = estado anterior UNION todas as teclas/botões down tentados no lote. Não remover pendência anterior só por ter tentado up. Manter ordem do lote para desired, sem logs/Debug dos conjuntos.
- Se send confirma todo o lote: commit desired. Se Err, inclusive parcial: guardar possible e marcar cleanup_pending. Exemplo down+up do mesmo código em lote falho conserva o código como pendente; KeyUp falho não apaga o anterior.
- release_all constrói somente ups a partir de cópia dos conjuntos, sem drain. Só limpa conjuntos/cleanup_pending após confirmação completa. Falha conserva TODAS as pendências para outra tentativa; não adivinhar prefixo bem-sucedido.
- Antes de aceitar novo emit/enter com cleanup_pending, fazer uma tentativa de release_all; se ainda falhar, recusar a nova injeção. Nenhum replay de movimentos/downs do lote falho. Resto fracionário de movimento/scroll não é prova de posição real; em erro resetar o resto em vez de repetir efeito presumido.
- stop tenta release e devolve seu erro; não declara soltura física concluída. Enquanto a instância existir pode haver nova tentativa; se o processo/adaptador for destruído, não prometer recuperação futura das pendências em memória. Sem loop infinito, nova captura automática ou persistência de teclas em disco.
- B05 não altera a política de desktop seguro, grants ou liberação por H01/B02. Um up repetido é medida conservadora; não afirmar que Windows o aceitou, nem que se sabe se o down existiu.

**Posse:** input/adapter/win/emulate.rs e testes puros do bookkeeping ou Windows com send falso. Não editar win/ffi.rs. Se extrair helper compilável no Linux, ele só modela conjuntos/commit, sem FFI ou informações de tecla em diagnóstico.

**Aceites:** down anterior + up falho mantém pendência; down+up e dois códigos em lote parcialmente falho conservam união; primeira release falha e segunda sucesso envia ups novamente e só então zera; enter/emit ficam barrados até limpeza; sucesso integral mantém sequência normal. Prova fake demonstra bookkeeping, não SendInput real, desktop seguro/UIPI ou soltura física.

## B-07: ciclo de vida separado da fila descartável

**Fonte:** capture.rs do adaptador Windows mistura Msg::Notice/Out na sync_channel; try_send pode perder Notice; overflow drena ambos. Worker::emit espera canal cheio e só observa stop, atrasando avisos. A correção B02 do consumidor preserva avisos que chegaram, mas não recupera os já perdidos antes.

**Decisão:** uma caixa de estado de ciclo de vida de tamanho fixo, independente da fila de dados, com invalidação monotônica. Não resolver com fila ilimitada nem callback bloqueante.

- Callback de Notice grava apenas atomics/latches: houve invalidação; estado de sessão mais recente; desktop precisa de conferência; displays mudaram; hooks perdidos. Lock/DesktopSwitched/HooksLost fecham a captura local imediatamente pelos flags existentes. Não executar consulta de monitor/SendInput nem esperar lock/fila no callback.
- Manter marca "houve lock/invalidação" separada de "estado atual unlocked": Lock seguido de Unlock antes do consumo ainda invalida a sessão antiga. Repetições coalescem; não somar fila sem limite. Estado desconhecido de desktop permanece fechado até consulta válida.
- Worker processa esses latches antes de dados e também em timeout. Ao esperar saída cheia, emit_data verifica invalidation/stop e abandona dado antigo para reconciliar ciclo de vida. Overflow descarta somente Out antigos; nunca apaga latches.
- Para o módulo observar ciclo de vida mesmo que o canal de Input esteja cheio ou substituído, acrescentar ao contrato INTERNO CaptureAdapter uma fonte opcional de snapshot/watch, com default None nos outros adaptadores: lifecycle_receiver() -> Option<watch::Receiver<CaptureLifecycle>>. Sem novo campo do protocolo.
- CaptureLifecycle contém revisão/invalidation_generation, session_locked conhecido/conservador, zones atualizadas e erro terminal fixo quando houver. Não contém InputEvent/tecla. O watch é atualizado pelo worker, fora do callback, e existe por instância do adaptador, não por sessão de captura. Último estado substitui o anterior, mas o contador de invalidações conserva o fato intermediário.
- Consumidores continuous/loopback dão prioridade à mudança de lifecycle antes de dados. Ao avançar invalidation_generation, encerram sessão/limpam batch e descartam o receptor da captura antiga pelo caminho B02; depois reconciliam o estado atual. Um Unlock não reabilita dados antigos; só nova captura/sessão pode enviar.
- stop/rotina de troca não reseta o snapshot antes de o consumidor reconciliar. HooksLost continua usando RestartPolicy existente; stop prevalece e não instala hooks novos. Conferência de desktop e zonas pode falhar: fechar e informar erro, sem fabricar estado unlocked. Nova geração de hooks não reutiliza held/ativação antigos.

Essa pequena API interna exige coordenação com o dono B02 de capture.rs/module.rs; publicar schema antes de editar. Se ele já tiver canal de lifecycle equivalente, usar esse canal em vez de criar outro. O indispensável é saída independente do receptor de dados e retenção de invalidações, não o nome do método.

**Posse:** input/adapter/{mod.rs,win/capture.rs}; helper puro de mailbox; input/{capture,module}.rs e fake conforme a fronteira B02 já em edição. Sem mudanças em win/ffi.rs nem em protótipos QUIC.

**Aceites:** preencher fila de dados, registrar cada Notice e drenar overflow -> lock/display/hooks continuam observáveis; Lock+Unlock coalescidos encerram a sessão antiga mesmo com estado final desbloqueado; saída cheia não impede o worker de fechar localmente/publicar lifecycle; troca de rx não remove a invalidação; nenhum Input/Activated anterior é reenviado depois dela; Stop+HooksLost não recria hooks; memória permanece limitada. Ensaios de mailbox/fake e CI Windows não provam comportamento físico de hooks/desktop.

## Integração, invariantes e limites

Ordem: B09 com Rivet agora; publicar schema de B01 e canal B07 antes de tocar arquivos compartilhados; B03/B05 independentes, com TDD do comportamento e revisão de segurança. Registro de contrato precede implementação; este arquivo não concede posse por cima do editor atual.

Preservar protocolo v3, enums postcard, grants explícitos, falha fechada, limites B02, autoridade H01, contagem de evento sem conteúdo, unsafe só ffi.rs e nenhuma dependência nova. JSON local aditivo recebe serde(default). Contador de relógio/callback não usa saturação como substituto de política.

H03 (retenção/desinstalação) fica fora: exige decisão específica, sem apagar perfil/autostart ou alterar instaladores. Hipóteses/refutados não são reabertos. Windows/Wayland/Tauri reais, SendInput, entrega de notificação e incidente específico do mouse permanecem manuais; cada teste posterior deve dizer se foi fake, CI Windows ou físico.

## Adendos de revisão e execução
- B01: escolher expulsão determinística LRU de grupos observados, sem relógio de parede como ordem: repetição move grupo ao início; novo grupo ocupa início e expulsa o último se exceder20. Ordem de chegada resolve empate. Cadência global monotônica30 s não reinicia por resync. Resumo fala de ocorrências/causas/pares OBSERVADOS, não total absoluto perdido entre polls; conjuntos de diversidade também limitados20 com indicador de pelo-menos quando excedidos. Defaults/contador saturante e JSON antigo em teste.
- B03: MAX é sentinela indisponível nos dois lados; new_offer falha também em local MAX-1. Não emite MAX que outro corrigido recusaria. Testar MAX-1 e MAX, módulo vivo com diagnóstico fixo. MAX_REMOTE_AHEAD=1.000.000 é restrição explícita de compatibilidade, não unidade de tempo.
- B05: aceite deve alcançar emit E enter reais do adaptador com send falso/CI Windows, além de helper puro. Nenhum sucesso parcial limpa pendência por inferência.
- B07: ao obter/substituir lifecycle receiver, consumir imediatamente valor atual (não só changed), manter gate fechado até snapshot válido e baseline de generation sem apagar invalidação pendente. Lock+Unlock coalescido invalida antiga sessão; false inicial válido permite apenas ativação nova. None usa obrigatoriamente snapshots do caminho atual Linux/Windows (§10/§11seg), sem dispensá-los. Desktop disponível não é sessão WTS desbloqueada; contrato§15.9 distingue ambos. Stop/pausa prevalecem. Publicar schema exato e posse antes de código, após fatia H01/B02 liberar arquivos.
- B09: STATE_NOT_SAVED descreve Pause OU Resume realmente solicitado; confirmação antiga não apaga aviso mais recente. Bandeja com reconsulta falha conserva unknown/aviso. Backend/casca Rivet; frontend Task2, sem edição concorrente.
