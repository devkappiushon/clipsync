# Uso

Guia do primeiro uso e dos comandos de teste. Para instalar os pacotes, veja [instalar](instalar.md). Limites de plataforma e avisos de segurança ficam em [limitações](limitacoes.md).

## Módulos

Teclado e mouse formam o módulo de entrada; o clipboard de texto é o outro. Dá para usar só um ou os dois. A escolha pode ser alterada no app em **Preferências > Módulos**, preservando identidade e pareamentos. `clipsync init` cria ou recria a identidade e só deve ser usado na criação do perfil, não para trocar módulos de um perfil existente.

## Configurar a passagem

1. Instale a mesma versão nas duas máquinas (os pacotes estão em [instalar](instalar.md#baixar)). No app, abra **Dispositivos** > **Parear novo dispositivo** em uma máquina e escolha **Aceitar**; na outra, escolha **Conectar** e digite o endereço mostrado. Confira o mesmo código de 6 dígitos nas duas. No computador que receberá o controle, em **Permissões iniciais**, ative “{nome do par} pode controlar o teclado e mouse deste computador”, confirme **Conceder controle** e clique em **Concluir**. Se algo não aparecer, abra **Diagnóstico** no menu lateral.
2. Abra **Layout de telas** e use **Preparar telas** em cada máquina que precise atualizar seu inventário. No notebook, confirme pessoalmente eventual diálogo do portal. Depois, no desktop Windows com o mouse físico, posicione as telas no rascunho da seção **Organizar cada tela**, por exemplo com o notebook entre A e B.
3. Use **Conferir prévia** e depois **Aplicar organização**. Para testar, abra **Ajustes avançados**, selecione o outro computador no desenho e acione **Testar passagem (5 s)**. O teste pode controlar o destino e pedir permissão do sistema. Depois de iniciá-lo, empurre a borda ligada e mova o mouse; resultado sem eventos é inconclusivo, e a volta deve ser conferida manualmente.

O caminho normal da v0.5.0 é o `layout.toml` no formato v2, com as ligações por PeerId. O `[[input.barrier]]` manual do `config.toml`, com `peer` pelo nome, é o caminho legado do loopback: o teste aceita qualquer par conhecido e conectado, por exemplo dois perfis na mesma máquina. Ter barreiras manuais e ligações no `layout.toml` ao mesmo tempo é recusado, com qualquer ligação, e não apenas por os dois arquivos existirem.

A conexão aceita protocolo 1 ou posterior; a organização por monitor exige protocolo 4 nas duas pontas. Protocolo 4 não é a versão do aplicativo.

## Roteiro de 10 minutos

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

## Input Leap

Enquanto o módulo de entrada não estiver validado entre máquinas reais, o Input Leap continua com teclado e mouse. O ClipSync foi criado para substituir o Input Leap, mas essa substituição ainda depende da validação física. Os dois não devem capturar ao mesmo tempo; no Linux, `clipsync diagnose` detecta Input Leap e Barrier ativos e orienta a pausa. Nada no Input Leap é alterado por este projeto.
