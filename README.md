# ClipSync (nome de código)

Teclado, mouse e área de transferência entre os meus computadores, em um aplicativo só, sem servidor central. Cada módulo é opcional: dá para usar só teclado e mouse, só clipboard, ou os dois. A escolha é feita no `clipsync init` e fica em `config.toml` (`[modules]`).

Estado em 2026-10-27: nível 1 (provas técnicas) concluído neste notebook; MVP de linha de comando em construção. Nada validado em outra máquina ainda. Detalhes: `docs/matriz-compatibilidade.md`.

## Baixar

Cada release traz um pacote por plataforma, com o binário `clipsync` e este README:

- `clipsync-v0.1.0-linux-x86_64.tar.gz`
- `clipsync-v0.1.0-windows-x86_64.zip` (`clipsync.exe`)

O release também publica `SHA256SUMS`. Confira antes de executar (troque `v0.1.0` pela tag que você baixou):

```sh
# Linux e macOS
sha256sum -c SHA256SUMS --ignore-missing
```

```powershell
# Windows (PowerShell): compare com a linha do pacote em SHA256SUMS
Get-FileHash .\clipsync-v0.1.0-windows-x86_64.zip -Algorithm SHA256
```

Requisitos: no Linux, glibc 2.39 ou mais nova (o binário é compilado no Ubuntu 24.04). No Windows, nenhum pacote extra: o `.exe` leva a biblioteca de execução embutida. O perfil fica em `~/.config/clipsync` no Linux e em `%LOCALAPPDATA%\clipsync` no Windows; não aponte `--profile` para uma pasta compartilhada, porque a chave privada mora nela.

O que funciona em cada plataforma nesta versão:

| Recurso | Linux (X11 e XWayland) | Windows |
|---|---|---|
| Malha QUIC e pareamento | sim | sim |
| Clipboard de texto | sim | sim (só texto) |
| Teclado e mouse | experimental | ainda não disponível |

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

A definir pelo autor antes do primeiro release (sugestão: MIT).

feito por montesouza.dev
