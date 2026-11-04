# Publicação da v0.5.0 · 31/10/2026

Publicada em 2026-10-31T04:02:24Z, após autorização de João. [Release e downloads](https://github.com/devkappiushon/clipsync/releases/tag/v0.5.0). A tag aponta para `590c137f00b125f76574d43b7fd89b89a23fe0b1`, o mesmo commit da `main` em 03/11/2026.

## Execução confirmada

- [CI 41161430657](https://github.com/devkappiushon/clipsync/actions/runs/41161430657): Linux, Windows e Node aprovados no commit publicado; fmt, Clippy, scripts de privacidade e unsafe, testes do workspace com o app Tauri e os 264 testes Node da interface.
- A execução Windows do commit anterior (`f3cdd01`, run 41159184403) falhou num teste cujo coletor descartava a notificação `DeviceRemoved`; a correção em `590c137` ficou restrita ao teste e o CI voltou a passar.
- [Empacotamento manual 41161435861](https://github.com/devkappiushon/clipsync/actions/runs/41161435861): aprovado no mesmo commit; os seis pacotes publicados são os artefatos desse workflow. A execução automática da tag (41159222870) foi cancelada por duplicar o build.
- Em 03/11/2026 o andar DOC baixou de novo `.deb`, `.tar.gz` e `.zip` e conferiu os hashes com o `SHA256SUMS`: 3 de 3 OK. Os sete digests remotos abaixo vêm da API do GitHub.

## Arquivos publicados

| Arquivo | Bytes | SHA256 |
|---|---:|---|
| `clipsync-app-v0.5.0-linux-amd64.deb` | 8533498 | `87d7b93265e2d1435ccc7db98fd147f5f21ea240796bd29487d455e9f0939496` |
| `clipsync-app-v0.5.0-linux-x86_64.AppImage` | 86702584 | `b40588ff0702245f94ebc86a73f344868cc39ca301da15ac06c546eafb9e5336` |
| `clipsync-app-v0.5.0-windows-x86_64-setup.exe` | 5301049 | `4ff54ab9e33a6bfe21eede494e8af58780a6e2a21444a5705ec6f1f8fa574f01` |
| `clipsync-app-v0.5.0-windows-x86_64.msi` | 7299072 | `17765340558f5c8792c153f5881e3d12fcf2e6f3c46cc25f8ec8ddefc8459b24` |
| `clipsync-v0.5.0-linux-x86_64.tar.gz` | 4804419 | `523fda2d7c1003c130cd98f1c1157704d0ba5fa9bf8d8b481b4829a2d6085c39` |
| `clipsync-v0.5.0-windows-x86_64.zip` | 3723490 | `30bfdfe337d420d854e7cb9b449cebf04fdd81b0988648cf73657e1651709e90` |
| `SHA256SUMS` | 629 | `9ff8335d6afa02e2ccf171bca3760db46fa6e1aa7b0ecf4765da288d3ab19ddc` |

Integridade do pacote não comprova instalação nem uso físico. O README embutido no `.tar.gz` e no `.zip` é o do commit `590c137`; mudanças posteriores no repositório não alteram os pacotes já publicados.

## O que permanece aberto

[Pendências da v0.5.0](../pendencias-v0.5.0.md): travessia física A ↔ notebook ↔ B, soltura de teclas, app Tauri interativo, permissões reais, leitores de tela, instalação no Windows e o achado menor T4-SEG-01. O pacote `clip-sync 0.5.0` está instalado no notebook Linux (conferido com `dpkg-query` em 03/11/2026); a instalação no Windows não foi verificada.
