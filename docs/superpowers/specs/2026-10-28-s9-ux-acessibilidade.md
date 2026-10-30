<!-- Especificação derivada das auditorias e da autorização de execução 2/3/4 de João. -->
# S9 · especificação de correções UX e acessibilidade

Data: 28/10/2026. Arquiteto: /root/contrato. Base de código inspecionada: d41aa65, interface da v0.3.1. Proposta para o Maestro integrar em entrada nova do contrato; não altera os 16 documentos já preparados por Quill e liberados ao Maestro. Os IDs e as ressalvas da auditoria estão em /tmp/clipsync-correcao-cobertura.md.

## 1. Escopo e decisões

João autorizou executar as correções. Esta proposta fecha o desenho necessário sem pedir nova autorização genérica. Permanecem fora: incidente recente do cursor, backlog anterior, protocolo novo, concessão de controle de saída, telemetria de hooks, atualização do Figma, release e teste físico automático.

Prioridade: seis P1 UX (módulos; sentido da permissão; atalhos honestos; computador correto para configurar; prévia direcional; ação visível em 960) e seis fluxos P1 de foco. Corrigir também as causas confirmadas de contraste, reflow, semântica e edição sem arrasto. As 42 entradas brutas de acessibilidade não são 42 defeitos independentes.

Decisões:
1. Resolver a recuperação de módulos de verdade em Preferências. Alterar os dois módulos não recria identidade nem pareamentos. Manter pelo menos um módulo ligado, como o perfil atual exige.
2. Manter posição + confirmação como caminho simples. Monitor, borda, coordenadas e trechos ficam no avançado. Nada concede controle por inferência.
3. Dizer "este computador" e o nome do outro computador nos fluxos comuns. Manter o menu Dispositivos; "agente externo" permanece onde distinguir quem iniciou o processo é necessário.
4. Configurado, eventos recebidos e passagem física são evidências diferentes. Não adotar a manchete "Funcionou" proposta por Lens m11: recebimentos não provam ida e volta.
5. Manter o limite de pareamento atual nesta correção. Acrescentar aviso e recuperação; não declarar exceção normativa "Essencial" apenas por ser um mecanismo de segurança. Uma ampliação do prazo precisa de decisão específica de segurança/produto.
6. Não converter a tabela em grid por Access P2-1. A alegação normativa foi refutada e a seleção por Enter/Espaço manteve foco na auditoria. A fala da seleção segue verificação manual.
7. Ampliar texto no app real exige o caminho de zoom completo, não apenas resize do simulador. Proposta de exceção mínima às capabilities: única permissão core:webview:allow-set-webview-zoom restrita à janela main, necessária ao polyfill Tauri no Linux, junto a zoomHotkeysEnabled. Sem dependência nova, URL remota ou comando genérico.
8. Nenhum resultado desta rodada será chamado de conformidade WCAG ou prova de travessia Windows/Linux. Cada veredito cita método e alcance.

## 2. C01: módulos depois do onboarding

O desenho final está em `2026-10-29-s9-modulos.md` e contrato §15.7, após revisão Sentinel. Substitui a proposta inicial: config NÃO possuía lock; agora a transação exige locks de operação/config, posse/readiness próprios, efeitos parciais e reinício sempre pausado. UI consome `{changed,modules,effects,observed}` e detalhes de erro honestos.

Preferências terá dois switches de rascunho e Aplicar explícito, sem recriar identidade/pares. Voltar do onboarding ao passo de módulos edita perfil existente, nunca init/recreate. Identidade/nome já criados são somente leitura nesse retorno. Não anunciar edição como aplicada antes da resposta; erros podem ter efeitos de pausa/parada anteriores à gravação. Retomar após reinício é explícito.

## 3. C02/C03/C12: sentido, estado e recuperação

Permissão de entrada com nome explícito: "{nome} pode controlar o teclado e mouse deste computador", descrição dizendo conceder no computador que receberá controle e mantendo "Confiança total". O sentido oposto aparece como orientação para habilitar no outro computador; o controle local fictício continua indisponível. Foco inicial do diálogo em Cancelar. Nenhum Enter inicial concede confiança.

Sucesso de pareamento e painel de permissão não inferem papel persistente de input_in. Se a ação acabou de conceder entrada, a orientação principal aponta ao computador com o mouse e explica o que fazer lá. Configuração local tem CTA inequívoco "Configurar saída deste computador"; Layout informa "A passagem sai deste computador para {nome}". Ambos podem ser origem em momentos diferentes; não bloquear Layout de um receptor por suposição.

Preferências não usa global_shortcuts como prova de atividade. Mostrar disponibilidade por ação e condição observada (módulo desligado, sem ligação, pausado, retomada recusada). Remover chips de atalhos que não existem naquela plataforma/ação e o chip de Retomar quando resume_hotkey_refused. Abrir e Enviar uma vez seguem indisponíveis até no Windows. Não inventar registro de hooks.

Recuperação de processo parado: um aviso principal com uma ação, sem três avisos repetidos. Dispositivos e estado global reconsultam em agent_state e resync, com guarda de geração para resposta antiga não desfazer a nova. Não pintar todos conectados a partir de "iniciou"; usar devices_list real. Manter distinção entre offline e processo parado.

Par bloqueado: permissões não são editáveis na UI, com motivo e caminho Reparear. Usar os estados reais do backend; simulador não pode conceder confiança em par bloqueado. Permissões iniciais aparecem só quando o código foi confirmado. Clipboard desligado localmente tem explicação local, sem culpar lado desconhecido.

Protocolo antigo tem selo discreto na linha do par e explicação nos detalhes. Não anunciar versão exata de release que não foi transmitida. Diagnóstico nunca cria células de latência vazias quando a capacidade vier a existir: valor real ou "sem medição"; a capacidade atual permanece indisponível.

Pareamento orienta endereço alcançável: "Os dois computadores precisam se alcançar pela rede. Digite na outra máquina um destes endereços." Não classificar 10.x como VPN nem 192.168.x como rede física só pelo prefixo; não alterar porta/discovery. Onboarding usa texto de Windows/Linux conforme plataforma, sem portal Linux no Windows. Teste local é explicitamente opcional e não prova rede.

## 4. C04/C05/C10: passagem simples e avançado

Prévia mostra origem, destino e lados em português: "O mouse sai pela borda inferior de LG… deste computador e entra pela borda superior de eDP… de {nome}". Sinalizar as mesmas bordas no canvas sem depender só da cor. Manter apenas uma confirmação final, e não duplicar toast mais estado equivalente.

A geometria diagonal relatada por Lens é um subcaso a reproduzir antes de alterar a regra. Não centralizar sempre no conjunto de monitores: isso regrediria a mesa Dell + LG com notebook abaixo do LG. Ao clicar uma posição, preservar alinhamento perpendicular que já corresponde a um monitor externo; se o destino está completamente fora desse alinhamento, projetá-lo para a borda candidata real mais próxima. Calcular a proposta e a prévia sobre a mesma posição final, preservando trecho livre, bordas internas, cantos mortos, revisão e múltiplas ligações avançadas. Teste puro deve cobrir a mesa Dell/LG, quatro lados e destino fora do eixo.

Em 960×640 (e viewport efetivo 960×632), a escolha de posição, prévia curta e ação principal ficam antes do canvas alto, em ordem DOM e visual coerentes. Preferir reorganização no fluxo; não criar rodapé fixo que cubra controles ao ampliar. Em 320 CSS px permitir rolagem vertical. Estado sem par mostra botão Parear, sem seletor truncado ou instrução de arrastar algo inexistente. Sem monitores: texto curto integral e caminho verificável.

Avançado ganha equivalente completo de clique simples: escolha de computador, monitor e borda em formulário; início/fim (%) tanto de origem quanto de destino; posição x/y ou botões de passo para ajuste fino. Reutilizar comandos existentes e revisão atual. A alternativa fica na mesma página e alcança todas as funções dos alvos pequenos do canvas. Pode justificar a exceção Equivalente de 2.5.8 em vez de aumentar zonas SVG e fazer bordas vizinhas se sobreporem; medir os controles alternativos (>=24×24).

Alças continuam no canvas com nome de computador/monitor/borda, descrição separada e aria-valuemin/max coerentes. Home/End usam extremos PERMITIDOS pela outra alça, cantos mortos e trechos bloqueados, nunca 0/100 cegamente. pointercancel desfaz o deslocamento visual e não grava. Preservar details abertos, seleção e scroll ao atualizar.

Teste: "Eventos recebidos no teste", contadores separados; fora de ordem não equivale automaticamente a perda. Se houver perdas, aviso específico. Zero sempre inconclusivo, com link real para Diagnóstico. Manter os 5 segundos explícitos e a advertência de que não verifica retorno, sem começar sozinho.

## 5. C06/C07: foco e feedback como infraestrutura comum

Introduzir utilitário pequeno de continuidade, com chaves lógicas estáveis (data-focus-key), em vez de restaurar pelo índice/nome remoto ou pelo novoId a cada repaint. Interface recomendada:

    guardarFoco(raiz) -> registro | null
    restaurarFoco(registro, raiz, {fallback}) -> bool
    criarAnunciador() -> {no, status(texto), erro(texto), limpar()}

O registro só restaura quando o foco da ação foi removido/desativado involuntariamente. Se o usuário mudou para outro controle conectado, outra rota ou outro diálogo, não roubar o foco. A consulta usa chaves próprias estáveis; não interpolar nome/ID remoto em seletor CSS sem escape. Preferir manter o nó interativo existente; o helper cobre substituições inevitáveis.

comCarregamento mantém o botão focável enquanto ocupado, aria-busy/aria-disabled e trava de reentrada; não deve habilitar no finally um botão que passou a ser indisponível. Switch ocupado bloqueia nova ativação, preserva valor confirmado e volta em falha sem loop de change. Estado "não disponível" usa disabled real e motivo associado, distinto de "ocupado".

abrirDialogo registra gatilho e aceita fallback lógico. Ao fechar por cancelar, confirmar ou caminho nativo de cancel, restaura gatilho conectado após liberar ocupação; se Revogar removeu a linha, próxima linha ou Parear. Diálogos continuam modais e cancel não interrompe gravação em andamento.

Anunciadores existem vazios antes dos resultados e permanecem montados. Erros de nome/endereço e falhas de gravação são assertivos; carregamento inicial, código em dígitos separados, conclusão, teste e salvamento são polidos e únicos. Não colocar o código em aria-label de paragraph. Não repetir anúncios em polling silencioso nem alterar foco quando apenas chega estado de fundo. Na mudança de etapa, foco no título/primeiro controle lógico; endereço inválido permanece no campo com valor preservado.

Seis aceites independentes:
F1: Enter + oito setas no mesmo computador do canvas funcionam sem reencontrar o grupo.
F2: Cancelar/fechar/confirmar nos quatro diálogos auditados volta ao gatilho ou próximo alvo lógico; nenhum retorno involuntário ao BODY.
F3: ações com latência 160, 500 e 1400 ms, incluindo teste de onboarding, preservam foco e só disparam uma requisição.
F4: endereço inválido, chegada/confirmar código, troca de modo, alteração de permissão e Tentar de novo no Diagnóstico conservam valor/foco ou avançam logicamente, com anúncio único.
F5: Escape/Cancelar/Salvar no editor de borda volta à borda equivalente; se removida, ao computador ou formulário correspondente.
F6: Ligar e Testar conservam ação/resultado lógico, details e scroll; erro parcial continua distinguindo posição salva de ligação não confirmada.
Contraexemplo obrigatório: usuário leva foco a outro controle enquanto espera; a resposta não o puxa de volta.

## 6. C08/C09/C11/C14/C15: semântica e apresentação

- Estados concluídos do onboarding têm texto assistivo. Estado lateral e prévia rolável do suporte têm papel que admite nome. A expansão de Clipboard liga aria-expanded a alvo com id. Nome acessível de campo mantém o texto visível e unidade; instruções longas ficam em descrição.
- Canvas expõe seleção por aria-pressed no grupo-button, independente do foco. Para Pausar/Retomar e Travar/Destravar, manter rótulos de ação variáveis e retirar aria-pressed ambíguo; estado atual aparece no status textual. Não mudar papéis da tabela sem prova.
- Token próprio de contorno de controle alcança pelo menos 3:1 contra fundos adjacentes nos dois temas; não escurecer todas as linhas decorativas. Foco de borda e grupo local têm forma claramente distinta do repouso; segmentado selecionado tem marca/contorno adicional. Manter contraste de texto existente.
- Conter captions e texto somente assistivo em ancestral adequado. Janela não ganha rolagem por caption invisível; conteúdo longo continua rolável. Em largura estreita, navegação compacta e conteúdo não competem em três contêineres de rolagem. Segmented e ações de Clipboard quebram/reorganizam sem corte; exceções de tabela/canvas não justificam overflow da página inteira.
- Rótulos de máquinas ficam na própria área, não cobrem monitores de outra máquina; abreviar visualmente sem perder nome completo acessível e disponibilizar nome integral no painel. Tratar "sem monitores" integral. Medir grupos próximos, nomes longos e dois temas.
- Polimento depois dos funcionais: perigo contornado fora do diálogo, preenchido na confirmação final; título e switch do onboarding compartilham linha; largura de kv permite leitura sem coluna rígida desnecessária; alinhar ações discretas, direção dos ícones e polegar do switch com contraste suficiente. Dobra de conteúdo longo, tipografia diferente do Figma e capturas de dados distintos não são falhas por si.
- Zoom: habilitar zoomHotkeysEnabled e a permissão estrita descrita na decisão 7. Não mudar CSP nem acesso de rede. Validar Ctrl+/Ctrl-/restauração na versão real em Linux e Windows; opção de sistema não será presumida. MinWidth=960 não substitui teste de reflow com viewport CSS reduzida.

## 7. C13: prazo do pareamento

Contagem visual pode atualizar sem live a cada segundo. Anunciar uma vez quando faltarem 30 segundos, além de expiração. Oferecer recomeçar com nova tentativa e código, preservando texto não sensível do endereço, mas descartando código e grants da tentativa anterior. Não prolongar nem reutilizar o código atual. Janela atual continua 120 segundos.

Isso melhora previsibilidade e recuperação, mas NÃO fecha por si só 2.2.1. A adequação do limite/exceção fica registrada como pendência normativa de segurança/produto; não esconder a ressalva em "corrigido".

## 8. Posse proposta e ordem de execução

O Maestro designa pessoas, mantendo um editor por arquivo. Nomes abaixo são frentes, não convite a recrutar agentes.

| Frente | Posse exclusiva | Dependência/aceite |
|---|---|---|
| Backend de módulos | crates/clipsync-core/src/profile.rs e testes de perfil; clipsync-ui/src/{api.rs,lib.rs,agent.rs,modules.rs novo,clipboard.rs,msg.rs}; status.rs/layout.rs somente se necessário para a coordenação de pausa/trava; testes Rust correspondentes; ui-dev/mock.js e ui-dev/mock/* para o comando/ciclo de vida | Publicar schema antes do frontend; SEG revê preservação e falha parcial. CLI não precisa de comando novo para fechar o fluxo pedido. |
| Acessibilidade e base visual | ui/js/{acessibilidade.js novo,componentes.js,areas.js,dom.js,app.js,estado.js}; ui/index.html; ui/css/{tokens.css,components.css,app.css}; ui/js/telas/{clipboard.js,diagnostico.js}; CSS dessas duas telas; tauri.conf.json/capabilities somente após revisão Sentinel e aceite da exceção pelo Maestro | Publicar helpers de foco/anúncio antes dos consumidores. Testes próprios de política/semântica, portais. |
| UX e editor | ui/js/{layout-simples.js,travessia.js,travessia-vista.js}; ui/js/telas/{layout.js,pareamento.js,dispositivos.js,onboarding.js,preferencias.js}; CSS dessas cinco telas; ui-dev/testes/{travessia.test.mjs,oferta.test.mjs} e novos testes UX | Consome helpers de acessibilidade e modules_set. Não editar base CSS/estado.js por conta própria; enviar pedido ao dono. |
| Simulador/testes de contrato | O mesmo dono Backend tem mocks; cada builder tem teste novo de nome distinto; mock-travessia.test.mjs fica Backend | Não dois editores de mock.js/dados.js. Adicionar cenários de latência/erros sem script de automação de navegador. |
| Documentação | Maestro /root assume contrato.md e plano S9 após liberação de Quill; demais documentos só com posse combinada | Preservar histórico dos vereditos e os 16 diffs já existentes. |
| Revisão independente | Sem posse de produto; relatório de evidência e achados | Segurança cobre módulos/consentimento; warden repete os seis fluxos e amostras visuais. |

Ordem: contrato de interfaces -> helpers e módulos em paralelo -> consumidores UX -> revisão/portal -> integração de documentação com fatos observados. Não é preciso bloquear correções de foco aguardando fim do backend.

## 9. Verificação e limites

Automatizado: testes de comportamento dos helpers puros e regressões geométricas sem biblioteca nova nem DOM fictício apresentado como navegador; testes Rust de perfil/API/lifecycle com adapters fake. node --test crates/clipsync-app/ui-dev/testes/*.test.mjs; cargo fmt --all --check; cargo test/clippy --workspace --exclude clipsync-app --locked, scripts de unsafe/privacidade e cross-clippy de Windows como AGENTS. O app Tauri continua dependendo do CI para compilar nesta máquina.

Portal exclusivo por fluxo que muda estado, usando runtime existente do Maestro. Registrar innerWidth/innerHeight reais, cenário, tema, activeElement, DOMRect e scrollWidth. Casos: padrao, vazio, sem-perfil, input-desligado, sem-ligacao, receptor, windows, windows-recusa, protocolo-antigo, teste-zero, erro, estados, sem-monitores. Repetir foco nos dois temas e nas duas plataformas simuladas. Medidas essenciais em 1280×800, 960×640, 640×400 e 320×256; registrar quando a altura efetiva difere.

Não usar Playwright, servidor adicional, perfil real, aprovação de portal do sistema, captura real ou teste físico nesta tarefa. Echo mostrou teclas sintéticas isTrusted=false; esse método não certifica focus-visible, Escape nativo, Space nativo ou leitor de tela. Simular requestClose e click prova somente esses caminhos.

Manuais pendentes: Orca/NVDA (anúncio único/nome/seleção), Tauri/WebKitGTK/WebView2 (zoom e teclado nativo), alvos/ponteiro assistivo, passagem física de mouse, consistência das telas escuras ainda não cobertas e onboarding 960. Sem acesso, registrar "não verificado" com causa, não "passou".

Fontes primárias consultadas nesta proposta:
- [W3C: alternativa de ponteiro sem arrasto](https://www.w3.org/WAI/WCAG22/Understanding/dragging-movements.html). Teclado sozinho não substitui essa alternativa.
- [W3C: tamanho mínimo do alvo](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html). 24 CSS px e exceções de espaçamento/equivalente.
- [WAI-ARIA 1.2: row](https://www.w3.org/TR/wai-aria-1.2/#row). Permite contexto table e lista os estados suportados.
- [W3C: ajuste do tempo](https://www.w3.org/WAI/WCAG22/Understanding/timing-adjustable.html). Aviso sem extensão não prova o critério; segurança isolada não implica exceção.
- [Tauri: referência de configuração](https://v2.tauri.app/reference/config/), conferida com a fonte local fixada tauri-utils-2.10.2/src/config.rs:2188 e tauri-2.12.5/src/webview/mod.rs:1112.

## Decisão do Maestro e faseamento
A proposta é adotada para correção dos achados comprovados, com execução em fatias de posse exclusiva. C01 (módulos/ciclo de vida) e C14 (capability de zoom) passam pela revisão de segurança antes da implementação. A base de foco, feedback, semântica, contraste e reflow C06–C11/C15 pode avançar independentemente, sem editar os consumidores de UX atribuídos à fatia seguinte. Não adicionar lógica de produto para hipóteses sem reprodução. Incidente físico de cursor está autorizado para investigação no andar TESTES, mas não é declarado corrigido por esta especificação.
