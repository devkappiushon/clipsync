# S9 · consumidores UX e fluxos acessíveis

Autorização de João: continuar 2/3/4. Implementação por subagente e revisão independente, sem commits.

**Objetivo:** transformar achados confirmados em fluxos claros e operáveis com poucos passos.
**Spec:** contrato §15.6 e docs/superpowers/specs/2026-10-28-s9-ux-acessibilidade.md §§3–7; matriz canônica de cobertura. C01 está fixado no contrato§15.7/spec2026-10-29-s9-modulos; B09 em§15.8/spec complemento.

## Global Constraints
- Só arquivos da posse §15.6 e ampliação B09 da Task2 (§15.8); demais base/frontend, mocks/Rust/docs ficam com seus donos. UI api.js só wrapper modulesSet quando schema final liberado.
- Sem wire, dependências, innerHTML, style=, scripts inline, travessão em UI, dados do par em HTML, Playwright, servidor, perfil real, commit/push.
- Simulador não prova captura/consentimento/zoom/teclado nativo/uso físico; não alterar o sentido das permissões.
- Posição é caminho simples. Avançado oferece alternativa real ao arrasto, sem forçar bordas no fluxo principal.

## Review Focus
- Permissão/CTA não invertem direção; não confundir input_in com papel permanente.
- Quatro posições geram prévia e geometria coerentes em múltiplos monitores; preserve mesa Dell/LG.
- Operação/repintura não perde foco nem rouba foco deliberado, rota ou outro diálogo.
- Resize/DOM não são prova de zoom/teclas/leitor; apontar limites em cada resultado.

### Task 1: jornadas e editor de passagem
**Arquivos:** §15.6, exceto onboarding/preferencias/api.js reservados à Task2; testes novos UX e travessia/oferta existentes.
**Dependência:** base publica interfaces em acessibilidade.js/componentes.js/areas.js. Só alterar consumidor depois de liberação da base pelo Maestro.
- [ ] Ler contrato/spec/matriz e fontes; identificar IDs atendidos sem duplicar42.
- [ ] Integrar guardarFoco/restaurarFoco/anunciadores/busy nos fluxos completos, preservando estado sem usar nomes remotos como seletores.
- [ ] Corrigir sentido da permissão e pós-pareamento, estado e recuperação honesta, bloqueio/expiração; manter permissões/protocolo.
- [ ] RED/GREEN de geometria quatro lados/multimonitores/fora de eixo; corrigir só defeito demonstrado. Prévia deve nomear efeito real.
- [ ] Reordenar ação antes do canvas; oferecer equivalente sem arrasto; tratar Home/End/pointercancel/alvos/labels em sua própria área.
- [ ] Testes Node dirigidos de comportamento puro, sem DOM fictício volumoso. Nenhum servidor/browser externo.
- [ ] Relatório /tmp/clipsync-s9-ux-report.md com arquivos, RED/GREEN, IDs, dependências e limites. Revisão independente e portais Acess/UX após diff estável.

### Task 2: Preferências/onboarding e módulos
**Arquivos:** onboarding.js/preferencias.js e CSS próprio, api.js; testes próprios. Ampliação B09: js/app.js e js/estado.js somente aviso persistente/integração app_state.pause_warning; telas/clipboard.js somente aviso prévio de restart pausado e resposta effects/observed do modo. Demais base/consumidores ficam com seus donos. Backend/mocks continuam com Rivet.
**Dependência satisfeita:** contrato/schema C01 publicado, mocks18 testes verdes; base aprovada apósFIX1. B09 backend em implementação, schema§15.8 fixado; coordenar fixture/status com Rivet pelo Maestro. Task2 liberada para builder /root/preferencias.
- [ ] Switches em rascunho, aplicar explícito sem recriar identidade; refletir estado/falhas parciais reais.
- [ ] Retorno ao onboarding edita módulos do perfil existente; não reinit.
- [ ] Atalhos honestos/plataforma/nomes/estados/foco, sem anunciar função indisponível.
- [ ] Integrar simulador já preparado pelo dono backend; não fabricar sucesso no consumidor.
- [ ] Mostrar aviso persistente de pausa/retomada não confirmada/persistida, sem inferir paused=true em erro ou apagar por polling; usar campo opcional app_state.pause_warning. Clipboard explica restart pausado e distingue configuração salva de aplicação/restart externo; efeitos parciais levam à reconsulta. Não roubar foco nem duplicar anúncio.
- [ ] Testes de comportamento e integração da API/mocks em sua posse, sem DOM fictício volumoso; relatório /tmp/clipsync-s9-preferencias-report.md, revisão independente e portal coordenado. Não rodar suíte inteira enquanto Task1 UX tem REDs em andamento.

## Registro
- Base d41aa65. Docs Quill preservados, contrato do Maestro.
- Task1 em fix round1 após três Important; Task2 possui arquivos disjuntos e schema estável, liberada. Nenhuma edição de base pelos dois donos simultaneamente.
