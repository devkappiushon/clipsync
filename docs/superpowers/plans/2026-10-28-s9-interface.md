# S9 · plano da base de interface acessível

> Para agentes: superpowers:subagent-driven-development, TDD e revisão independente. João autorizou executar correções e testes; nenhum commit nesta rodada.

**Objetivo:** corrigir continuidade de foco, feedback, semântica, contraste e reflow compartilhados antes dos consumidores UX.
**Arquitetura:** helpers pequenos, regiões persistentes e chaves de foco próprias; consumidores existentes mantêm contratos até a integração seguinte.
**Stack:** JavaScript/CSS local, Node somente testes, sem bibliotecas novas.
**Spec:** docs/contrato.md §15.4 e docs/superpowers/specs/2026-10-28-s9-ux-acessibilidade.md §§5–6; rastreabilidade na matriz de cobertura.

## Global Constraints
- Sem innerHTML, style=, scripts inline, travessão em UI, fonts remotas ou dependência nova; dados do par por textContent.
- Sem Playwright ou automação de navegador externa: validação visual por portais Maestri coordenados pelo root.
- Nenhum servidor, perfil real, diálogo de permissão, commit/push, mock de UI, Rust ou capability nesta fatia.
- Um escritor por arquivo; manter intactos os arquivos das demais frentes.
- Contraste de contorno de controle >=3:1, dois temas. Não confundir resize, isTrusted=false ou DOM com leitor de tela/zoom real.

## Review Focus
- Foco movido pelo usuário durante espera não é roubado pela conclusão.
- Duplo Enter/click durante busy dispara uma requisição, mantendo foco e indisponibilidade real.
- Fechar modal após remover o gatilho usa próximo alvo lógico; mudança de rota não restaura foco velho.
- Atualização silenciosa não duplica anúncios; erros preservam valor e têm região viva persistente.
- Conteúdo largo, caption assistivo e controles a 320 px não deslocam a navegação nem escondem ações.

### Task 1: infraestrutura e telas comuns
**Arquivos:** posse exata da §15.4: base JS/CSS/HTML, Clipboard e Diagnóstico e seus CSS; testes novos de nome exclusivo em ui-dev/testes. Não editar os consumidores UX (Layout, Pareamento, Dispositivos, Onboarding, Preferências), mocks ou Rust.
**Interfaces propostas:** guardarFoco(raiz), restaurarFoco(registro, raiz, {fallback}), criarAnunciador() -> {no,status,erro,limpar}; ajuste nomes só se necessário ao padrão existente, documentando assinatura final aos consumidores. Manter a API existente de componentes compatível.
- [ ] Ler fontes, spec e matriz; identificar comportamentos confirmados C06/C07/C08/C09/C11/C15 na posse.
- [ ] Escrever regressões de comportamento da política de foco/busy/retorno e das funções puras; ver RED antes de implementação. Não criar um DOM fictício volumoso nem chamar isso de teste de navegador. Os seis fluxos completos serão validados por portal na integração.
- [ ] Implementar helpers, busy sem foco perdido/reentrada, retorno de diálogo e anunciadores persistentes. Cobrir escolha explícita do usuário de mover foco e indisponibilidade posterior ao início.
- [ ] Integrar Clipboard/Diagnóstico, sem perder dados/ações, e ajustar semântica/nomes/expanded/estados nos arquivos da posse.
- [ ] Corrigir contorno dos campos, seleção adicional e reflow/caption. Preservar contraste de texto, scroll legítimo, dados longos e fluxo simples.
- [ ] Rodar node --test dos testes afetados e testes existentes pertinentes; greps das invariantes de UI e git diff --check.
- [ ] Entregar /tmp/clipsync-s9-base-ui-report.md com interfaces finais, arquivos, RED/GREEN, IDs cobertos, casos ainda dependentes dos consumidores e limites visuais.
- [ ] Revisão independente de spec e qualidade por warden; portal pelo andar Acess/UX após runtime do Maestro. Corrigir achados importantes antes de declarar concluído.

## Registro
- Base d41aa65; docs de Quill preservados e posse liberada.
- Decisão do Maestro: antecipar a base acessível enquanto o desenho das corridas H-01/B-02 recebe ajuste; arquivos disjuntos. Não declarar seis fluxos resolvidos só pelo helper.
- Continuação: consumidores UX/módulos terão plano próprio; C14 zoom aguarda segurança.

### Task 2: zoom do app real (C14)
**Arquivos:** apenas crates/clipsync-app/tauri.conf.json e capabilities/default.json, posse Rivet.
**Contrato:** §15.5; aprovação técnica Sentinel com condições.
- [ ] Adicionar zoomHotkeysEnabled=true à janela main; acrescentar somente core:webview:allow-set-webview-zoom às permissões existentes.
- [ ] Preservar windows=[main], remote ausente, CSP, deps e demais configurações/permissões.
- [ ] Validar parse JSON e diff exato, sem testes que espelham configuração. Não compilar Tauri indisponível nem abrir perfil real.
- [ ] Relatar /tmp/clipsync-s9-zoom-report.md; verificação física dos atalhos/zoom permanece pendente. Revisão independente de diff antes de fechar C14. Sem commit.
