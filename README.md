# ClipSync (nome de código)

Teclado, mouse e área de transferência entre os meus computadores, em um aplicativo só, sem servidor central. Cada módulo é opcional: dá para usar só teclado e mouse, só clipboard, ou os dois. A escolha pode ser alterada no app em **Preferências > Módulos**, preservando identidade e pareamentos. Também pode ser feita no `clipsync init` durante a criação do perfil; esse comando não deve ser usado para apenas trocar módulos de um perfil existente.

Estado em 2026-10-30: a v0.4.0 reúne as correções S9 de controle de entrada, pausa, pareamento, configuração de módulos e acessibilidade. A v0.3.1 introduziu o Layout simples por posição; essa configuração continua sendo o caminho normal. **O mouse entre suas duas máquinas ainda precisa de teste físico.** Veja [o que falta validar](docs/pendencias-v0.4.0.md) e as [notas da versão](docs/releases/v0.4.0.md). O roteiro detalhado está em `docs/contrato.md` §13.12.

A fonte atual prepara a **v0.5.0 candidata**, com disponibilidade mais clara e um mapa independente por monitor, para posicionar o notebook entre duas telas do desktop. Veja o [guia dos monitores](docs/monitores-independentes.md). Essa versão ainda depende dos gates de integração e dos testes reais antes de ser publicada. As instruções de download e configuração abaixo continuam referentes à v0.4.0 publicada.

## Baixar

Baixe a [v0.4.0 publicada](https://github.com/devkappiushon/clipsync/releases/tag/v0.4.0). Cada release traz o app de desktop e a linha de comando, mais o `SHA256SUMS`:

- `clipsync-app-v0.4.0-linux-x86_64.AppImage` e `clipsync-app-v0.4.0-linux-amd64.deb`: app de desktop no Linux.
- `clipsync-app-v0.4.0-windows-x86_64-setup.exe` e `clipsync-app-v0.4.0-windows-x86_64.msi`: app de desktop no Windows.
- `clipsync-v0.4.0-linux-x86_64.tar.gz` e `clipsync-v0.4.0-windows-x86_64.zip`: só a linha de comando (`clipsync`), com este README.

Os pacotes do app e da CLI são separados. Para `clipsync status`, `diagnose` e os comandos de teste, baixe também o tar.gz no Linux ou o zip no Windows.

Confira antes de executar (troque `v0.4.0` pela tag que você baixou):

```sh
# Linux
sha256sum -c SHA256SUMS --ignore-missing
```

```powershell
# Windows (PowerShell): compare com a linha do pacote em SHA256SUMS
Get-FileHash .\clipsync-app-v0.4.0-windows-x86_64-setup.exe -Algorithm SHA256
```

### App de desktop

O app traz as telas e o agente no mesmo processo, com ícone na bandeja. Fechar a janela só esconde o app; para encerrar, use "Sair" no menu da bandeja.

No Linux, sem sudo, com o AppImage:

```sh
scripts/instalar-app-linux.sh ~/Downloads/clipsync-app-v0.4.0-linux-x86_64.AppImage
```

O script fica no repositório, não no release: ele copia o AppImage para `~/.local/bin/clipsync-app.AppImage` e cria o atalho "ClipSync" no menu de aplicativos. Sem o clone, copie o AppImage para `~/.local/bin`, marque como executável e crie o atalho à mão, ou use o `.deb`. Rodar de novo com um AppImage mais novo atualiza. O app precisa do WebKitGTK 4.1; se o AppImage não abrir por falta de FUSE, instale o `libfuse2` (no Ubuntu 24.04 ou mais novo, `libfuse2t64`). No GNOME, o ícone da bandeja depende da extensão AppIndicator. Com o `.deb`: `sudo apt install ./clipsync-app-v0.4.0-linux-amd64.deb`.

No Windows, rode o `setup.exe` (ou o `.msi`). O instalador não é assinado, então o SmartScreen avisa: clique em "Mais informações" e depois em "Executar assim mesmo". O app usa o WebView2, que já vem no Windows 10 e 11 atualizados; se faltar, o instalador baixa.

### Configurar a passagem do mouse

1. Instale a mesma versão nas duas máquinas (os pacotes estão em **Baixar**). No app, abra **Dispositivos** > **Parear novo dispositivo** em uma máquina e escolha **Aceitar**; na outra, escolha **Conectar** e digite o endereço mostrado. Confira o mesmo código de 6 dígitos nas duas e, no computador que vai receber o controle, marque **Pode controlar meu teclado e mouse**. Se algo não aparecer, abra **Diagnóstico** no menu lateral.
2. No computador com o mouse físico, abra **Layout de telas**, escolha a posição do outro computador ou arraste-o no desenho e clique em **Ligar nessa posição**. O app escolhe os monitores e os trechos livres.
3. Confira a passagem e a volta. **Conferir a passagem do mouse** oferece um teste de entrada; resultado sem eventos é inconclusivo. A escolha manual de bordas fica em **Ajustes avançados**.

O caminho normal da v0.4.0 é o `layout.toml`, com as ligações por PeerId. O `[[input.barrier]]` manual do `config.toml`, com `peer` pelo nome, é o caminho legado do loopback: o teste aceita qualquer par conhecido e conectado, por exemplo dois perfis na mesma máquina. Ter barreiras manuais e ligações no `layout.toml` ao mesmo tempo é recusado, com qualquer ligação, e não apenas por os dois arquivos existirem.

### Linha de comando

Requisitos: no Linux, glibc 2.39 ou mais nova (o binário é compilado no Ubuntu 24.04). No Windows, nenhum pacote extra: o `.exe` leva a biblioteca de execução embutida. O perfil fica em `~/.config/clipsync` no Linux e em `%LOCALAPPDATA%\clipsync` no Windows; não aponte `--profile` para uma pasta compartilhada, porque a chave privada mora nela.

Avisos para Windows (revisão de segurança S5, em `docs/seguranca.md`):

1. O perfil fica em `%LOCALAPPDATA%\clipsync` e guarda a chave privada. Não aponte `--profile` nem `CLIPSYNC_PROFILE` para raiz de disco, `C:\Users\Public`, `C:\ProgramData`, compartilhamento de rede ou pasta sincronizada (OneDrive, Dropbox). Só pasta dentro do seu perfil de usuário.
2. A proteção da chave é a ACL do seu usuário: qualquer programa que rode como você, e qualquer administrador, lê a chave.
3. Em máquina com mais de um usuário (compartilhada, RDP), o Windows ainda é experimental. Na CLI, pare o agente com Ctrl+C; no app, encerre pelo **Sair** do menu da bandeja. Se o console da CLI fechar sem passar pelo Ctrl+C, rode `clipsync run` de novo antes de usar `clipsync send`. O app fala com o agente pelo mesmo pipe nomeado da CLI, então a mesma limitação de pipe vale para a janela.
4. O clipboard do Windows lê tudo o que é copiado, inclusive o que gerenciadores de senha marcam como sensível. Use `clipsync pause` ao copiar senhas.
5. O texto recebido de outro dispositivo é gravado com os marcadores que pedem ao Windows para não guardar no histórico (Win+V) nem na nuvem. Isso depende de o Windows respeitar o marcador.
6. O clipboard do Windows é experimental: polling de 250 ms, só texto, ainda não testado em máquina real.
7. Teclado e mouse no Windows usam ganchos de baixo nível do sistema. O ClipSync só os instala com o módulo ligado e uma ligação desta máquina no layout, tira os ganchos ao pausar, não registra tecla nenhuma e não injeta nada na tela do UAC nem com a tela bloqueada. Programas abertos como administrador não recebem o teclado e o mouse de outra máquina.

O que funciona em cada plataforma nesta versão:

| Recurso | Linux (GNOME Wayland com XWayland) | Windows |
|---|---|---|
| Malha QUIC e pareamento | sim | sim |
| Clipboard de texto | sim | sim (só texto) |
| Teclado e mouse (layout de telas por posição) | experimental (GNOME Wayland) | experimental (atalhos Win+Alt+1 a 9, L e P) |

**Clipboard no Linux (revisão de segurança S5).** O `x-kde-passwordManagerHint` existe e o arboard o suporta, mas o ClipSync não o usa: no ClipSync para Linux esse tratamento não está implementado. O texto recebido é gravado como texto comum e a leitura não consulta marcador de conteúdo sensível, então o que o gerenciador de senhas marcar segue sujeito a ser lido e enviado. A pendência S5-03, em `docs/seguranca.md`, cobre a leitura no Windows.

Só a sessão gráfica do Ubuntu 26.04 com GNOME 50.1 foi testada. O binário Linux do release é compilado no `ubuntu-24.04`; o CI usa `ubuntu-latest`. O portal InputCapture existe desde o GNOME 45, mas versões anteriores não foram testadas; KDE e X11 puro não foram testados.

## Documentos

Ordem de leitura e regras de trabalho: `AGENTS.md`. Estado atual: `CONTEXTO.md` e `docs/workboard.md`; entrada comum dos andares: `docs/contexto-equipe.md`.

- `docs/contrato.md`: contrato vinculante. §15 registra a S9; §14 preserva a S8; §11 a S5 (Windows e release), §12 a interface e §13 o layout de telas. O pedido do usuário registrado aqui vence suposição.
- `docs/prompt-mestre.md`: especificação v4 de origem; o anexo detalhado do clipboard é `docs/prompt-mestre-anexo-original.md`.
- `docs/handoff-codex.md`: história de S1 a S7, anterior à S8.
- `docs/seguranca.md` e `docs/verdicts.md`: revisões de segurança e vereditos, uma seção por rodada.
- `docs/decisoes/`: ADRs (stack, transporte QUIC, pareamento).
- `crates/clipsync-app/ui/README.md`: guia da interface e do simulador; `docs/telas.md`: especificação da S6, base do protótipo no Figma.
- `docs/matriz-compatibilidade.md`: evidência dos spikes de 2026-10-27.
- `docs/teste-app-real.md`: como dirigir o app real pela acessibilidade (AT-SPI).
- `docs/releases/v0.3.1.md`: notas publicadas da versão.

## Requisitos de desenvolvimento

- Rust 1.99.1 via `rustup` (o `rust-toolchain.toml` fixa a versão). A CLI e os crates compilam com `build-essential`.
- O app Tauri 2 exige os headers e as libs do WebKitGTK (`libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`), instalados no CI. Sem eles, compile só a CLI e os crates, com `--exclude clipsync-app`.
- Linux GNOME Wayland para os portais de entrada; XWayland ativo (`DISPLAY` definido) para o clipboard. Só a sessão gráfica do Ubuntu 26.04 com GNOME 50.1 foi testada; o binário do release é compilado no `ubuntu-24.04` e o CI usa `ubuntu-latest`; KDE e X11 puro não foram testados.
- Sem sudo em nenhum passo.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
sh /tmp/rustup-init.sh -y --profile minimal --component clippy,rustfmt --no-modify-path
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release -p clipsync-cli      # binário em target/release/clipsync
cargo test --workspace --exclude clipsync-app --locked  # testes sem sessão gráfica
```

## Roteiro de teste de 10 minutos (numa máquina só)

Dois perfis no mesmo notebook, `a` e `b`, em portas diferentes. Os passos 4 e 5 trocam o conteúdo do clipboard. Para não mexer nele, crie os perfis com `--fake-adapters` e pule `probe clipboard --write`.

```sh
export PATH="$PWD/target/release:$PATH"

# 1. identidade, nome e módulos (a pergunta de clipboard tem padrão NÃO)
clipsync init --profile /tmp/cs-a --name notebook-a --modules input,clipboard --port 47800
clipsync init --profile /tmp/cs-b --name notebook-b --modules input,clipboard --port 47801

# 2. pareamento: os dois terminais mostram o mesmo código de 6 dígitos; confirme "s" nos dois
clipsync pair host --profile /tmp/cs-a --allow clipboard-in,clipboard-out
clipsync pair join --profile /tmp/cs-b 127.0.0.1:PORTA_IMPRESSA --allow clipboard-in,clipboard-out,input-in

# 3. agentes (um terminal cada) e estado
clipsync run --profile /tmp/cs-a
clipsync run --profile /tmp/cs-b
clipsync status --profile /tmp/cs-a

# 4. clipboard: observar sem roubar foco, escrever texto sintético, enviar entre os perfis
clipsync probe clipboard --watch 15s --profile /tmp/cs-a
clipsync probe clipboard --write --profile /tmp/cs-a
printf 'olá ção' | clipsync send --stdin --profile /tmp/cs-a
clipsync inbox --profile /tmp/cs-b

# 5. entrada: barreira só na metade de cima da borda direita, emulação, latência entre perfis
# os probes diretos usam adaptadores reais; CLIPSYNC_PROFILE mantém o token no perfil de teste
CLIPSYNC_PROFILE=/tmp/cs-a clipsync probe input --capture --profile /tmp/cs-a
CLIPSYNC_PROFILE=/tmp/cs-a clipsync probe input --emulate --profile /tmp/cs-a
clipsync probe input --loopback --to notebook-b --seconds 10 --profile /tmp/cs-a

# 6. pausa e diagnóstico
clipsync pause --profile /tmp/cs-b && clipsync resume --profile /tmp/cs-b
clipsync diagnose --profile /tmp/cs-a
```

Em `--allow`, `-in` é o que aquele perfil aceita receber e `-out` o que ele pode enviar; o loopback de `a` para `b` exige `input-in` em `b`, não em `a`. Os probes diretos `input --capture` e `input --emulate` usam os adaptadores reais, mesmo que o perfil tenha sido criado com `--fake-adapters`; o `--profile` do probe não chega ao adaptador, então o `--emulate` usa o perfil de `CLIPSYNC_PROFILE` para gravar o token do portal, não o pessoal.

Com `clipboard = false` no `config.toml` do perfil `b`, o `send` de `a` é recusado com motivo legível.

## Convivência com Input Leap

Enquanto o módulo de entrada não estiver validado entre máquinas reais, o Input Leap continua com teclado e mouse. Os dois não devem capturar ao mesmo tempo; no Linux, `clipsync diagnose` detecta Input Leap e Barrier ativos e orienta a pausa. Nada no Input Leap é alterado por este projeto.

## Licença

Todos os direitos reservados, até o autor escolher uma licença aberta. Sem licença, o código pode ser lido, mas não pode ser copiado, alterado nem redistribuído.

feito por montesouza.dev
