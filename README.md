# ClipSync (nome de código)

Teclado, mouse e área de transferência entre os meus computadores, em um aplicativo só, sem servidor central. Cada módulo é opcional: dá para usar só teclado e mouse, só clipboard, ou os dois. A escolha é feita no `clipsync init` e fica em `config.toml` (`[modules]`).

Estado em 2026-10-28: a v0.1.0 publicou a linha de comando; a v0.2.0 trouxe o app de desktop, com as telas e o agente no mesmo processo; a v0.3.0 traz teclado e mouse no Windows e o layout de telas entre Windows e Linux, por borda de monitor. O teclado e o mouse entre máquinas reais ainda não foram validados: o roteiro de teste está em `docs/contrato.md` §13.12. Detalhes: `docs/matriz-compatibilidade.md`.

## Baixar

Cada release traz o app de desktop e a linha de comando, mais o `SHA256SUMS`:

- `clipsync-app-v0.3.0-linux-x86_64.AppImage` e `clipsync-app-v0.3.0-linux-amd64.deb`: app de desktop no Linux.
- `clipsync-app-v0.3.0-windows-x86_64-setup.exe` e `clipsync-app-v0.3.0-windows-x86_64.msi`: app de desktop no Windows.
- `clipsync-v0.3.0-linux-x86_64.tar.gz` e `clipsync-v0.3.0-windows-x86_64.zip`: só a linha de comando (`clipsync`), com este README.

Confira antes de executar (troque `v0.3.0` pela tag que você baixou):

```sh
# Linux e macOS
sha256sum -c SHA256SUMS --ignore-missing
```

```powershell
# Windows (PowerShell): compare com a linha do pacote em SHA256SUMS
Get-FileHash .\clipsync-app-v0.3.0-windows-x86_64-setup.exe -Algorithm SHA256
```

### App de desktop

O app traz as telas e o agente no mesmo processo, com ícone na bandeja. Fechar a janela só esconde o app; para encerrar, use "Sair" no menu da bandeja.

No Linux, sem sudo, com o AppImage:

```sh
scripts/instalar-app-linux.sh ~/Downloads/clipsync-app-v0.3.0-linux-x86_64.AppImage
```

O script copia o AppImage para `~/.local/bin/clipsync-app.AppImage` e cria o atalho "ClipSync" no menu de aplicativos. Rodar de novo com um AppImage mais novo atualiza. O app precisa do WebKitGTK 4.1; se o AppImage não abrir por falta de FUSE, instale o `libfuse2`. No GNOME, o ícone da bandeja depende da extensão AppIndicator. Com o `.deb`: `sudo apt install ./clipsync-app-v0.3.0-linux-amd64.deb`.

No Windows, rode o `setup.exe` (ou o `.msi`). O instalador não é assinado, então o SmartScreen avisa: clique em "Mais informações" e depois em "Executar assim mesmo". O app usa o WebView2, que já vem no Windows 10 e 11 atualizados; se faltar, o instalador baixa.

### Linha de comando

Requisitos: no Linux, glibc 2.39 ou mais nova (o binário é compilado no Ubuntu 24.04). No Windows, nenhum pacote extra: o `.exe` leva a biblioteca de execução embutida. O perfil fica em `~/.config/clipsync` no Linux e em `%LOCALAPPDATA%\clipsync` no Windows; não aponte `--profile` para uma pasta compartilhada, porque a chave privada mora nela.

Avisos para Windows (revisão de segurança S5, em `docs/seguranca.md`):

1. O perfil fica em `%LOCALAPPDATA%\clipsync` e guarda a chave privada. Não aponte `--profile` nem `CLIPSYNC_PROFILE` para raiz de disco, `C:\Users\Public`, `C:\ProgramData`, compartilhamento de rede ou pasta sincronizada (OneDrive, Dropbox). Só pasta dentro do seu perfil de usuário.
2. A proteção da chave é a ACL do seu usuário: qualquer programa que rode como você, e qualquer administrador, lê a chave.
3. Em máquina com mais de um usuário (compartilhada, RDP), o Windows ainda é experimental. Pare o agente com Ctrl+C; se fechar a janela, rode `clipsync run` de novo antes de usar `clipsync send`.
4. O clipboard do Windows lê tudo o que é copiado, inclusive o que gerenciadores de senha marcam como sensível. Use `clipsync pause` ao copiar senhas.
5. O texto recebido de outro dispositivo é gravado com os marcadores que pedem ao Windows para não guardar no histórico (Win+V) nem na nuvem. Isso depende de o Windows respeitar o marcador.
6. O clipboard do Windows é experimental: polling de 250 ms, só texto, ainda não testado em máquina real.
7. Teclado e mouse no Windows usam ganchos de baixo nível do sistema. O ClipSync só os instala com o módulo ligado e uma ligação desta máquina no layout, tira os ganchos ao pausar, não registra tecla nenhuma e não injeta nada na tela do UAC nem com a tela bloqueada. Programas abertos como administrador não recebem o teclado e o mouse de outra máquina.

O que funciona em cada plataforma nesta versão:

| Recurso | Linux (X11 e XWayland) | Windows |
|---|---|---|
| Malha QUIC e pareamento | sim | sim |
| Clipboard de texto | sim | sim (só texto) |
| Teclado e mouse (layout de telas por borda de monitor) | experimental (GNOME Wayland) | experimental (atalhos Win+Alt+1 a 9, L e P) |

## Documentos

- `docs/prompt-mestre.md`: especificação que manda (v4).
- `docs/contrato.md`: contrato técnico do MVP (árvore, interfaces, critérios de pronto).
- `docs/workboard.md`: fatias, estados e decisões. `docs/verdicts.md`: vereditos do warden com evidência.
- `docs/decisoes/`: ADRs (stack, transporte QUIC, pareamento).
- `docs/telas.md`: as sete telas da UI futura (Tauri), base do protótipo no Figma.
- `docs/prompt-mestre-anexo-original.md`: requisitos detalhados do módulo de clipboard.

## Requisitos de desenvolvimento

- Rust 1.99.1 via `rustup` (o `rust-toolchain.toml` fixa a versão). Sem lib de sistema: tudo compila com `build-essential`.
- Linux Wayland (GNOME 45+ ou KDE 6.1+) para os portais de entrada; XWayland ativo (`DISPLAY` definido) para o clipboard.
- Sem sudo em nenhum passo.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
sh /tmp/rustup-init.sh -y --profile minimal --component clippy,rustfmt --no-modify-path
source ~/.cargo/env
cargo build --release -p clipsync-cli      # binário em target/release/clipsync
cargo test --workspace                     # testes sem sessão gráfica
```

## Roteiro de teste de 10 minutos (numa máquina só)

Dois perfis no mesmo notebook, `a` e `b`, em portas diferentes. Nenhum passo mexe no seu clipboard sem pedir confirmação.

```sh
export PATH="$PWD/target/release:$PATH"

# 1. identidade, nome e módulos (a pergunta de clipboard tem padrão NÃO)
clipsync init --profile /tmp/cs-a --name notebook-a --modules input,clipboard --port 47800
clipsync init --profile /tmp/cs-b --name notebook-b --modules input,clipboard --port 47801

# 2. pareamento: os dois terminais mostram o mesmo código de 6 dígitos; confirme "s" nos dois
clipsync pair host --profile /tmp/cs-a --allow clipboard-in,clipboard-out,input-in
clipsync pair join --profile /tmp/cs-b 127.0.0.1:PORTA_IMPRESSA --allow clipboard-in,clipboard-out

# 3. agentes (um terminal cada) e estado
clipsync run --profile /tmp/cs-a
clipsync run --profile /tmp/cs-b
clipsync status --profile /tmp/cs-a

# 4. clipboard: observar sem roubar foco, escrever com confirmação, enviar entre os perfis
clipsync probe clipboard --watch 15s --profile /tmp/cs-a
clipsync probe clipboard --write --profile /tmp/cs-a
printf 'olá ção' | clipsync send --stdin --profile /tmp/cs-a
clipsync inbox --profile /tmp/cs-b

# 5. entrada: barreira só na metade de cima da borda direita, emulação, latência entre perfis
clipsync probe input --capture --profile /tmp/cs-a
clipsync probe input --emulate --profile /tmp/cs-a
clipsync probe input --loopback --to notebook-b --seconds 10 --profile /tmp/cs-a

# 6. pausa e diagnóstico
clipsync pause --profile /tmp/cs-b && clipsync resume --profile /tmp/cs-b
clipsync diagnose --profile /tmp/cs-a
```

Com `clipboard = false` no `config.toml` do perfil `b`, o `send` de `a` é recusado com motivo legível.

## Convivência com Input Leap

Enquanto o módulo de entrada não estiver validado entre máquinas reais, o Input Leap continua com teclado e mouse. Os dois não devem capturar ao mesmo tempo; `clipsync diagnose` detecta Input Leap e Barrier ativos e orienta a pausa. Nada no Input Leap é alterado por este projeto.

## Licença

Todos os direitos reservados, até o autor escolher uma licença aberta. Sem licença, o código pode ser lido, mas não pode ser copiado, alterado nem redistribuído.

feito por montesouza.dev
