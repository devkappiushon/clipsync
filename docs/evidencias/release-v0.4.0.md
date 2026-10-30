# Publicação da v0.4.0 · 30/10/2026

Publicada em 2026-10-30T05:43:46Z, após autorização de João. [Release e downloads](https://github.com/devkappiushon/clipsync/releases/tag/v0.4.0). A tag aponta para `6cc1384a7d036877e7077d3c6b63838c64750d4b`; um registro documental posterior não altera os binários publicados.

## Execução confirmada

- [CI 41006607785](https://github.com/devkappiushon/clipsync/actions/runs/41006607785): Linux e Windows aprovados; fmt, Clippy, scripts de privacidade/unsafe e testes do workspace incluindo Tauri.
- Linux: 624 testes principais aprovados, cinco execuções auxiliares separadas, três testes gráficos ignorados. Windows: 614 principais, cinco auxiliares e um gráfico ignorado. Ambos incluem 27 testes do app.
- Windows executou os 11 testes de emulação, 14 de captura e 10 do módulo Windows. O conjunto de 10 inclui casos portáveis e específicos da plataforma; não são 10 novos testes desta rodada. Fakes continuam presentes e não provam SendInput/UIPI ou soltura física.
- Node: 164/164 localmente depois do ajuste de versão; essa suíte não roda no CI atual.
- [Empacotamento manual 41006610958](https://github.com/devkappiushon/clipsync/actions/runs/41006610958): aprovado no mesmo commit. Os artefatos foram promovidos sem recompilar. A execução automática da tag `41010031820` foi cancelada por duplicar esse build, sem falha de código.

## Arquivos conferidos

Seis pacotes não vazios; CLI Linux executada somente com `--version`, que retornou `clipsync 0.4.0`. Debian: `clip-sync`, versão `0.4.0`, arquitetura `amd64`. AppImage e CAB do MSI passaram pela leitura de integridade; ZIP Windows passou pelo CRC, contém CLI PE AMD64 e README correspondente ao commit após normalizar as quebras CRLF do checkout Windows.

Os sete tamanhos e digests SHA256 remotos, incluindo `SHA256SUMS`, conferiram com os arquivos enviados antes da publicação. Após publicar, os sete arquivos foram baixados novamente: `sha256sum -c SHA256SUMS` confirmou 6/6 OK, e os sete tamanhos/digests corresponderam à API e aos arquivos enviados. Integridade do pacote não comprova instalação ou uso físico.

| Arquivo | Bytes | SHA256 |
|---|---:|---|
| `clipsync-app-v0.4.0-linux-amd64.deb` | 8135112 | `4c33f46e311199621c96700ff52d9ae4108f77c2643c7062d885fe9f151dfbb6` |
| `clipsync-app-v0.4.0-linux-x86_64.AppImage` | 86333944 | `a1b632153c90d2e2caaa2a09a9902fe3727da63c33243898b0f1c651a08f847e` |
| `clipsync-app-v0.4.0-windows-x86_64-setup.exe` | 4985156 | `75f8e247aebce154a645a14b32cad41b29195e46f00d4db8f7141af0fcdba4d0` |
| `clipsync-app-v0.4.0-windows-x86_64.msi` | 6873088 | `58ce4ebdfe2d9366ec4c332a233156cb84452a424b065416cd9bb5ba9ba0c600` |
| `clipsync-v0.4.0-linux-x86_64.tar.gz` | 4487225 | `b3c509ede782d69602682eda22929668b3090a89daf6acdb52c36aba07c634df` |
| `clipsync-v0.4.0-windows-x86_64.zip` | 3366565 | `769c56ec6328996ac21d19ccded79e079f97675fd3faef9c50d7eb89cdce8e2b` |

## O que permanece aberto

[Pendências explícitas](../pendencias-v0.4.0.md): instalação e travessia Windows/Linux, entrada proporcional e salto ao centro, teclas/soltura, permissões e desktop seguro, app/bandeja/notificações, zoom/leitores e lacunas de portal. Nenhum instalador foi executado no computador do usuário nesta publicação. O protocolo continua 3, mínimo 1; perfil e pareamentos devem ser preservados.

Evidências locais de origem: `.superpowers/release-v0.4.0/` (logs, metadados de CI, digests e recibos). Os relatórios extensos de coordenação ficam fora do release.
