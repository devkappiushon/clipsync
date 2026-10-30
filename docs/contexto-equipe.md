# ClipSync: contexto comum dos andares

## Preparação da v0.4.0 · 30/10/2026

João autorizou agora commit, push, CI, empacotamento e publicação, ao responder “então faça” ao roteiro desses passos. Esta autorização substitui o limite anterior de não publicar nesta rodada. Preparar os seis pacotes no workflow manual, conferir CI e hashes antes de publicar. Última publicada neste registro ainda é v0.3.1; v0.4.0 está em preparação, não deve ser anunciada como disponível antes da confirmação remota.

Validação local S9 encerrada:597 testes Rust principais passaram,0falhas,3gráficos ignorados e5execuções auxiliares separadas; Node164/164. Revisão integrada aprovada. [Pendências explícitas da próxima versão](pendencias-v0.4.0.md): mouse nas duas máquinas,Tauri interativo,zoom/leitores e lacunas de portal. [Resumo da validação local](evidencias/s9-local-2026-10-30.md).

O material automático abaixo é histórico e não pode reabrir tarefas já concluídas nem revogar a autorização acima.

## Estado atual S9 · validação local encerrada, 30/10/2026

- Revisão integrada e único re-review focal do delta composto aprovados. Foram corrigidos perda de pausa sob fila cheia, expectativa mesh anterior ao agrupamento B01, ciclo do fake standalone, motivo da recusa de Switch e encaminhamento de lifecycle no wrapper. Contrato §15.14; fonte final estável. Revisões focais C01/B09, B01/B03/B05, WTS, I01, Pareamento round4 e B07 FIX1 preservadas.
- Gate final Linux: **597 testes Rust principais passaram, zero falhas, três testes manuais ignorados**; cinco execuções auxiliares em subprocessos também passaram, separadas do total principal. Fmt, clippy workspace/all-targets sem app, cross-clippy Windows GNU de input/win/ui/cli e scripts unsafe/privacidade/diffcheck passaram. Node: 164/164 sobre as mesmas 49 fontes UI. Evidências: `.superpowers/retomada-s9-2026-10-30/validacao-final-2/` e `fecho-local.md`.
- Portal: 60 medições Pixel aprovadas por revisão dos artefatos. Echo declarou 6 PASSA/5 INCONCLUSIVO; o critério completo mantém 5 PASSA/6 INCONCLUSIVO porque não mediu largura 960. Amostras independentes Argus/Revisor UX ficaram sem execução por vínculo incorreto no Maestri; coordenadores Acess/UX atingiram quota. PAR-C, composição inacessível ao mock e prova assistiva continuam limitados. Não há conformidade global declarada.
- Runtime central foi encerrado pelo Maestro após os operadores concluírem; processo ausente e porta 5174 recusando conexão. Portais preservados no canvas. Não iniciar outro servidor sem coordenação.
- **Pendentes externos:** Tauri/CI, Windows nativo (11 testes B05, 14 capture, 6 win), notificações, zoom/teclas/leitores reais e travessia em duas máquinas. A validação local não comprova o relato físico do mouse. Nenhum commit, push, tag ou nova release. HEAD d41aa65; v0.3.1/f700caa continua publicada.

Os registros abaixo preservam o histórico. Esta seção e os relatórios posteriores prevalecem para o estado vigente.

## O que estamos construindo

ClipSync compartilha teclado, mouse e área de transferência entre computadores do usuário, sem servidor central, para substituir o Input Leap. Rust, workspace com oito crates e desktop em Tauri 2. Alvos atuais: Windows e Linux GNOME Wayland. O usuário quer configurar a posição dos computadores com poucos passos, sem escolher bordas obrigatoriamente.

## Fonte de verdade

Repositório central: `/home/kappxs/Documentos/ANTIGOS/clipsync`. Siga a ordem de leitura de `AGENTS.md`: `CONTEXTO.md` (topo) e `docs/workboard.md` para o estado atual, este arquivo como entrada comum e `docs/contrato.md` (§14 para S8) antes de alterar comportamento. A história está em `docs/handoff-codex.md`, que antecede a S8; a evidência mais recente prevalece. Consulte também `docs/verdicts.md` e `docs/seguranca.md`.

Este arquivo é o índice comum. Notas de cada andar devem apontar para esta fonte, em vez de manter cópias independentes de todo o contrato. Em checkout antigo, leia a fonte central antes de assumir que uma pendência ainda existe. Preserve alterações locais; atualização de branch, merge, land ou reset exigem o fluxo apropriado.

## Estado entregue

- v0.3.1 publicada: https://github.com/devkappiushon/clipsync/releases/tag/v0.3.1. Código: `f700caa`.
- Configuração simples por posição/arrasto com confirmação “Ligar nessa posição”; monitor e trecho calculados, com bordas em Ajustes avançados.
- Diagnóstico contextual no app/CLI/bandeja, aviso de protocolo antigo, orientação após parear, teste de entrada explicado e interrupção ao bloquear a origem.
- CI Linux/Windows aprovado (`40783057611`); empacotamento aprovado (`40783058074`). Seis pacotes publicados e hashes conferidos. A run duplicada `40789025116` foi cancelada porque os mesmos artefatos validados foram promovidos ao release.
- Código e simulador revisados por warden e segurança. Sem nova dependência, versão do protocolo ou capability.

## O que continua pendente

O usuário vai instalar e testar nas duas máquinas. A travessia física, as permissões reais do Linux e o roteiro Windows/Linux (§13.12) ainda precisam de evidência. Não afirmar que o mouse real foi corrigido apenas porque o CI passou. Se falhar, levantar versão instalada, status, diagnose, ligação do Layout e permissão de receber controle, sem ler segredos.

O backlog antigo de segurança/manutenção, terceiro computador e decisão de licença não foi incluído nesta rodada. Não iniciar essas frentes sem uma tarefa do Maestro rastreável ao pedido do usuário.

## Como os andares colaboram

1. Rodar `maestri list`, ler a nota local de contexto (briefing/feature-spec e board/workboard) e esta fonte central.
2. Confirmar ao Maestro o objetivo, o escopo atribuído, o checkout e os arquivos sob sua responsabilidade antes de implementar. Uma pessoa por arquivo; runtime pertence ao Maestro.
3. Agentes podem ter conexão entre andares. Notas do Maestri só se conectam dentro do mesmo andar: usar as notas locais como entrada, apontando para esta fonte. Cada coordenador conecta suas notas aos agentes do próprio andar; o Maestro central acessa essas notas pela conexão com o coordenador.
4. Atualizar o workboard e reportar evidência de comandos, limites e pendências. Não iniciar servidor, commit, push, release, merge ou land por iniciativa própria.
5. Português em interface e documentação; sem Playwright, sem ler .env ou configuração de outro programa, sem logs de teclas e sem aceitar permissões do sistema pelo usuário. Manter invariantes de unsafe, protocolo, dependências fixadas e interface de AGENTS.md.

Receber esta nota é alinhamento de contexto, não autorização para começar outra implementação.

## Mapa dos andares

| Andar | Coordenador | Foco do contexto |
|---|---|---|
| Térreo | Maestro · scaffold | Produto, contrato, integração, runtime e release |
| SEG & COMPLIANCE | Maestro | Segurança, privacidade, controles e evidências |
| Acess. | Maestro #2 | Acessibilidade, teclado/foco, contraste e verificação |
| UX & UI | Maestro #3 | Posicionamento simples, feedback e clareza da interface |
| DOC | Maestro #4 | Contrato, estado, evidências e instruções de uso |
| TESTES | Maestro do andar | Teste físico Windows/Linux, roteiro §13.12 e reprodução do relato do cursor (adiado a este andar) |

As notas locais preservam seus modelos para trabalhos futuros. A conexão e o contexto não abrem tarefas de implementação ou auditorias. Antes de operar o canvas, confirme em `maestri list` o nome do seu próprio terminal; uma identidade antiga após troca de agente não deve ser usada para alterar outro time.


## Rodada autorizada 2/3/4 · 2026-10-28

João autorizou continuar o fecho DOC (2), implementar as correções das auditorias (3) e testar (4). Esta autorização substitui o limite anterior de apenas auditar; não significa que as correções ou os testes estejam concluídos. Sem commit, push, tag ou release nesta rodada.

O coordenador DOC passou a Codex, mantendo o nó Maestro #4, o papel e as conexões. Foi autorizado reiniciar os Codex bloqueados no mesmo nó com `codex --no-daemon --sandbox workspace-write --ask-for-approval on-request`; acesso ao socket local usa a escalada normal, sem bypass ou alteração de configuração global.

Quill entregou os 16 documentos e ficou bloqueado pelo limite gratuito. Encerrada a posse dessa entrega, Maestro #4 assumiu somente os dois ajustes finais de `docs/telas.md`: identificar o selo de atalhos e retirar a informação efêmera de runtime/portais, preservando os limites de verificação. As alterações entregues por Quill devem ser preservadas.

O arquiteto nativo prepara a proposta S9; o Maestro central avisará a reserva de `docs/contrato.md`. Este registro não altera contrato de produto nem atribui arquivos de implementação. A distribuição das correções e dos testes cabe ao Maestro central, com posse exclusiva por arquivo e revisão. O runtime 5174 está desligado nesta retomada; somente o Maestro central pode iniciá-lo. Sem Playwright, perfil real, leitura de `.env` ou aprovação de diálogos do sistema. A travessia física continua sem comprovação.

## Execução S9 · 29/10/2026

O contrato §15 está reservado ao Maestro e registrado com posse por arquivo. H-01/B-02 e base acessível estão em implementação; C01 aguarda incorporação dos ajustes de segurança; configuração C14 foi implementada por Rivet e aprovada por Sentinel, com zoom real ainda pendente. Consulte a seção S9 de docs/workboard.md para estado, donos e limites. Os bloqueios de socket dos terminais retomados foram resolvidos por escalada normal autorizada, sem configuração global. A autorização 2/3/4 continua vigente; sem commit/push/release.


### S9 · revisão das entregas e coleta de consumidores (29/10/2026)
- BASE: revisão após FIX1 aprovada; geometria Clipboard/Diagnóstico em32ensaios e D-S9-01 retestado em4casos. Prova DOM tem limite hasFocus=false/zero focusin; leitor/teclas reais não certificados. C14 configuração aprovada, zoom Tauri real pendente.
- Layout/Dispositivos/Pareamento: revisão de código encerrada após FIX2, coordenadas normalizadas da resposta; coleta nos andares Acess/UX em andamento, fontes congeladas em /tmp/clipsync-s9-consumidores-portal-manifest.json.
- Preferências/onboarding/B09 frontend:35testes focados verdes na entrega, revisão encontrou W-S9-PREF-01 (aviso antigo oculta tentativa atual sem confirmação). Duas regressões RED confirmadas; patch de produto aguarda fechamento da coleta que usa a mesma casca. Não interpretar esses REDs intencionais como falha antiga da base.
- C01+B09 Rust/mocks: revisão Sentinel devolveu3Important (preflight de tabelas, observação externa desconhecida e reserva de bootstrap tardio); Rivet executa FIX1 focal. Relatório /tmp/clipsync-s9-modulos-review-seguranca.md.
- H01/B02:330testes dirigidos/3manuais ignorados na entrega, revisão encontrou2Important de fronteira stop/join com fila cheia e rearme antes de controles atuais; builder executa FIX1. Wrapper WTS conferido por leitura independente, sem execução Windows. Relatório /tmp/clipsync-s9-backend-review.md.
- B01/B03/B05/B07: desenho complementar aprovado com ressalvas incorporadas, plano/posse final em preparação; não implementados ainda. H03 requer decisão específica de retenção, sem exclusão de perfil.
- TESTES: matriz em /tmp/clipsync-s9-testes-matriz.txt; suíte integrada aguarda diffs corrigidos e estáveis. Nenhum novo commit/push/tag/release. Instalação/prova física Windows/Linux e CI do app Tauri continuam pendentes.


### S9 · retomada após recuperação do IPC, 29/10/2026
O Maestro confirmou acesso ao socket com escalada normal. Acess/UX/DOC/TESTES receberam a retomada e atingiram o limite gratuito OpenCode; SEG/Rivet/Sentinel falharam no bootstrap Codex mesmo após um reinício autorizado. Os nós/papéis/notas foram preservados. Os andares não estão operacionais; auxiliares nativos executam as posses já autorizadas. Não interpretar recibos de leitura como implementação ou teste concluído.

- Evidência focal FIX1 I1/I2 recuperada: 9 testes de parada/controle/bridge e4 de retarget/zonas passaram,0falhas/ignorados; seis hashes conferem com revisão independente antes/depois. Relatório durável `.superpowers/sdd/2026-10-28-s9-seguranca/retomada-fix1-execucao.md`. Gate de handoff input para B07 liberado, mantendo limites fake/Linux.
- W-S9-PREF-01 aplicado ao app.js: composição conserva incerteza atual e aviso persistente. Cinco testes focais verdes e40/40 nos sete arquivos de interface executados em processos separados; o lote anterior sem isolamento entre arquivos teve3falhas e foi preservado no relatório. Revisão integrada em andamento, portal pendente. `.superpowers/sdd/2026-10-29-s9-consumidores/retomada-interface-report.md`.
- Pareamento: correções de foco na espera, alerta terminal duplicado e caminho de corrigir endereço ainda em preparação; coleta anterior liberada sem afirmar aprovação integral. UI tem dono nativo exclusivo.
- B01: implementação parcial de Rivet preservada e em conclusão por novo dono nativo. B07: dono exclusivo iniciou TDD após handoff; adendo permite somente snapshot inicial nos starts de dois harnesses de pressão, preservando assertions/gates.
- C01+B09 FIX1, B03, WTS e código B05 mantêm aprovações recebidas no histórico; artefatos antigos /tmp não estão acessíveis nesta retomada. Não reconstituir logs como originais. Onze testes B05 Windows ainda sem execução nativa; suíte integrada S9, appTauri, zoom real, leitores e duas máquinas continuam pendentes.
- Notas atuais copiadas sem edição em `.superpowers/retomada-s9-2026-10-29/`. Runtime5174 sem listener e curl recusado nesta conferência; somente Maestro pode iniciar outro. Sem commit/push/tag/release.

### Continuidade após troca de sessão · 29/10/2026
Os relatórios duráveis sobreviveram; os auxiliares nativos anteriores não constam como ativos. Novo dono B07 continua a implementação parcial pelo task-4-report.md, sem reaplicar scripts /tmp ausentes. B01 entregue (task-3-report.md) e Pareamento round3 entregue (retomada-interface-report.md); ambos congelados para revisão independente, sem declarar aceite antes do parecer. A revisão integrada do aviso W-S9-PREF-01 segue aprovada.
A sessão atual não recebeu MAESTRI_SOCKET nem MAESTRI_CLI; o binário /opt/Maestri/resources/cli/maestri existe e debug informa que só funciona dentro de terminal Maestri. Porta5174 sem listener. Não confundir esse estado com o acesso anterior por escalada nem com a janela minimizada da coleta passada. Não recriar nós/conexões por inferência. Portais/interação real pendentes; código/revisões/testes locais continuam autorizados. Nenhum commit/push/release.

### Retomada após reinício do servidor · 29/10/2026, noite
Acesso ao IPC respondeu HTTP 200 no diagnóstico, mas `maestri list` identificou esta conversa como **Revisor · achados #2**, ligada a Maestro #2/Signal e a um portal Mercado/Concorrência. Isso diverge do papel autorizado Maestro · scaffold. Nenhuma nota, identidade, conexão ou terminal desse grafo foi alterado. Porta 5174 sem listener; não há runtime iniciado nesta retomada. Validação pelos portais e reativação dos andares aguardam vínculo correto.

Os auxiliares nativos foram retomados após conferir snapshots e logs existentes: `/root/b07_finaliza` é o único editor B07; `/root/pareamento_r4_finaliza` conclui B-R1 round4 nos dois arquivos de pareamento; `/root/suporte_i01_finaliza` corrige a anonimização de nomes históricos apenas em diag.rs e seu teste. A reinicialização interrompeu os auxiliares antes de novas entregas; confirmações de atividade foram recebidas na retomada. Posse exclusiva e autorização 2/3/4 mantidas, sem commit/push/release.

B01 foi aprovado no delta por revisão independente (`task-3-review.md`); I01 é achado preexistente separado. Pareamento mantém B-R1 aberto até o re-review. B07 tem FIX1 9/9 registrado, mas o teste novo de controle durante cleanup ainda falhou na última execução salva; a fatia não está aprovada. A suíte integrada Node anterior teve 158/158 e antecede a correção round4, portanto não a comprova.


## Retomada 30/10/2026 · identidade central correta
maestri list confirmou Maestro · scaffold, maestro:true e os cinco coordenadores existentes. Reativação enviada uma vez em ask --batch, prompts preservados em .superpowers/retomada-s9-2026-10-30/reativacao-prompts.json; respostas ainda pendentes, não declarar andares operacionais por mero envio. Sem troca de nó/programa/permissão.
B01 revisado por fonte/logs, I01 nomes históricos corrigido com testes focais verdes e em revisão independente. Pareamento round4 tem38/38verdes, falta alinhar inicializador do harness consumidores e re-review. B07 novos casos11/11 verdes e suite input concluída segundo autor, manifesto/review pendentes. Aviso W-S9-PREF-01 integrado/revisado. C01+B09FIX1/B03/WTS/B05 preservados. Nenhuma aprovação Tauri/Windows físico/zoom/leitor/mouse real.
Posse produto: nativos b07_finaliza input; pareamento_r4_finaliza pareamento+testes; I01 diagUI congelado para reviewer. DOC recebeu posse apenas ui/README.md e docs/telas.md. Maestro central mantém contrato/contexto/workboard e runtime. Porta5174 não iniciada ainda. Sem commit/push/release.
