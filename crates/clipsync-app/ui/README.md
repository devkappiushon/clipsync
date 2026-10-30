# Interface do ClipSync (ui)

Frontend estático do app Tauri 2: HTML, CSS e JavaScript puros, sem npm, sem bundler e sem conteúdo remoto. A casca (`crates/clipsync-app`) serve esta pasta como `frontendDist`. A lógica das telas fica no crate `clipsync-ui`; aqui só há apresentação. Contrato: `docs/contrato.md` §12 e §14.

## Regras que valem em todo arquivo

- Dado vindo do agente entra na tela só por `textContent` ou pelo construtor `h()` de `js/dom.js`. atribuir HTML em texto é proibido.
- Sem atributo de estilo em linha e sem `<script>` inline: a CSP (`default-src 'none'; script-src 'self'; style-src 'self'`) bloqueia os dois. Estilo vai em `css/`, e o JS muda classe ou atributo `data-`.
- Sem travessão em texto de interface. Separe com vírgula, dois-pontos, `·` ou parênteses.
- Tema claro e escuro por tokens em `css/tokens.css` (escala de 8 pt, um raio, uma elevação). Nada de cor solta nas telas.
- Todo botão assíncrono passa por `comCarregamento` (o `botao()` já faz isso). Toda tela tem estado vazio e estado de erro com a `message` do agente.
- Recurso sem backend aparece desabilitado com "indisponível nesta versão", conforme `app_state.features`. Nunca fica escondido.

Critério de pronto da interface: §12.9 item 5 do contrato. Ele busca estilo em linha, atribuição de HTML e o caractere de travessão nesta pasta, e não pode achar nada. Por isso este README descreve essas regras sem escrevê-las.

## Estrutura

```
index.html            casca: barra lateral, <main>, módulo js/app.js
css/                  fonts, tokens, app (casca), components e telas/<tela>.css
fonts/                Inter, Geist e Geist Mono em woff2 variável (OFL.txt e FONTS.md)
js/api.js             único ponto de acesso a dados: invoke("clipsync", { req }) e eventos
js/estado.js          AppState da barra lateral e distribuição dos eventos
js/componentes.js     botão, selo, cartão, interruptor, campo, segmentado, tabela, diálogo
js/areas.js           cabeçalho, região assíncrona, avisos e estados vazio e de erro
js/telas/<tela>.js    uma tela por arquivo
```

O que só serve ao desenvolvimento não mora aqui: fica em `crates/clipsync-app/ui-dev/`, nos mesmos caminhos relativos. São o simulador do agente (`mock.js` e `mock/<dominio>.js`, um domínio por arquivo) e o catálogo de componentes (`componentes.html` e `js/galeria.js`). A pasta `ui/` é a que vai no pacote do app, então nada disso entra nele.

Telas: onboarding (3 passos), dispositivos, pareamento, layout de telas, clipboard, diagnóstico e preferências.

## Rodar no navegador (com o simulador)

Da raiz do repositório:

```sh
python3 -I scripts/servir-ui-dev.py          # porta 5174 por padrão
python3 -I scripts/servir-ui-dev.py 5173     # outra porta
```

O script serve `ui/` por cima de `ui-dev/` nas mesmas URLs: um caminho que não existe em `ui/` é procurado em `ui-dev/`. Assim `/mock.js`, `/mock/...` e `/componentes.html` existem no navegador e não existem no pacote. Responde com `Cache-Control: no-store` e escuta só em `127.0.0.1`. Um `python3 -m http.server` apontado só para `ui/` não serve: a interface falha com erro na tela, porque falta o simulador.

Servidor de dev é exceção à regra de não subir servidor, só enquanto você confere: derrube no fim. No Maestri, o servidor é do Maestro central: não suba outro.

Como `js/api.js` decide de onde vêm os dados:

- Com `window.__TAURI__` (dentro do app), chama o comando `clipsync` e nunca toca no simulador.
- Sem `window.__TAURI__`, só carrega o simulador em `http://127.0.0.1` ou `http://localhost`. Em qualquer outro endereço (inclusive a origem do app), falha fechado: mostra erro e não importa nada.
- Nenhum outro arquivo de `ui/` importa o simulador.

### Cenários

Escolha pela URL: `/?cenario=<nome>#/<tela>`.

| Parâmetro | Efeito |
|---|---|
| `cenario=padrao` | três pares, um bloqueio, layout com uma ligação |
| `cenario=vazio` | sem pares, caixa e bloqueios vazios, layout só com esta máquina |
| `cenario=sem-perfil` | perfil não criado: tudo cai no onboarding |
| `cenario=erro` | agente com falha: telas mostram o erro e "Tentar de novo"; gravar preferências falha com `io` |
| `cenario=estados` | par offline, par bloqueado por chave diferente, oferta de layout pendente, exportação do pacote falhando |
| `cenario=pareamento` | pareamento já no passo do código, sem esperar (sair da tela cancela a tentativa) |
| `cenario=windows` | PC Windows da S7: dois monitores (um 4K a 150%), teclado e mouse ligados, atalhos ativos, recusas de destino listadas como bloqueio |
| `cenario=windows-recusa` | o mesmo cenário Windows, com o Win+Alt+P de retomar recusado pelo sistema: abre pausado e o bloqueio aparece no Diagnóstico |
| `cenario=sem-ligacao` | pares conectados, nenhuma passagem local; combine com `plataforma=windows` para dois monitores na origem |
| `cenario=receptor` | sem ligação local, com permissão de entrada: orienta configurar no computador do mouse físico |
| `cenario=ligacao-remota` | há ligação entre dois pares, mas nenhuma nesta máquina |
| `cenario=protocolo-antigo` | par conectado no protocolo 2; aviso não inventa a versão do app |
| `cenario=sem-monitores` | pares sem monitores anunciados; não permite criar monitor fictício |
| `cenario=input-desligado` | módulo de teclado e mouse desligado, Layout indisponível |
| `cenario=teste-zero` | teste devolve zero eventos; resultado inconclusivo |
| `cenario=modulos-externo` | agente externo: aplicar módulos falha sem reiniciar; trocar o modo de clipboard só salva a configuração e pede reinício externo |
| `cenario=modulos-parado` | agente parado: aplicar módulos grava e continua parado, sem iniciar |
| `cenario=modulos-desconhecido` | estado do agente desconhecido: aplicar falha, e pausa e trava aparecem como não observadas |
| `cenario=modulos-ocupado` | outra alteração do perfil em andamento: `profile_busy`, nada é gravado |
| `cenario=modulos-falha-pausa` | a pausa não persiste: a aplicação para antes de salvar e o resumo diz que a pausa pode ter ficado só em memória |
| `cenario=modulos-falha-parada` | a parada do agente não é confirmada: módulos não são salvos e a posse continua ocupada |
| `cenario=modulos-falha-escrita` | gravação falha: a edição aborta sem reiniciar |
| `cenario=modulos-conflito-tardio` | a configuração muda durante a parada: `stale_config` e nada é salvo |
| `cenario=modulos-externo-antes-save` | outro agente assume antes de salvar: configuração intacta e pausa não observada |
| `cenario=modulos-externo-apos-save` | outro agente assume depois de salvar: módulos salvos, nenhuma retomada enviada e trava não confirmada |
| `cenario=modulos-falha-inicio` | módulos salvos, inicialização do agente falha: a malha não volta sozinha |
| `cenario=modulos-falha-trava` | mesmo ramo de `modulos-falha-inicio`: a inicialização falha e a trava não é confirmada quando o input continua ligado |
| `cenario=b09-falha-pausa` / `b09-falha-retomada` | pausa ou retomada aplicada em memória sem persistir: aviso persistente até a confirmação seguinte |
| `cenario=b09-pausa-volatil` / `b09-retomada-volatil` | abre com o aviso de persistência já visível |
| `cenario=b09-falha-inicio` / `b09-falha-trava` | mesmo ramo: a troca de modo salva e a inicialização falha; a trava aparece como não confirmada quando o input continua ligado |
| `cenario=b09-externo` / `b09-parado` | troca de modo com agente externo (só configuração, reinício externo) ou parado (salva sem iniciar) |
| `cenario=b09-desconhecido` | agente próprio com estado não confirmado: aviso de pausa não confirmada e pausa e trava sem valor |
| `cenario=b01-grupos` | bloqueios fictícios para a tabela: legado sem agregação, grupos repetidos (7, 3 e 2), causa geral sem par e contador saturado no teto |
| `plataforma=windows` | o mesmo, combinável com outro cenário (ex.: `cenario=sem-perfil&plataforma=windows` mostra o onboarding do Windows) |
| `tema=claro` ou `tema=escuro` | tema inicial, para capturar as telas nos dois temas |

### Testes do simulador

Sem navegador e sem npm, com o `node --test` embutido. Provam o simulador, não o agente (esse é testado em Rust, no `clipsync-ui`).

```sh
node --test crates/clipsync-app/ui-dev/testes/*.test.mjs
```

## Configuração da passagem do mouse

O fluxo principal do Layout usa a posição do computador (esquerda, direita, acima ou abaixo). Arrastar prepara a mesma escolha; `Ligar nessa posição` grava a posição e a ligação, com a revisão retornada pela primeira gravação. Os monitores e trechos externos são calculados com os dados reais do layout. Uma ligação existente é atualizada pelo id; várias ligações personalizadas pedem os ajustes avançados para não apagar trechos do usuário.

O fluxo por posição grava `layout.toml` com as ligações por PeerId. O `[[input.barrier]]` manual do `config.toml` é o caminho legado do loopback, com o par pelo nome; o teste aceita qualquer par conhecido e conectado. Ter barreiras manuais e ligações no layout ao mesmo tempo dá erro de configuração, com qualquer ligação; não basta os dois arquivos existirem.

Cantos mortos, atalhos, cópia para pares e editor manual de bordas ficam em `Ajustes avançados`. O estado de travessia vem de `app_state.traversal` e `traversal_changed`; configuração encontrada não é prova de captura funcionando. O aviso de protocolo usa número, não a versão do app.

`Conferir a passagem do mouse` explica e abre o teste apenas no clique. O usuário cruza a borda e move o mouse. O resultado mostra eventos contabilizados e perdas; zero eventos é inconclusivo. O teste não comprova a volta pelo trecho de entrada nem mede latência de rede confiável entre relógios de processos distintos.

`travessia.test.mjs` exercita os cálculos reais de `ui/js/travessia.js`: monitor externo alinhado mesmo com alturas diferentes, deslocamento sob o segundo monitor, coordenadas negativas, trechos internos/ocupados e cantos mortos (incluindo arredondamento f32), ausência de monitores, ligações somente remotas, atualização sem duplicação e interpretação de resultados. Os testes de oferta verificam o contrato do simulador; o agente real tem testes Rust separados.

## Módulos pelo app (C01)

Preferências tem o cartão "Módulos", com os interruptores de teclado e mouse e de área de transferência, e o botão "Aplicar módulos". O passo "O que sincronizar" do onboarding reusa a mesma edição quando o perfil já existe. Aplicar não refaz o perfil: a API local `modules_set` grava só os dois módulos em `config.toml` e preserva identidade, chave, certificado, pares, layout e `ui.toml` (contrato §15.7 item 4). Refazer o perfil continua sendo outra operação, com identidade nova e novo pareamento.

- O pedido leva `modules` com os dois booleanos e `expected` com o valor que a tela leu. Se a configuração mudou nesse meio tempo, a resposta é `stale_config` e nada é gravado; campo fora de `modules` e `expected` é recusado.
- Sucesso devolve `changed`, `modules`, `effects` e `observed`. `changed: false` é no-op, sem gravação, pausa, parada ou início. Erro traz `details.effects` e `details.observed`; a tela mostra o resumo, reconsulta e só então libera o botão.
- Com o agente próprio no ar, a aplicação confirma a pausa e a parada, reserva o listener, grava e reinicia pausado, restaurando a trava desta tela no bootstrap. Se não conseguir reservar o listener, não grava. Retomar é ação explícita do usuário. Com agente externo, o app não reinicia nada; com agente parado, grava e continua parado.
- O rascunho local não é prova de gravação: só a resposta e a reconsulta atualizam o valor de referência. Resposta que chega sem confirmar o estado deixa o cartão em "não confirmado" até nova consulta.

## Pausa, aviso e estado desconhecido (B09)

O topo do conteúdo mostra aviso persistente quando a pausa ou a retomada não foi confirmada, acima da tela aberta. A fonte é `app_state.pause_warning`, opcional e aditivo; o texto diz qual ação foi pedida de verdade e distingue "aplicada em memória, não salva" de "comunicação incerta" (contrato §15.8).

- Aviso antigo não apaga falha nova, polling não limpa aviso e uma resposta anterior à falha atual não o remove. Só a confirmação de persistência da mesma ação limpa.
- `paused: null` significa não observado. A tela não converte isso em pausado nem em retomado, e os atalhos aparecem indisponíveis com o motivo.
- Clipboard: o texto do modo avisa antes da troca que, se ela reiniciar o agente deste app, a malha fica pausada. Com agente externo, só a configuração é salva e `restart_needed` volta verdadeiro; a tela não afirma que o modo vale no processo externo.

## Bloqueios do Diagnóstico (B01)

O agrupamento é do agente, não da tela. `recent_blocks` traz no máximo 20 grupos pela chave par, módulo e motivo; chegada nova ou repetida move o grupo para o início e causa nova expulsa o último (contrato §15.10). O nome do par é só apresentação.

- A tabela mostra primeira ocorrência, última ocorrência e ocorrências observadas. `occurrences` ausente vale 1 e `last_unix_ms` ausente cai para `at_unix_ms`.
- Contagem acima do inteiro seguro do JavaScript aparece como "pelo menos N", porque o valor acima disso não é exato na tela.
- A cadência de 30 s e o teto de notificação são da casca, não desta pasta. O Diagnóstico continua mostrando a história observada mesmo com a notificação de bloqueio desligada.

## Limites do simulador

O simulador prova a tela e o contrato dos comandos, não o agente. Nesta pasta, tudo que aparece em `effects`, `observed`, `recent_blocks` e `pause_warning` é escrito à mão em `ui-dev/mock/`.

- `cenario=b01-grupos` cobre legado sem agregação, grupos repetidos, causa geral sem par e contador no teto. Ele não cobre a agregação dinâmica, a cadência de 30 s nem o teto de 20 grupos, que são do agente e da casca.
- Reinício pausado, restauração da trava e estado desconhecido trocam `estado.agente` no mock; nenhum processo reinicia. Lock de configuração, gravação de `config.toml` e `state.toml` e reserva de listener são Rust.
- Seguem sem prova: app Tauri real, zoom real, leitor assistivo e AT-SPI, clipboard do sistema, hooks, sessão WTS e desktop do Windows, travessia física e duas máquinas.
- O ciclo de vida da captura (B07) segue em revisão, sem aceite. O round4 do Pareamento foi aprovado pelos revisores nativos, o que não é prova em portal, app real ou duas máquinas.

## Acrescentar uma tela

1. Crie `js/telas/<nome>.js` com `export default function montar({ rota, ir, assinar })`, devolvendo `{ no, desmontar? }` (ou uma promessa disso).
2. Registre a rota em `ROTAS` de `js/app.js` e o link em `index.html`, se for da barra lateral.
3. Crie `css/telas/<nome>.css`, ligue em `index.html`, e use só tokens e classes prefixadas pelo nome da tela.
4. Peça dados só por `api.<comando>()`. Para um comando novo, mude antes o contrato e o `ui-dev/mock/<dominio>.js`.
5. Escreva os estados de carregando, vazio e erro (`area()` de `js/areas.js` já cobre os três).

## Catálogo de componentes

`/componentes.html`, no servidor de dev, mostra cada componente em todos os estados, para conferir os dois temas e a acessibilidade. A página e o `js/galeria.js` ficam em `ui-dev/` e importam os módulos de `ui/js/` pelos mesmos caminhos relativos.

## Fontes

Inter no corpo, Geist nos títulos, Geist Mono em código, ids e valores de tabela (`tabular-nums`). Origem, versão e SHA-256 em `fonts/FONTS.md`. Self-hosted, `font-display: swap`, sem CDN.

## Crédito

`<meta name="author">` no `index.html` e "feito por montesouza.dev" na seção Sobre de Preferências.
