# Pendências da v0.5.0

Lista única do que ainda está aberto depois da publicação da v0.5.0 (31/10/2026, commit `590c137`). Consolidada em 03/11/2026 a partir de `CONTEXTO.md`, `docs/pendencias-v0.4.0.md`, `docs/workboard.md`, `docs/contrato.md` §16, `docs/seguranca.md` e das notas da versão. Um item só sai daqui com evidência registrada: sistema, versão, cenário e resultado. **CI verde, simulador e adaptadores falsos não fecham nenhum item da seção 1.**

## 1. Validação física (gate de qualquer versão seguinte)

| Pendente | Como comprovar |
|---|---|
| Travessia A ↔ notebook ↔ B com o mouse físico do desktop Windows | Mesma v0.5.0 nas duas máquinas, organização aplicada; atravessar A → notebook → B e B → notebook → A; registrar entrada proporcional, retorno pelo lado de entrada e eventual salto ao centro. Exige dois monitores no Windows. |
| Posição real do cursor na chegada | Comparar a posição esperada com a observada; a proposta de medição P5 (oráculo físico) ficou inconclusiva por falta de instrumento. |
| Teclado, roda e soltura | Shift ou tecla pressionada ao atravessar; Pausar; bloquear origem e destino; desconectar. Nenhuma tecla pode ficar pressionada. |
| Incidente "teclado não digita no notebook" | Adiado em 30/10/2026. Um teste guiado posterior funcionou com os dois teclados; a causa não foi identificada. Precisa de aplicativo, campo e sequência da falha original. |
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
