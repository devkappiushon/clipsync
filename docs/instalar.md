# Instalar

Guia para instalar a v0.5.0 publicada no Linux e no Windows. Para o primeiro uso depois de instalar, veja [uso](uso.md). Limites de plataforma e avisos de segurança ficam em [limitações](limitacoes.md).

## Baixar

Baixe a [v0.5.0](https://github.com/devkappiushon/clipsync/releases/tag/v0.5.0). Cada release traz seis pacotes e o `SHA256SUMS`:

- `clipsync-app-v0.5.0-linux-x86_64.AppImage` e `clipsync-app-v0.5.0-linux-amd64.deb`: app de desktop no Linux.
- `clipsync-app-v0.5.0-windows-x86_64-setup.exe` e `clipsync-app-v0.5.0-windows-x86_64.msi`: app de desktop no Windows.
- `clipsync-v0.5.0-linux-x86_64.tar.gz` e `clipsync-v0.5.0-windows-x86_64.zip`: só a linha de comando (`clipsync`), com o README.

Os pacotes do app e da CLI são separados. Para `clipsync status`, `diagnose` e os comandos de teste, baixe também o tar.gz no Linux ou o zip no Windows.

Confira antes de executar (troque `v0.5.0` pela tag que você baixou):

```sh
# Linux
sha256sum -c SHA256SUMS --ignore-missing
```

```powershell
# Windows (PowerShell): compare com a linha do pacote em SHA256SUMS
Get-FileHash .\clipsync-app-v0.5.0-windows-x86_64-setup.exe -Algorithm SHA256
```

O `--ignore-missing` confere somente os arquivos presentes na pasta; sucesso com três pacotes não valida os seis. O script do AppImage executa o arquivo para extrair o ícone, então confira o hash antes de rodá-lo.

## App de desktop

O app traz as telas e o agente no mesmo processo, com ícone na bandeja. Fechar a janela só esconde o app; para encerrar, use "Sair" no menu da bandeja.

### Linux

Com o AppImage, sem sudo, a partir da raiz do repositório:

```sh
scripts/instalar-app-linux.sh ~/Downloads/clipsync-app-v0.5.0-linux-x86_64.AppImage
```

O script fica no repositório, não no release: ele copia o AppImage para `~/.local/bin/clipsync-app.AppImage` e cria o atalho "ClipSync" no menu de aplicativos. Sem o clone, copie o AppImage para `~/.local/bin`, marque como executável e crie o atalho à mão, ou use o `.deb`. Rodar de novo com um AppImage mais novo atualiza.

O app precisa do WebKitGTK 4.1. Se o AppImage não abrir por falta de FUSE, instale o `libfuse2` (no Ubuntu 24.04 ou mais novo, `libfuse2t64`). No GNOME, o ícone da bandeja depende da extensão AppIndicator.

Com o `.deb` (o pacote se chama `clip-sync` e depende de `libwebkit2gtk-4.1-0` e `libgtk-3-0`):

```sh
sudo apt install ./clipsync-app-v0.5.0-linux-amd64.deb
```

### Windows

Rode o `setup.exe` (ou o `.msi`). O workflow e o manifesto não configuram assinatura, então o SmartScreen pode exibir um aviso; esse comportamento não foi observado em hardware nesta revisão. Se o aviso aparecer, clique em "Mais informações" e depois em "Executar assim mesmo". O app usa o WebView2, que já vem no Windows 10 e 11 atualizados; se faltar, o instalador tenta baixar, o que exige internet e uma política que permita a instalação.

## Rede e firewall

As máquinas conversam por UDP na porta `47800` (ou na que estiver em `listen` no `config.toml`). Basta um dos lados conseguir chamar o outro, mas com a porta fechada nos dois sentidos a ligação depende só de quem chama.

No Ubuntu com UFW ativo, a entrada vem bloqueada. Libere a porta para a sua rede:

```sh
sudo ufw allow 47800/udp
```

Sem essa regra, o Linux só se conecta quando ele mesmo chama o outro computador. Se o IP do outro mudar (DHCP), a partir da v0.5.2 o ClipSync procura o par pelo nome da máquina (`NOME` e `NOME.local`) e grava o endereço novo; em versões anteriores era preciso parear de novo. A busca pelo nome só funciona quando o nome do dispositivo no ClipSync é o nome da máquina na rede, que é o padrão.

## Linha de comando

Requisitos: no Linux, glibc 2.39 ou mais nova (o binário é compilado no Ubuntu 24.04). No Windows, nenhum pacote extra: o `.exe` leva a biblioteca de execução embutida.

O perfil fica em `$XDG_CONFIG_HOME/clipsync` no Linux quando `XDG_CONFIG_HOME` está definido, com fallback para `~/.config/clipsync`; `--profile` e `CLIPSYNC_PROFILE` têm precedência. No Windows, fica em `%LOCALAPPDATA%\clipsync`. Não aponte `--profile` nem `CLIPSYNC_PROFILE` para pasta compartilhada, raiz de disco, `C:\Users\Public`, `C:\ProgramData` ou pasta sincronizada, porque a chave privada mora nela. Use só uma pasta dentro do seu perfil de usuário. Os demais avisos de perfil e privacidade estão em [limitações](limitacoes.md#windows).

## Atualizar

Encerre o ClipSync pelo **Sair** da bandeja nas duas máquinas e instale a mesma versão em ambas. Preserve o perfil existente: `clipsync init` cria ou recria a identidade e não é o caminho para atualizar nem para trocar módulos. O app AppImage também pode ser atualizado rodando o script de instalação de novo.

O layout da v0.5.0 usa formato v2. A v0.4.0 não lê esse formato; antes de voltar à versão anterior, use a recuperação v1 em **Ajustes avançados** de um aplicativo que ofereça essa recuperação, com backup válido. Abrir ou preparar telas não migra o layout. O passo a passo está em [monitores independentes](monitores-independentes.md#6-backup-recuperação-e-ajustes-avançados).
