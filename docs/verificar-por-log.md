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
| Infraestrutura | `agent.up`, `peer.connected`, `peer.disconnected`, `module.input.start`, `module.input.stop`, `portal.session.created`, `portal.connect_eis`, `log.truncated` | `peer`, `nome`, `barreiras`, `portal`, `motivo` |

Motivos de `routes.empty`: `suspended`, `no_local_inventory`, `local_without_return`, `no_organization`, `unresolved_legacy`, `no_remote_inventory`, `remote_not_ready`, `local_monitor_missing`, `remote_monitor_missing`, `zone_not_found`, `no_links`, `unsupported_topology`.

Motivos de `session.closed`: `return` (sucesso), `refused`, `error` (com `erro`), e as interrupções `pause`, `lock`, `stop`, `peer_down`, `switch`, `inventory`.

## Para a IA

1. Peça os dois arquivos (ou o pacote de suporte do Windows) e rode o comando com `--json`.
2. Leia `quebra`, `parada` e o primeiro passo que não está `[ok]`. Não atribua causa a um passo sem a linha de prova.
3. Responda com o passo, o motivo registrado e a ação no lado certo (origem ou destino). Se a resposta for "nenhuma tentativa registrada", a investigação é nos pré-requisitos: inventário dos dois lados, rotas e `capture.ready`.

O que este guia não cobre: travessia física comprovada. Os eventos mostram o que cada máquina registrou; a prova de que o cursor apareceu no outro monitor continua sendo a pessoa na frente das telas.
