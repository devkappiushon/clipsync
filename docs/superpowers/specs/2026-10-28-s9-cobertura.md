# Matriz de cobertura das auditorias

Data: 28/10/2026. Fontes: /tmp/clipsync-uxui-fecho-final.txt (429 linhas numeradas), /tmp/clipsync-acessibilidade-para-ux-final.txt (472 linhas numeradas), CSV Pixel /tmp/clipsync-pixel-ui-continuacao-2026-10-28.csv. Os números físicos incluem cabeçalho. Base d41aa65; esta matriz não é uma nova execução dos portais.

Contrato proposto: /tmp/clipsync-correcao-contrato-proposto.md. A matriz cobre 42 IDs brutos de acessibilidade, sem somar recibos Argus nem repetir tickets por autor. Prioridade abaixo conserva a origem; a ordem de implementação agrupa por causa. Nenhuma linha significa "corrigido".

## Causas e donos sugeridos

| Causa | Trabalho | Dono proposto |
|---|---|---|
| C01 | Módulos editáveis sem recriar perfil; retorno do onboarding | Backend + UX, arquivos separados |
| C02 | Sentido do controle, local de configuração, permissão explícita | UX |
| C03 | Atalhos/estados honestos, recuperação, bloqueados, protocolo, métricas | UX + A11y/base, por tela |
| C04 | Prévia da passagem e resultado sem falsa conclusão | UX/editor |
| C05 | Prioridade visual da ação e estados vazios do Layout | UX/editor |
| C06 | Continuidade de foco e reentrada, seis aceites F1–F6 | A11y/base + consumidores UX |
| C07 | Regiões persistentes de feedback, erros e conclusão | A11y/base + consumidores UX |
| C08 | Nome, descrição, estado e relações semânticas | A11y/base + consumidores UX |
| C09 | Contorno, foco e seleção distinguíveis | A11y/base, editor nos SVG |
| C10 | Editor avançado sem arrasto, alvos equivalentes e cancelamento | UX/editor |
| C11 | Reflow, caption fora do contêiner e cortes do Clipboard | A11y/base |
| C12 | Orientação por plataforma, rede e terminologia | UX |
| C13 | Aviso e recuperação do prazo de pareamento; adequação normativa pendente | UX; SEG decide qualquer alteração de prazo |
| C14 | Ampliação Tauri completa; exceção mínima de capability proposta | A11y/base após revisão Sentinel |
| C15 | Rótulos do canvas e polimento visual confirmado | UX/editor + A11y/base por arquivo |

## Inventário bruto A11y, 42 entradas rastreadas

Legenda: R = reproduzido no simulador/DOM pelos autores; L = confirmado por leitura; H = efeito/requisito ainda não verificado; X = alegação rejeitada. R nunca significa fala assistiva, Tauri ou máquina física. As referências § apontam ao contrato proposto.

| ID de origem | Pri. origem | Causa | Evidência e decisão de cobertura |
|---|---|---|---|
| Access/P1-1 | P1 | C06 F1 | R: Echo E3 e Argus A1; nó SVG removido a cada tecla. Manter peer_id lógico e oito movimentos contínuos, §5. |
| Access/P1-2 | P1 | C06 F2/F3 | R em diálogo/latência longa por Echo E1/E2; NÃO reproduzido em pausa curta. Corrigir busy/reentrada/restauração sem roubar foco; testar ambos os resultados. |
| Access/P1-3 | P1 | C06 F4 + C07 | R endereço inválido e confirmação de código; Clipboard/permissões L. Um trabalho com subcasos e aceites separados, §5. Fala ainda H. |
| Access/P1-4 | P1 | C06 F5 | R Echo E4/Argus A3: Escape próprio remove borda/popover. Voltar à borda equivalente, sem atribuir prova de Esc nativo. |
| Access/P2-1 | P2 | C08, sem correção presumida | X alegação de aria-selected proibido em row/table; WAI-ARIA admite esse estado. Manter tabela; fala da seleção H. Não impor grid/roving Tab. |
| Access/P2-2 | P2 | C09 | L contraste de foco da borda 1,11:1; :focus-visible real H. Indicador distinto com contorno, medir nos dois temas, §6. |
| Access/P2-3 | P2 | C08/C10 | R DOM por Argus A5: campos de/a sem contexto. Origem/destino com legend e início/fim em %, §4/§6. |
| Access/P2-4 | P2 | C08 | L conclusão de etapa só visual. Texto assistivo concluído; atual mantém aria-current, §6. |
| Access/P2-5 | P2 | C08 | L nome em generic na lateral. Papel nomeável; Echo E13 é outro elemento da mesma causa, §6. |
| Access/P2-6 | P2 | C08/C03 | L motivos de disabled ausentes. Associar explicação em pareamento/notificações/copiar; não exigir Tab no disabled nativo nem reabilitá-lo, §3/§6. |
| Access/P2-7 | P2 | C09 | R cores computadas 1,663 claro/1,690 escuro; Argus A4/Echo E7/Pixel contraste duplicados. Token de contorno >=3:1, §6. |
| Access/P2-8 | P2 | C07 | L fim de carga sem região persistente; fala H. Busy na área + conclusão única em carga explícita, silêncio no polling, §5. |
| Access/P3-1 | P3 | C03 | L atalhos anunciados sem capacidade; R Lens M6 recusado. Chips por ação/plataforma e recusa; não provar atividade por flag, §3. |
| Access/P3-2 | P3 | C08 | L expansão Clipboard sem alvo. Id persistente e aria-controls, §6. |
| Access/P3-3 | P3 | C08/C10 | L instrução no nome e texto só de arrasto. Nome curto + descrição; anunciar alternativa por botões/setas, §4/§6. |
| Access/P3-4 | P3 | C03 | L ramo peer_rtt=true produz células vazias, mas capacidade atual falsa; defeito condicional, não visível atual. Usar sem medição no ramo sem dado, sem habilitar recurso, §3. |
| Access/P3-5 | P3 | C08/C10 | L slider sem contexto/Home/End. Limites permitidos reais, nome completo e unidade; não forçar 0/100, §4. |
| Meridian/N1 | P1 | C06 F6 + C07 | R por Argus A3: Ligar termina BODY. Foco/resultado lógico e feedback único; não confundir com teste-zero, §5. |
| Meridian/N2 | P1 | C06 F4 + C07/C08 | L chegada de código, R confirmação por Echo E5 são gatilhos distintos. Código em dígitos na região persistente; foco na nova etapa, §5. |
| Meridian/N3 | P2 REC | C02/C06 F2 | R foco inicial Conceder por Echo E6; recomendação APG, não falha normativa isolada. Inicial Cancelar, retorno separado, §3/§5. |
| Meridian/N4 | P2 | C10 | L lacuna confirmada: origem e posição fina sem clique equivalente. Formulário avançado completo na mesma página, §4; teclado sozinho não resolve. |
| Meridian/N5 | P2 | C10 | L zonas 10 px/alças 14 px; falha normativa depende de exceções/medida. Equivalente completo >=24×24 e verificação, §4, sem sobrepor hitboxes. |
| Meridian/N6 | P2 decisão | C13 | L 120 s sem aviso. Acrescentar aviso/recomeço mantendo código novo. NÃO fechar 2.2.1 por aviso; exceção/extensão continua decisão SEG, §7. |
| Meridian/N7 | P2 | C14 | L zoomHotkeysEnabled ausente; H ampliação real. Proposta flag + permissão main estrita, Sentinel antes de implementar. Teste Tauri por plataforma, §6. |
| Meridian/N8 | P2 | C07 | L resultado de teste de onboarding inserido sem live; fala H. Resumo único persistente, independente da correção de foco, §5. |
| Meridian/N9 | P2 | C09 | L foco da máquina local usa mesma cor do repouso. Indicador externo distinto; medida do foco real pendente, §6. |
| Meridian/N10 | P3 | C08 | L rótulo variável com aria-pressed ambíguo. Retirar pressed de Pausar/Retomar e Travar/Destravar; manter estado textual, §6. |
| Meridian/N11 | P3 | C10 | L pointercancel chama gravação. Cancelar restaura sem comando; testar separadamente de pointerup, §4. |
| Meridian/N12 | P3 medir | C09 | L seleção segmentada com contraste insuficiente de estado; H medida final. Marca/contorno além da cor, §6. |
| Echo/E1 | P1 | C06 F2 | R quatro diálogos: Revogar, Conceder, Exportar, Clipboard enviado. Duplicata parcial Access P1-2; cada retorno precisa de ensaio próprio. |
| Echo/E2 | P1 | C06 F3 | R 500/1400 ms perde foco, 160 ms preserva. Duplicata parcial Access P1-2; trava de reentrada e foco com latência, §5. |
| Echo/E3 | P1 | C06 F1 | R Enter/seta no SVG remove nó. Duplicata Access P1-1; preservação de seleção e oito passos, §5. |
| Echo/E4 | P1 | C06 F5 | R Escape do handler do popover. Duplicata Access P1-4; não é Esc nativo dialog, §5. |
| Echo/E5 | P1 | C06 F4 + C07 | R confirmação de código e Tentar de novo no Diagnóstico; duplica causa Access P1-3. Fala H, §5. |
| Echo/E6 | P2 | C02/C06 F2 | R foco inicial Conceder. Duplicata Meridian N3; Cancelar recebe foco, §3. |
| Echo/E7 | P2 | C09 | R contorno do Nome do onboarding soma amostra à de Argus. Duplicata Access P2-7, não outro ticket. |
| Echo/E8 | P2 | C11 | R resize 320: overflow Clipboard; 640×400: três rolagens. Reflow sem cortes fora de tabela/canvas, §6; NÃO prova zoom de N7. |
| Echo/E9 | P2 | C07 | R DOM erro de nome associado, foco mantido; anúncio H. Feedback assertivo persistente sem apagar valor, §5. |
| Echo/E10 | P3 | C11 | R caption de Diagnóstico desloca documento/scroll lateral. Mesma causa Pixel-2/PUI-04; conter legenda, manter scroll útil, §6. |
| Echo/E11 | P3 | C08 | R ausência de estado de seleção no g. aria-pressed no button SVG; árvore/fala H. Não é tabela Access P2-1, §6. |
| Echo/E12 | P3 | C11 | R botões Clipboard passam 3 px da área em 960. Ajustar largura/empilhamento e medir ação/foco integral, §6. |
| Echo/E13 | P3 | C08 | R pre suporte com nome mas sem papel. Segunda ocorrência Access P2-5; papel nomeável sem perder scroll teclado, §6. |

Contagem: 17 Access (4+8+5), 12 Meridian (N1–N12), 13 Echo (E1–E13) = 42. Argus A1–A5 são recibos das linhas acima, não achados adicionais. "Seis P1" descreve fluxos de foco, não soma dos autores.

## UX/UI: cobertura sem criar novas permissões ou copiar desenho antigo

| IDs de origem | Causa/prioridade atual | Decisão e critério |
|---|---|---|
| Estratégia A1 + Revisor R1 + PUI-09 | C01, P1 UX 1 | modules_set e cartão Preferências; preservar identidade/pares/config/pausa/trava; erro parcial explícito. CTA tem destino executável. |
| Estratégia A2 | C02, P1 UX 2 | Nome do outro computador e sentido explícito nos dois painéis. Confiança total continua; controle inverso não ganha capacidade/protocolo. |
| Revisor R2 | C03, P1 UX 3 | Disponibilidade por ação; nenhum selo "ativos" só por feature. Sem ligação, pausa e recusa explicados. |
| Lens M1 | C02, P1 UX 4 | Configurar localmente tem rótulo inequívoco; quem acaba de conceder entrada recebe instrução para o computador com mouse. Não inferir papel permanente. |
| Lens M2 | C04, P1 UX 5 | Prévia nomeia borda de saída/entrada e destino; destaca lados. Diagonal relatada precisa reprodução específica antes de mudança geométrica; preservar mesa Dell/LG. |
| Lens M3 | C05, P1 UX 6 | Escolha/prévia/ação antes do canvas alto; visível sem rolar em 960×632/640, sem sobrepor no zoom. |
| Revisor R3 + Lens M5 | C12, P2 | Descrição Windows não menciona portal/X11; manter permissões de sistema com usuário. |
| Pixel-1 + PUI-03 + Lens m9 | C15, P2 do coordenador | Uma causa: rótulo invade monitor alheio. Caixa/linha reservada, nome completo acessível; medir claro/escuro e 960/1280. P1 de Pixel conservado como divergência histórica. |
| Pixel-2 + PUI-04 | C11, P2 do coordenador | Duplicata Echo E10, caption/scroll externo. Não eliminar scroll legítimo para caber no Figma. |
| Estratégia A3 + Lens m4 + Revisor E2 | C12, P3 | Ajuda sobre endereço alcançável. Rejeitar rótulo "VPN/rede local" inferido de IP e LAN como exigência exclusiva. Porta/discovery inalterados. |
| Estratégia A4 | C01, confirmado | Voltar após criar perfil usa edição de módulos, não recreate. Nome/identidade existentes não viram candidato novo. |
| Estratégia A5 | C12, editorial | Computador e passagem no caminho simples; termos técnicos no avançado/diagnóstico quando necessários; menu Dispositivos mantido. |
| Lens M4 | C03, P2 | Uma faixa principal; recarregar lista/global em agent_state/resync; descartar resposta velha. Contradição física ainda H, simulador confirmado. |
| Lens M6 | C03, P2 | Recusa aparece junto a Retomar, chip removido, botão funcional preservado. Duplicata parcial Access P3-1. |
| Lens m1 | C02/C03, P3 | Título "Passagem deste computador para {nome}", sem direção invertida. |
| Lens m2 | C04/C07, P3 | Uma confirmação pós-salvar; sem duplicata persistente com estado calculado. |
| Lens m3 | C02, P3 | Permissões iniciais só após código confirmado; explicação da próxima etapa antes disso. |
| Lens m5 + Pixel-3 | C15, P3 | Cancelar comparação com contorno; perigo preenchido na confirmação final. Não mudar significado ou defaults. |
| Lens m6 | C03, P3 | Explicar clipboard local desligado quando conhecido; sem atribuir falha ao par sem dado. |
| Lens m7 | C03, P3 | Bloqueado não edita grants pela UI; motivo e Reparear. Simulador precisa refletir recusa. Sem restaurar confiança por toggle. |
| Lens m8 | C05, P3 | Vazio com Parear, sem seletor truncado nem instrução de arrasto inexistente. |
| Lens m10 | C03, P3 | Selo de protocolo antigo por linha + consequência nos detalhes, sem versão de app inventada. |
| Lens m11 | C04, P3 | Recebidos e fora de ordem/perdidos separados. Não adotar "Funcionou"; retorno físico segue pendente. |
| Lens m12 | C04, P3 | Diagnóstico vira link/ação no teste inconclusivo. |
| Lens m13 | C05/C15, P3 | "Sem monitores" integral no canvas/painel. |
| Lens N1 | C12, NIT | Onboarding descreve posição do computador, sem exigir escolher bordas. |
| Lens N2 | C12, NIT | Teste local explicitamente opcional; Concluir sem teste é compreensível. |
| Lens N3 | C06 F6, NIT | details e seleção mantidos ao atualizar posição, sem apagar foco/estado. |
| Pixel-4 | C15, P3/observação | Painel longo pela S8 não é defeito por dobrar. Reduzir duplicatas e manter Revogar alcançável, sem ocultar dados necessários. |
| Pixel-5 | C11/C15, P3 | kv adaptável, rótulo/valor legíveis; número de linhas diferente do Figma não é falha isolada. |
| Pixel-6 | C15, P3 | Título e interruptor alinhados em linha com descrição abaixo; mantém nome e alvo. |
| Pixel-7 | C15, NITs | Conferir ações discretas, chevrons, thumb e segmentado; aplicar consistência onde confirmada. Fonte de cabeçalho e aviso sem ícone adicional não são falha por si; selos já têm forma+texto. |
| Revisor E1; Estratégia C2/C3 | Critério proposto, não bug | Não impor "uma entrada técnica" ou "dois cliques" como requisito. Preservar destino/posição/confirmar, com início da jornada declarado. |

## Preservações e hipóteses rastreáveis

| Origem | Destino |
|---|---|
| PUI-01, PUI-02 | Preservar estado conservador e caminho simples/avançado S8. |
| PUI-05, PUI-06, PUI-07 | Preservar grants desligados, capacidades indisponíveis e alternativas reais; não copiar Figma que promete função ausente. |
| PUI-08 | Dados/texto diferentes do Figma não constituem defeito visual demonstrado. |
| PUI-10 | Preservar teste local/loopback; não inserir aprovação de rede fictícia. |
| PUI-11 | Empilhamento responsivo é adaptação; M3/E8/E12 têm aceites específicos, sem condenar toda dobra. |
| PUI-12 | Validar sete telas escuras ainda sem nova prova: onboarding1/2/3, pareamento, clipboard, diagnóstico, preferências. |
| PUI-13 | Validar onboarding três passos em 960×640; nenhuma aprovação inferida da captura1280. |
| Estratégia Q1–Q6/H1–H5 | Hipóteses de entendimento/mesa/endereço; observar com usuário. A falta de controle de módulos é confirmada e tratada, não só hipótese H4. |
| Lens diagonal M2 | Reprodução focal antes de mudar alinhamento; não regredir Dell/LG publicado. |
| Meridian N6 | Prazo segue120s; aviso/recomeço não são declaração de conformidade. Decisão específica SEG se adequação normativa exigir extensão. |
| Meridian N7, leitores, Tauri | Método real pendente: resize e DOM não demonstram zoom/screenreader/tecla nativa. |
| Quadro Figma Componentes3:2 | Sem captura válida, não usar como aprovação/reprovação. |
| Argus após Echo | Precisa revisão independente dos novos recibos; esta leitura não reexecutou seus casos. |
| Mouse físico/Windows e incidente recente | TESTES com usuário, fora desta implementação; não concluir por simulador/unit test. |

## Aceite de fecho da rodada

1. Os seis P1 UX têm percurso com começo/fim verificável; módulos preservam identidade, pares e estado inclusive nos erros.
2. F1–F6 de foco executados separadamente, incluindo caso de latência curta que já preservava foco e usuário que muda foco durante espera.
3. Contorno nos dois temas, reflow e alvos/equivalentes medidos com cenário/viewport efetivo. Sem deduzir conformidade geral.
4. Todos os 42 IDs têm um destino acima. X/H/decisão pendente permanecem explícitos; ninguém converte "não reproduzido" em "passou".
5. Revisão de segurança antes de aplicar módulos/zoom; warden confronta execução final. O Maestro registra documentos em entrada nova e preserva vereditos históricos.
