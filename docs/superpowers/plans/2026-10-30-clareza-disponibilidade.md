# Clareza de disponibilidade · plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development ou superpowers:executing-plans para executar por tarefa. Passos usam checkbox.

**Goal:** Explicar disponibilidade e autorização remota sem mudar habilitação ou concessões.
**Architecture:** Apresentação usando somente os dados já recebidos; helper puro pequeno para causas locais se reduzir duplicação. Backend e wire preservados.
**Tech Stack:** JavaScript ES modules, componentes DOM existentes, testes Node do ui-dev.
**Spec:** docs/superpowers/specs/2026-10-30-clareza-disponibilidade-design.md, aprovado por João em 30/10 ao pedir executar tudo e release somente após finalização.

## Global Constraints
- Nenhuma regra de habilitação, grant, confirmação, comando de gravação, API Rust, protocolo ou dependência muda. As variáveis de motivo participam do gate: conservar seus ramos nulos/não nulos.
- Preservar componentes, foco, anúncios, tokens, segurança de texto e recursos futuros visíveis/desabilitados. Não introduzir HTML de nomes recebidos.
- Um editor por arquivo. Root possui contrato, documentação e runtime. Preservar CONTEXTO.md preexistente, perfis reais e artefatos de outras rodadas.
- Português, sem travessão na interface; sem innerHTML/style inline. Nenhum teste por Playwright/browser equivalente.
- Commit e publicação autorizados por João ao final; root fará integração/commits, trabalhador não publica nem muda instalação.

## Review Focus
1. Remoto offline com remote_modules=false por default não pode ser descrito como módulo comprovadamente ausente (T1).
2. Motivo de confiança bloqueada prevalece sobre causa local/estado remoto; etapa de pareamento conserva gate (T1).
3. Placeholder de direção remota não pode se tornar novo grant nem alterar o controle local ou sua confirmação (T1).
4. Estado enabled ausente no Clipboard não comprova plataforma indisponível ou módulo desligado (T1).
5. Nome remoto malicioso continua texto seguro e mudanças de descrição não derrubam foco/semântica do controle real (T1).

### Task 1: Apresentação coerente de disponibilidade e direção

**Files:**
- Modify: crates/clipsync-app/ui/js/telas/dispositivos.js
- Modify: crates/clipsync-app/ui/js/telas/pareamento.js
- Modify: crates/clipsync-app/ui/js/telas/clipboard.js
- Modify: crates/clipsync-app/ui/js/telas/diagnostico.js
- Create, se útil: crates/clipsync-app/ui/js/disponibilidade.js
- Test: crates/clipsync-app/ui-dev/testes/disponibilidade.test.mjs
- Existing regressions: ui-dev/testes/ux-consumidores-s9.test.mjs, ux-pareamento-s9.test.mjs, b09.test.mjs. Só ajustar harness se a nova fronteira importada exigir; não reduzir asserts.

**Interfaces:**
- Consumes: obterApp/recurso existentes, AppState.input_supported/modules/agent/agent_error; Device.state/remote_modules/perms/name; estado de etapa e disponibilidade agregada de Pareamento; ClipboardState.enabled/classification.
- Produces: textos e semântica de orientação sem nova API. Se helper necessário: motivoInputIndisponivel(app) -> string, usado SOMENTE quando recurso(input) já era false; não decide habilitação.
- Copy exata e condições: spec e tabela P1/P2 de .superpowers/brainstorming-pos-v0.4.0-2026-10-30/ux-proposta-continuidade.md. U18 orientação estática sem input/handler; controle local real preservado.

- [x] **Step 1:** Adicionar regressões de comportamento dos ramos nas telas (usar harness de produção existente, não assert de substring). Cobrir remote default false/offline, input_supported=false, modules.input=false, agente/estado desconhecido, blocked e etapas do pareamento, enabled=false/ausente/true, concessão local e orientação remota sem escrita, nome hostil como texto; vazio Diagnóstico não promete RTT.
- [x] **Step 2:** Executar node --test crates/clipsync-app/ui-dev/testes/disponibilidade.test.mjs; registrar falhas esperadas relevantes antes da correção.
- [x] **Step 3:** Corrigir cinco grupos da spec. Módulo desligado e classificação do Clipboard sem diagnóstico conflitante; não alterar formatar global nem dados backend. Pareamento mantém progresso/confirmação, Dispositivos mantém handlers/gates.
- [x] **Step 4:** Executar teste novo e regressões pertinentes acima; se todas passarem, uma execução de todos ui-dev/testes/*.test.mjs com concorrência2 e heap512. Registrar contagens reais, comandos e saídas. Sem Cargo para alteração só JS.
- [x] **Step 5:** Conferir diff próprio, entregar relatório/delta ao root para revisão de spec+qualidade/consentimento. Root integra só arquivos deste recorte; publicação depende também do plano de monitores e gate final.

### Task 2: Conter nomes longos no painel de Dispositivos

**Origem:** PX-A01/UX-A-01 observado na Janela A, não atribuído a regressão. Rótulo local e orientação U18 transbordam com token longo; main cresce 102 px em 1280. Caso e valores em `.superpowers/sdd/2026-10-30-clareza-disponibilidade/portal-ux/revisor/revisao-pixel-v01.md`.

**Files:** somente `crates/clipsync-app/ui/css/telas/dispositivos.css`, se suficiente. Ampliação de posse exige informar root antes de escrever. Nenhum JS, mocks, componente global, dependência ou gate.

- [x] **Step 1:** Ler a evidência e os seletores reais do painel/controle. Preservar texto completo, valores/grants e hit targets. Não introduzir elipse, corte ou overflow oculto para esconder o problema.
- [x] **Step 2:** Corrigir quebra/contenção de nomes/rótulos/descrições longos de modo focal em Dispositivos, com largura mínima adequada aos filhos flex/grid. Não escrever teste de substring de CSS; a regressão é geométrica e a evidência pertinente é DOM no portal.
- [x] **Step 3:** Conferir diff/check e entregar delta/relatório congelados ao root para revisão independente. Sem nova suíte Node ou Cargo para CSS isolado.
- [x] **Step 4:** Root publica manifesto novo e pede reteste somente da geometria afetada: 1280 dois temas, texto original/longo, conferindo `scrollWidth/clientWidth` de main/rótulos/descrições, texto integral e ações. Checar 960/320 como regressão da regra em larguras estreitas. Usar viewport efetivo e registrar limites das capturas, sem declarar matriz anterior concluída. Os recortes não cobertos da janela original continuam pendentes.


### Task 3: Conter nomes longos no Pareamento

**Origem:** PG-P01 da janela DOM V02–V05: nome sintético longo provoca excesso em rótulo, orientação U18, introdução/instrução; em 320 o main cresce 68 px. Evidência: `.superpowers/sdd/2026-10-30-clareza-disponibilidade/portal-geometria-restante/pixel/relatorio.md` e JSONs V02 claro1280/320. Não há regressão atribuída ao CSS de Dispositivos. A operação de ambos os portais foi devolvida pelo coordenador, com manifesto 50/50 preservado.

**Files:** exclusivamente `crates/clipsync-app/ui/css/telas/pareamento.css`. Não tocar regras globais/componentes, JS, mocks, protocolos ou gates. Se a causa exigir outra posse, informar ao root antes de editar.

- [x] **Step 1:** Ler evidência DOM e árvore/seletores reais em pareamento.js e CSS. Identificar como tamanho intrínseco/quebra propaga do texto aos itens flex/grid; confrontar a correção focal de Dispositivos sem copiá-la por suposição.
- [x] **Step 2:** Corrigir a causa com regras de apresentação restritas à tela. Preservar texto integral, controles, valores, semântica e tamanho dos alvos; sem elipse, clipping, overflow oculto, truncamento ou redução artificial de fonte. Não corrigir indiscriminadamente o componente global.
- [x] **Step 3:** Entregar relatório, diff e hash congelados; git diff --check. Sem teste que só confere substring de CSS nem nova suíte Node/Cargo. Revisão independente de escopo/correção antes do portal.
- [x] **Step 4:** Root gera manifesto novo e pede V02 em dois temas × 1280×800/960×640/320×640, texto original/longo, somente geometria DOM. Setup deve aguardar estado renderizado antes de medir, com prazo e falha registrados; nenhuma operação de navegação simultânea. Não conceder controle nem concluir pareamento; restaurar textos e cancelar a tentativa ao final. Conferir main, rótulos, título/descrição U18, introdução/instrução, texto integral e ações/switches. Não repetir V01/V03/V05 aprovados; V04 ausentes podem ter janela separada após o reteste. Histórico raster permanece separado.
