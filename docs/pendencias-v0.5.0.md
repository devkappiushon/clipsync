# Pendências da v0.5.0

Lista única do que ainda está aberto depois da publicação da v0.5.0 (31/10/2026, commit `590c137`). Consolidada em 03/11/2026 a partir de `CONTEXTO.md`, `docs/pendencias-v0.4.0.md`, `docs/workboard.md`, `docs/contrato.md` §16, `docs/seguranca.md` e das notas da versão. Um item só sai daqui com evidência registrada: sistema, versão, cenário e resultado. **CI verde, simulador e adaptadores falsos não fecham nenhum item da seção 1.**

## 1. Validação física (gate de qualquer versão seguinte)

| Pendente | Como comprovar |
|---|---|
| Travessia A ↔ notebook ↔ B com o mouse físico do desktop Windows | Mesma v0.5.0 nas duas máquinas, organização aplicada; atravessar A → notebook → B e B → notebook → A; registrar entrada proporcional, retorno pelo lado de entrada e eventual salto ao centro. Exige dois monitores no Windows. |
| Travessia PC Windows → notebook Linux, uma direção | Comprovada em 04/11/2026 com o build `7e5b1e7` e `clipsync verificar-travessia` nos dois logs: seis sessões com retorno, onze passos `[ok]`. O sintoma anterior era `attempt.filtered{peer_offline}` (notebook desconectado); resolvido ao parear de novo. Faltam a volta notebook → PC e a cadeia com três máquinas. |
| Posição real do cursor na chegada | Comparar a posição esperada com a observada; a proposta de medição P5 (oráculo físico) ficou inconclusiva por falta de instrumento. |
| Teclado, roda e soltura | Shift ou tecla pressionada ao atravessar; Pausar; bloquear origem e destino; desconectar. Nenhuma tecla pode ficar pressionada. |
| Incidente "teclado não digita no notebook" | Adiado em 30/10/2026. Reproduzido em 04/11/2026 com os logs novos: falhou só na primeira sessão depois de o emulador do notebook ser criado (`emulate.start{reutilizado:false}`, portal e EIS recém-conectados) e funcionou na segunda, com o emulador reaproveitado. Nessa sessão o PC enviou 734 datagramas sem descarte e o notebook emitiu tudo sem `emulate.error`. O log ainda não conta teclas por sessão nem registra os dispositivos EIS; é a próxima instrumentação. Teste que separa os lados: reiniciar só o notebook e atravessar; depois só o PC. Precisa de aplicativo, campo e sequência da falha original. Em 11/11/2026 o relato voltou: "na primeira vez que eu passo o mouse para o notebook, o teclado não funciona; tenho que passar pro PC, digitar algo, e depois voltar". O log do notebook de 11/11 19:21 UTC mostra a primeira sessão com `portal.session.created` e `portal.connect_eis` e nenhum `emulate.error`; não há log do PC dessa data. Hipótese registrada na nota: as teclas vão para um dispositivo que o libeis não considera EMULATING (recriado ou pausado) ou o PC não as envia. |
| **F2 · PC travado, captura que não solta** | Relato de 11/11/2026: "muitas vezes meu mouse e teclado param e o PC fica como se fosse travado, o mouse não volta, é como se o clipsync travasse e não liberasse o mouse e o teclado; ele não mexe no notebook nem no pc". `tls.rs` usa KEEP_ALIVE de 5 s e IDLE_TIMEOUT de 15 s; o log do notebook de 10/11 tem `peer.disconnected` "timed out" e sessões em que todo emit falhava enquanto o PC seguia capturando. `wincore.rs` bloqueia todo evento local em Arming/Active (`Verdict::Block`) e, lendo o código, não há vigia na origem que solte a captura por falta de resposta do destino antes do timeout do QUIC. Hipótese: o destino para de responder com a captura ativa e a origem segura mouse e teclado até 15 s ou mais. Como comprovar: log do PC (`%LOCALAPPDATA%\clipsync\logs\clipsync.jsonl`, ainda faltando) e teste no app real medindo a soltura em até 2 s (proposta do contrato §17.8). |
| **F3 · PC para de enviar até reabrir o app** | Em 11/11/2026, das 14:32 às 16:20 (hora local), o par ficou conectado e o clipboard funcionou, mas nenhum `enter.received` chegou; voltou cerca de 2 s depois de o usuário fechar e reabrir o ClipSync no PC (16:21). Reconectar o par às 14:32 e às 16:19 não bastou. Em `clipsync-win/src/ffi.rs`, o gancho só é vigiado com o cursor estacionado (o tick compara com PARKED), então um gancho removido pelo Windows com o cursor parado não é percebido; não há prioridade de thread nem saída do modo de economia. `adapter/win/capture.rs` recria os ganchos a cada `capture.start`. Relato de 10/11: "quando eu coloco o clipsync em segundo plano e o sistema operacional corta recursos, o sistema para". Hipótese: thread do gancho lenta ou presa; sem log do PC não fecha a causa. Como comprovar: log do PC da janela e o evento proposto `hooks.reinstalled` (§17.8). |
| Busy=10 persistente depois de aceitar a oferta de organização | Tratado em FIX1 e FIX2 no simulador; sem confirmação no app real. |
| Ambas as máquinas autorizando entrada | Conferir as direções de saída configuradas, controle e retorno sem sessões sobrepostas. |
| Permissões e plataformas reais | Portais do GNOME Wayland; UAC, desktop seguro, atalhos Win+Alt, notificações e mudança de DPI ou de monitor no Windows. |
| App Tauri interativo | Abrir os instaladores publicados nas duas máquinas e conferir janela, bandeja, avisos e zoom nativo. Build e testes no runner não provam interação. |
| Acessibilidade assistiva | Tab, Shift+Tab, Enter e Esc com foco visível no app real; zoom e reflow; Orca e NVDA com anúncio único. |
| Instalação no Windows | Instalar a v0.5.0 e remover a v0.4.0 se ainda estiver lá. O notebook Linux já tem `clip-sync 0.5.0` (`dpkg-query`, 03/11/2026). |

Roteiro de referência: [contrato §13.12](contrato.md#1312-roteiro-manual-do-usuário-pc-windows-e-notebook). Para reportar: versão das duas instalações, origem e destino, posição no Layout, ação feita e resultado observado. Use texto fictício no clipboard; nunca envie segredo no diagnóstico.

## 2. Produto e segurança

| Pendente | Estado |
|---|---|
| Licença | "Todos os direitos reservados" até o autor escolher. Decisão do usuário; MIT foi sugerida e não decidida. |
| Terceiro computador | Pergunta aberta no contrato §13.14; nenhum teste com três máquinas. |
| Imagens e arquivos no clipboard | Só texto (`clipsync-proto/src/clip.rs` rejeita outros formatos). Sem decisão de implementar. |
| Clipboard sensível no Linux | `x-kde-passwordManagerHint` não é usado nem consultado (revisão S5). S5-03 cobre a leitura no Windows. |
| T4-SEG-01 (menor, diferido) | Corrida local entre conferir e abrir o backup do layout; exige processo já capaz de escrever no perfil. |
| Backlog S1 e S5 | Centralizar `write_private`; instaladores Windows assinados; ajuda e erros do `clap` em inglês; filtro de teste `control_`; embrulho do `quinn-proto`. |
| Recursos visíveis e desabilitados | Pausar par (`peer_pause`), última atividade do par, RTT e latência por par, modo de clipboard com confirmação, pausar captura e modo privado. Sem backend nesta versão. |
| Atalhos globais no Linux | Não existem (só o adaptador Windows registra Win+Alt+1 a 9, L e P). |
| Reencontro por nome (N1) | Reencontro por nome refeito em 11/11 (fatia N1), aguardando re-revisão do Argus e do Sentinel. |

## 3. Interface e design

| Pendente | Estado |
|---|---|
| Revisão visual do app com cara de portfólio | Feita em 04/11/2026 pelo portal `Telas Maestro` (10 capturas, 1280×800 e 960×640, temas claro e escuro). Corrigido: tabela de Dispositivos sem rolagem horizontal, vão acima do primeiro aviso em Dispositivos e Layout, cabeçalho de coluna mono na fonte de texto, versão do simulador em 0.5.0. Registro em `docs/contrato.md` §16.6. A conferência por DOM de 03/11 (contraste acima de 4,5:1, sem vazamento horizontal) segue válida. |
| Capturas das telas para o README | Feitas em 04/11/2026: `docs/telas/v0.5.0/` (sete telas em webp, 1280×800); o README mostra Dispositivos e Layout. As de `docs/telas/comparacao/` continuam da v0.2.x. |
| Figma | Não foi atualizado com as telas da v0.5.0 (editor de monitores, prévia das passagens). Plano Starter com cota mensal de chamadas. |

## 4. Repositório e documentação

| Pendente | Estado |
|---|---|
| Tornar o repositório público | Feito em 04/11/2026: repositório público e `main` protegida contra force-push e deleção. Árvore sem segredo, chave ou token (varredura em 03/11/2026); caminhos locais, nome de host, IP da LAN e ids reais anonimizados na árvore. O histórico do git ainda os contém; reescrever invalidaria as 7 tags e os links dos releases. |
| LICENSE | Ausente; ver seção 2. |
| README e guias | Feitos em 04/11/2026 (commit `af7e089`): README com capturas; `docs/instalar.md`, `docs/uso.md`, `docs/limitacoes.md`, `docs/desenvolvimento.md`. |
| Referências a `.superpowers/` | Vários documentos citam evidências nessa pasta, que fica fora do repositório. Elas valem só para quem tem o checkout original. |

Histórico: [pendências da v0.4.0](pendencias-v0.4.0.md) e [notas da v0.5.0](releases/v0.5.0.md).


## 5. Estado de F1/F2/N1 e pendências associadas · 12/11/2026

Este estado prevalece sobre as descrições anteriores de ausência de vigia/instrumentação e de revisão de N1 ainda pendente. As linhas acima preservam os relatos e hipóteses históricos; não descrevem sozinhas o código atual. F1, F2 e N1 receberam aprovação com ressalvas para commit no [R10](verdicts.md#r10--conjunto-v053--12112026), ainda não publicado. A aprovação não fecha nenhuma validação física, não comprova causa do incidente nem autoriza release sem seus gates. As fontes de entrada citadas pertencem ao recorte de 12/11 identificado por SHA no [adendo do contrato](contrato.md#178-deltas-de-11112026); não pressupõem checkout imutável.

| Item | Estado atual e limite | Evidência |
|---|---|---|
| F1 · teclado da primeira sessão | Instrumentação EIS, continuidade do mouse com descarte de teclas sem replay, reconciliação de solturas e aviso no status local do destino implementados. Causa física não provada; apresentação Tauri e teclado real continuam pendentes. | R9/R10 em `docs/verdicts.md`; `crates/clipsync-input/src/adapter/portal.rs:675`, `:697`, `:819`, `:1015`, `:1272`; `crates/clipsync-input/src/module.rs:1377`. |
| F2 · captura sem resposta | Vigia por Stats implementada: 500 ms nominal e prazo lógico de 2 s desde abertura/última resposta válida; devolução local antes da limpeza. Falha genérica legada recusa ApplyFailed. Sem garantia de soltura remota, teto físico sob carga extrema ou confirmação de teclado aplicado por Stats saudáveis. | R4/R10; `crates/clipsync-input/src/capture.rs:81`, `:852`, `:893`, `:1071`; `crates/clipsync-input/src/module.rs:2025`. |
| N1 · reencontro por nome | Cache separado do pareamento implementado e aprovado no recorte R5/R10. Endereços que a v0.5.2 gravou incorretamente não são saneados automaticamente; novo pareamento é indicado só no caso afetado, sem apagar identidade. DHCP/firewall físicos ainda pendentes. | `crates/clipsync-core/src/transport.rs:1018`, emissor `:1033`; `crates/clipsync-core/src/agent.rs:157`, `:197`; `crates/clipsync-core/src/pairing.rs:242`, `:275`; R5/R10. |
| F3 · gancho do Windows | Em implementação separada, fora da aprovação F1/F2/N1. Não anunciar `hooks.reinstalled` ou alterações concorrentes como correção publicada/validada. | R10-ESC-01 em `docs/verdicts.md:5385`; [contrato §17.8](contrato.md#178-deltas-de-11112026). |
| A1 · aviso de devolução local | Implementado na v0.5.3: ao devolver o controle por silêncio do destino ou por recusa de aplicação, a origem registra o retorno no estado do módulo e notifica `control_returned`, com identidade de ocorrência para não perder devoluções repetidas. S2-PRIV-01 fica parcialmente mitigado: o aviso chega depois da devolução, depende da consulta de 1 s e das notificações do sistema, e não impede a digitação local. Falta ver a notificação no Windows real, com a janela minimizada. | [Contrato §17.9](contrato.md#179-aviso-de-retorno-a1--13112026); R13 e R14 em `docs/verdicts.md`; `crates/clipsync-input/src/msg.rs:3`; `crates/clipsync-ui/src/events.rs:482`. |
| R10-LOG-01 · cauda/pacote | Fechado no código da v0.5.3 (parecer R12): a cauda do pacote de suporte aceita os eventos, motivos e campos novos de entrada, sem texto livre. Binário anterior ainda os omite: confirmar a versão instalada e preservar o JSONL bruto. `hooks.reinstalled` segue fora, com a F3. | `crates/clipsync-ui/src/diag.rs:760`, `:847`, `:892`, `:933`, `:985`; R12 em `docs/verdicts.md`; [guia de log](verificar-por-log.md#ciclo-eis-e-limites-do-pacote-de-suporte). |

As referências `.superpowers/` são evidências locais do checkout original, não arquivos publicados. Este adendo registra o estado de 12/11; nenhum item manual foi encerrado nem A1/F3 implementado por esta atualização documental.
