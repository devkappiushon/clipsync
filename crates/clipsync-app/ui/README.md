# Interface do ClipSync (ui)

Frontend estático do app Tauri 2: HTML, CSS e JavaScript puros, sem npm, sem bundler e sem conteúdo remoto. A casca (`crates/clipsync-app`) serve esta pasta como `frontendDist`. A lógica das telas fica no crate `clipsync-ui`; aqui só há apresentação. Contrato: `docs/contrato.md` §12.

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
componentes.html      catálogo de componentes em todos os estados (js/galeria.js)
css/                  fonts, tokens, app (casca), components e telas/<tela>.css
fonts/                Inter, Geist e Geist Mono em woff2 variável (OFL.txt e FONTS.md)
js/api.js             único ponto de acesso a dados: invoke("clipsync", { req }) e eventos
js/estado.js          AppState da barra lateral e distribuição dos eventos
js/componentes.js     botão, selo, cartão, interruptor, campo, segmentado, tabela, diálogo
js/areas.js           cabeçalho, região assíncrona, avisos e estados vazio e de erro
js/telas/<tela>.js    uma tela por arquivo
```

O simulador do agente não mora aqui: fica em `crates/clipsync-app/ui-dev/` (`mock.js` e `mock/<dominio>.js`, um domínio por arquivo). A pasta `ui/` é a que vai no pacote do app, então nada de simulação entra nele.

Telas: onboarding (3 passos), dispositivos, pareamento, layout de telas, clipboard, diagnóstico e preferências.

## Rodar no navegador (com o simulador)

Precisa de um servidor de dev que sirva `ui/` por cima de `ui-dev/` nas mesmas URLs: um caminho que não existe em `ui/` é procurado em `ui-dev/`. Assim `/mock.js` e `/mock/...` existem no navegador e não existem no pacote. O Maestro mantém um desses, com `Cache-Control: no-store`, em `http://127.0.0.1:5173` e `http://127.0.0.1:5174`. Um `python3 -m http.server` apontado só para `ui/` não serve: a interface falha com erro na tela, porque falta o simulador.

Servidor de dev é exceção à regra de não subir servidor, só enquanto você confere: derrube no fim.

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
| `plataforma=windows` | recursos de teclado e mouse desligados |
| `tema=claro` ou `tema=escuro` | tema inicial, para capturar as telas nos dois temas |

## Acrescentar uma tela

1. Crie `js/telas/<nome>.js` com `export default function montar({ rota, ir, assinar })`, devolvendo `{ no, desmontar? }` (ou uma promessa disso).
2. Registre a rota em `ROTAS` de `js/app.js` e o link em `index.html`, se for da barra lateral.
3. Crie `css/telas/<nome>.css`, ligue em `index.html`, e use só tokens e classes prefixadas pelo nome da tela.
4. Peça dados só por `api.<comando>()`. Para um comando novo, mude antes o contrato e o `ui-dev/mock/<dominio>.js`.
5. Escreva os estados de carregando, vazio e erro (`area()` de `js/areas.js` já cobre os três).

## Fontes

Inter no corpo, Geist nos títulos, Geist Mono em código, ids e valores de tabela (`tabular-nums`). Origem, versão e SHA-256 em `fonts/FONTS.md`. Self-hosted, `font-display: swap`, sem CDN.

## Crédito

`<meta name="author">` no `index.html` e "feito por montesouza.dev" na seção Sobre de Preferências.
