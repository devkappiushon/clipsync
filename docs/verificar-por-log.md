# Verificar a travessia pelo log

Guia para descobrir, a partir dos arquivos de log das duas máquinas, até onde uma tentativa de passar o mouse chegou e qual foi o primeiro ponto que falhou. Vale para quem usa o ClipSync e para uma IA que só lê a saída do comando. Regras completas no [contrato §17](contrato.md).

## Onde fica o log

Cada máquina escreve um arquivo por processo do agente, em linhas JSON, só com estado, contagens e ids curtos: nunca conteúdo do clipboard, tecla ou keycode.

| Sistema | Caminho |
|---|---|
| Linux | `~/.config/clipsync/logs/clipsync.jsonl` |
| Windows | `%LOCALAPPDATA%\clipsync\logs\clipsync.jsonl` |

Rotação por tamanho: ao passar de 5 MiB o arquivo vira `clipsync.jsonl.1`, e o anterior `clipsync.jsonl.2`. O caminho aparece em `clipsync status`, em `clipsync log caminho` e na tela Diagnóstico do app. O arquivo só é escrito pelo agente (app ou `clipsync run`); comandos da CLI nunca escrevem nele.

## Como coletar

1. Faça a tentativa com o mouse físico (ou use **Testar passagem** em Ajustes avançados do Layout de telas).
2. Copie `logs/clipsync.jsonl` de cada máquina para uma pasta só. Se houver `.1` ou `.2` do mesmo horário, copie também.
3. No Windows, sem terminal: **Diagnóstico > Exportar pacote de suporte** gera um `.txt` em Downloads com as últimas 400 linhas do log, já redigidas.

**Pacote de suporte a partir da v0.5.3:** a cauda passa a aceitar os eventos e campos novos de entrada (`peer.readdressed`, `eis.device`, `eis.loop`, `peer_unresponsive`), fechando o achado R10-LOG-01 (parecer R12). Binário anterior à v0.5.3 ainda os omite. Preserve os JSONL originais e confira a versão do binário; a ausência de um campo na cauda não prova ausência no log bruto. Veja o [estado atual do filtro](#ciclo-eis-e-limites-do-pacote-de-suporte).

## O comando

```
clipsync verificar-travessia --origem pc.jsonl --destino notebook.jsonl
```

A origem é a máquina que tem o mouse físico; o destino, a que deveria receber o cursor. O comando só lê os arquivos: não precisa do agente, da rede nem do perfil.

| Opção | Efeito |
|---|---|
| `--destino <arquivo>` | Log do destino. Sem ele, os passos do destino saem como `[sem dado]`. |
| `--ultima` | Usa a última tentativa registrada na origem (padrão). |
| `--sessao <id>` | Confere só a sessão com esse id nos dois arquivos. |
| `--desde <instante ou duração>` | Limita a janela: `2026-11-04T18:00:00Z` ou `30m`, `2h` (contado do último registro da origem). |
| `--json` | Mesma conclusão em JSON, para a IA ou para guardar. |

## Como ler a saída

Uma linha por passo, na ordem em que a travessia acontece: `inventario_origem`, `inventario_destino`, `rotas`, `captura`, `tentativa`, `sessao`, `entrada_destino`, `emulacao`, `datagramas`, `retorno`, `fechamento`.

- `[ok]`: o evento esperado está no log; a linha crua vem ao lado, como prova.
- `[falhou]`: há um evento explícito de erro ou recusa, com o motivo. O primeiro `[falhou]` é a **quebra**.
- `[sem dado]`: nenhum registro. Pode ser passo não alcançado ou log truncado; o verificador não adivinha. O primeiro `[sem dado]` antes de a cadeia completar é a **parada**.

Dois casos de `[ok]` vêm com nota. `datagramas` e `retorno` dizem `confirmado nos dois lados`, `confirmado só na origem` ou `confirmado só no destino`: a prova de um lado basta, e a linha mostrada é desse lado. `inventario_destino` diz `não se aplica: entrada por layout legado (enter_span)` quando a sessão abriu pelo layout antigo, em que o destino não prepara inventário.

Os relógios das duas máquinas raramente batem. Os eventos da sessão são casados por par, sessão e execução, com tolerância de 30 segundos entre as máquinas; uma diferença maior que essa deixa os passos do destino como `[sem dado]`. Se o destino reiniciou a menos de 30 segundos da tentativa, o verificador não escolhe entre as duas execuções: os passos do destino saem `[sem dado]` com `reinício do destino na faixa de incerteza do relógio`, e só uma falha explícita conta. Acerte a hora das duas antes de coletar e evite atravessar nos primeiros 30 segundos depois de abrir o app.

Sem tentativa na janela, o comando mostra só os pré-requisitos e conclui "nenhuma tentativa registrada: a origem nunca viu o cursor na barreira". É exatamente o sintoma de um mouse que nunca sai da tela.

Códigos de saída: `0` tudo `[ok]`, ou só pré-requisitos sem quebra; `1` quebra provada, ou tentativa que não completou; `2` erro de uso; `3` arquivo inexistente, ilegível ou de esquema mais novo.

### Exemplo: inventário do notebook nunca chegou ao PC

```
[ok] inventario_origem | {"v":1,"ts":"2026-11-04T18:00:02.000Z",...,"evento":"inventory.prepared","monitores":2,"uids":["a1","b2"]}
[sem dado] inventario_destino: inventário de monitores não registrado; sem registro: passo não alcançado ou log truncado
[falhou] rotas: no_remote_inventory | {"v":1,"ts":"2026-11-04T18:00:02.200Z",...,"evento":"routes.empty","motivo":"no_remote_inventory","rotas":0}
[sem dado] captura: captura com barreiras para o par selecionado não registrada; sem registro: passo não alcançado ou log truncado
[sem dado] tentativa: nenhuma tentativa registrada: a origem nunca viu o cursor na barreira; sem registro: passo não alcançado ou log truncado
primeiro ponto de quebra: rotas
parada: inventario_destino; sem registro: passo não alcançado ou log truncado
```

Leitura: o PC preparou os próprios monitores e mandou o inventário, mas nunca recebeu o do notebook, então não montou rota nem barreira. O próximo passo é no notebook: **Preparar telas** no Layout, aceitar o diálogo do sistema e conferir se aparece `inventory.prepared` no log dele.

### Exemplo: notebook desconectado

Caso real de 04/11/2026, PC Windows como origem, antes de parear de novo:

```
[ok] inventario_origem | {"ts":"2026-11-04T17:30:01.148Z",...,"evento":"inventory.prepared","monitores":2,...}
[sem dado] inventario_destino: arquivo do destino não informado; sem registro: passo não alcançado ou log truncado
[ok] rotas | {...,"evento":"routes.built","rotas":1,"barreiras":1,...}
[ok] captura | {...,"evento":"capture.ready","adaptador":"win-hooks",...}
[falhou] tentativa: peer_offline | {"ts":"2026-11-04T17:30:07.887Z",...,"evento":"attempt.filtered","attempt":1,"motivo":"peer_offline"}
primeiro ponto de quebra: tentativa
```

Leitura: o PC tinha rota e barreira e viu o cursor chegar à borda, mas o notebook não estava conectado naquele momento. A investigação é na conexão: `clipsync status` nas duas máquinas, e parear de novo se o endereço mudou. Depois do repareamento, o mesmo PC registrou seis sessões com `return`.

## Os eventos

Toda linha tem `v` (versão do esquema, hoje 1), `ts` (UTC com milissegundos), `nivel`, `host`, `alvo` e `evento`. Os campos abaixo entram quando existem; `peer` é sempre o id curto do par e `session` o id que viaja nas mensagens de entrada. `pos` é fração de 0 a 1 com três casas.

| Passo | Eventos | Campos principais |
|---|---|---|
| Inventário | `inventory.prepared`, `inventory.sent`, `inventory.received`, `inventory.missing` | `monitores`, `uids`, `peer`, `revisao`, `motivo` |
| Rotas | `routes.built`, `routes.empty` | `rotas`, `barreiras`, `pares`, `motivo` |
| Captura | `capture.start`, `capture.failed`, `capture.ready` | `adaptador`, `motivo`, `barreiras` (lista de `peer`, `monitor`, `borda`, `trecho`) |
| Tentativa | `attempt.begin`, `barrier.activated`, `attempt.filtered` | `attempt`, `origem` (`barrier`, `switch`, `test`), `barreira`, `pos`, `motivo` |
| Sessão | `session.open`, `session.sent`, `session.active`, `session.refused`, `session.closed` | `attempt`, `session`, `peer`, `mensagem`, `rota`, `motivo`, `erro`, `totais` |
| Destino | `enter.received`, `enter.refused`, `emulate.start`, `emulate.positioned`, `emulate.error` | `session`, `peer`, `mensagem`, `motivo`, `adaptador`, `pos`, `erro` |
| Datagramas e retorno | `datagram.first`, `return.received`, `return.sent` | `session`, `peer`, `pos`, `totais` |
| Infraestrutura | `agent.up`, `peer.connected`, `peer.disconnected`, `peer.readdressed`, `module.input.start`, `module.input.stop`, `portal.session.created`, `portal.connect_eis`, `log.truncated` | `peer`, `nome`, `barreiras`, `portal`, `motivo`, `origem` |

Motivos de `routes.empty`: `suspended`, `no_local_inventory`, `local_without_return`, `no_organization`, `unresolved_legacy`, `no_remote_inventory`, `remote_not_ready`, `local_monitor_missing`, `remote_monitor_missing`, `zone_not_found`, `no_links`, `unsupported_topology`.

Motivos de `session.closed`: `return` (sucesso), `refused`, `error` (com `erro`), e as interrupções `pause`, `lock`, `stop`, `peer_down`, `switch`, `inventory`. No conjunto F1/F2/N1 ainda não publicado, `peer_unresponsive` registra fechamento por silêncio do destino (`crates/clipsync-input/src/capture.rs:867`, emissão em `:1086`).

`peer.readdressed` registra que a origem reencontrou o par num endereço novo pelo nome da máquina (`origem=name`); não é passo da cadeia e não muda a conclusão. O emissor atual é `learn`, em `crates/clipsync-core/src/transport.rs:1018`, com o evento em `:1033`. O cache separado não reescreve `peers.toml` (`crates/clipsync-core/src/agent.rs:197`). A preparação da v0.5.3 não saneia endereços legados gravados incorretamente pela v0.5.2; novo pareamento é indicado somente nesse caso afetado, sem apagar identidade ou usar `init` (limite R5/R10; gravação do pareamento em `crates/clipsync-core/src/pairing.rs:242`, `:275`).

### Estado vigente de F1/F2/N1 · 12/11/2026

Este estado substitui a descrição anterior de eventos aguardando confirmação. O conjunto foi aprovado com ressalvas em [R10](verdicts.md#r10--conjunto-v053--12112026), mas ainda não foi publicado nem validado fisicamente. Os exemplos reais de 04/11 acima permanecem históricos. Contrato, vocabulário completo e SHAs que identificam as fontes de entrada citadas em 12/11: [§17.8](contrato.md#178-deltas-de-11112026). As linhas valem para esse recorte, não para futuras alterações do checkout.

- **Silêncio:** `session.closed{motivo=peer_unresponsive}` registra a devolução local da captura após prazo lógico de 2 s desde a abertura ou última Stats válida tratada pelo módulo. A sondagem é nominalmente de 500 ms, sem envio sobreposto. Par, conexão, sessão e timestamp precisam corresponder; uma resposta atrasada não compra tempo no consumo. A captura é liberada antes da limpeza de rede. Não é garantia de soltura física remota nem teto medido sob carga extrema. Fontes: `crates/clipsync-input/src/capture.rs:81`, `:839`, `:852`, `:893`, `:1071`, `:1325`; `crates/clipsync-input/src/module.rs:1465`, `:1629`.
- **Recusa por falha de emissão legada:** `enter.refused{motivo=apply_failed}` no destino e `session.refused{motivo=apply_failed}` na origem descrevem a recusa `ApplyFailed`. O fechamento correspondente é `session.closed{motivo=refused}`, não `apply_failed`. `enviada=false` no destino não comprova recebimento da recusa na origem. No ramo v4 ou erro `Locked`, a classificação continua `Locked`. Fontes: `crates/clipsync-input/src/module.rs:1896`, `:1918`, `:1939`, `:2025`; `crates/clipsync-input/src/capture.rs:367`, `:1635`, `:1717`.
- **Teclado ausente:** a falta apenas de teclado EIS pronto descarta e conta teclas, sem replay, mantendo a sessão do mouse. Pode durar enquanto o destino segue respondendo Stats; Stats saudáveis não provam teclado aplicado. Após descarte, `KEYBOARD_UNAVAILABLE` entra no status do destino com o texto “Teclado indisponível neste computador: as teclas não estão sendo aplicadas”, preservando motivos anteriores do módulo no recorte F1 aprovado em R9. No delta A1 concorrente identificado no contrato, `control_returned` tem precedência e substitui o resultado composto no status (`crates/clipsync-input/src/module.rs:1393`); C1 não aprova esse delta. A retomada retira o aviso F1 do adaptador; não há novo aviso remoto nem prova da apresentação Tauri. Fontes: `crates/clipsync-input/src/adapter/portal.rs:675`, `:730`, `:980`, `:1272`; `crates/clipsync-input/src/msg.rs:207`; `crates/clipsync-input/src/module.rs:1377`.

### Ciclo EIS e limites do pacote de suporte

| Evento no JSONL bruto | Como interpretar | Fonte em `crates/clipsync-input/src/adapter/portal.rs` |
|---|---|---|
| `eis.device` | `acao` distingue anúncio, retomada, pausa, remoção, ausência, descarte e reconciliação. `dispositivo` é número local; `capacidades` usa nomes fixos. `missing` no início não comprova que alguma tecla foi descartada. | `:577`, `:596`, `:608`, `:686`, `:719`, `:951`, `:1098` |
| `eis.device{acao=reconciled}` | Pedidos de soltura pendentes enviados quando o teclado volta ou no cleanup, inclusive `origem=release_all`. Pendências só são retiradas após frame/flush; isso não comprova aplicação física. | `:697`, `:731`, `:819`, `:983` |
| `eis.loop` | Fim do laço com motivo fixo e, quando disponível, categoria fixa de erro. `contagem`/`descartados` acumulam a vida do laço, não uma sessão; falhas `context`, `handshake`, `runtime` podem sair sem totais. | `:883`, `:896`, `:1012`, `:1061`, `:1073` |

Os totais de `eis.device` são por dispositivo e os de `eis.loop`, pela vida do laço; em `reconciled`, `contagem` é o tamanho daquele envio de solturas. São pedidos/descartes, não teclas comprovadamente aplicadas no sistema. Não há tecla, keycode ou replay do conteúdo descartado nesses eventos. As solturas ficam pendentes no laço, sem garantia após morte do processo ou indisponibilidade permanente do teclado (`crates/clipsync-input/src/adapter/portal.rs:608`, `:643`, `:697`, `:719`, `:1012`).

**R10-LOG-01 é o achado histórico da cauda incompleta.** Na revisão R10, os eventos EIS/de reencontro e campos novos eram omitidos ou removidos; `contagem` já era campo numérico permitido e `apply_failed` já era motivo aceito. A v0.5.3 corrige o filtro em `crates/clipsync-ui/src/diag.rs`: eventos em `:760`, motivos em `:847`, capacidades em `:892`, campos em `:933` e erro contextual de `eis.loop` em `:985`, aprovados no parecer R12 de `docs/verdicts.md`. Valor fora da lista continua omitido e erro em texto livre não passa. `hooks.reinstalled` segue fora. Não presumir que o pacote de um binário instalado contém essa mudança: confira a versão e preserve o JSONL bruto para investigar.

`hooks.reinstalled` pertence à proposta F3 em implementação separada, fora do conjunto F1/F2/N1 aprovado. Sua presença em alterações concorrentes não comprova entrega nem correção física do gancho. O aviso de teclado no destino também não fecha A1/S2-PRIV-01: o risco de as próximas teclas irem ao foco local após retorno silencioso exige validação da mitigação. A1 já tem implementação concorrente de aviso de retorno, mas C1 não certifica aprovação ou prova física (`crates/clipsync-input/src/capture.rs:869`, `:1031`; `crates/clipsync-input/src/msg.rs:3`). Não houve vazamento constatado nesta revisão. Fontes: R10 em `docs/verdicts.md:5385`, `:5425`; `.superpowers/incidente-2026-11-11/S2-sentinel.md:41–55`.

## Para a IA

1. Peça os dois arquivos e rode o comando com `--json`. O pacote de suporte pode ajudar, mas não substitui o JSONL bruto para o ciclo EIS e os campos novos quando o binário é anterior à v0.5.3, que ainda omite esses eventos na cauda.
2. Leia `quebra`, `parada` e o primeiro passo que não está `[ok]`. Não atribua causa a um passo sem a linha de prova.
3. Responda com o passo, o motivo registrado e a ação no lado certo (origem ou destino). Se a resposta for "nenhuma tentativa registrada", a investigação é nos pré-requisitos: inventário dos dois lados, rotas e `capture.ready`.

O que este guia não cobre: travessia física comprovada. Os eventos mostram o que cada máquina registrou; a prova de que o cursor apareceu no outro monitor continua sendo a pessoa na frente das telas.
