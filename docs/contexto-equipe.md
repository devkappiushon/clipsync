# ClipSync: contexto comum dos andares

## O que estamos construindo

ClipSync compartilha teclado, mouse e área de transferência entre computadores do usuário, sem servidor central, para substituir o Input Leap. Rust, workspace com oito crates e desktop em Tauri 2. Alvos atuais: Windows e Linux GNOME Wayland. O usuário quer configurar a posição dos computadores com poucos passos, sem escolher bordas obrigatoriamente.

## Fonte de verdade

Repositório central: `/home/kappxs/Documentos/ANTIGOS/clipsync`. Leia primeiro `docs/handoff-codex.md` para a história e depois `CONTEXTO.md` e `docs/workboard.md` para o estado atual. O handoff antecede a S8; a evidência mais recente prevalece. Consulte `AGENTS.md`, `docs/contrato.md` (§14 para S8), `docs/verdicts.md` e `docs/seguranca.md` antes de alterar comportamento.

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

As notas locais preservam seus modelos para trabalhos futuros. A conexão e o contexto não abrem tarefas de implementação ou auditorias. Antes de operar o canvas, confirme em `maestri list` o nome do seu próprio terminal; uma identidade antiga após troca de agente não deve ser usada para alterar outro time.
