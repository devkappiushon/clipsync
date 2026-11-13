# Uso

Guia do primeiro uso e dos comandos de teste. Para instalar os pacotes, veja [instalar](instalar.md). Limites de plataforma e avisos de segurança ficam em [limitações](limitacoes.md).

## Módulos

Teclado e mouse formam o módulo de entrada; o clipboard de texto é o outro. Dá para usar só um ou os dois. A escolha pode ser alterada no app em **Preferências > Módulos**, preservando identidade e pareamentos. `clipsync init` cria ou recria a identidade e só deve ser usado na criação do perfil, não para trocar módulos de um perfil existente.

## Configurar a passagem

1. Instale a mesma versão nas duas máquinas (os pacotes estão em [instalar](instalar.md#baixar)). No app, abra **Dispositivos** > **Parear novo dispositivo** em uma máquina e escolha **Aceitar**; na outra, escolha **Conectar** e digite o endereço mostrado. Confira o mesmo código de 6 dígitos nas duas. No computador que receberá o controle, em **Permissões iniciais**, ative “{nome do par} pode controlar o teclado e mouse deste computador”, confirme **Conceder controle** e clique em **Concluir**. Se algo não aparecer, abra **Diagnóstico** no menu lateral.
2. Abra **Layout de telas** e use **Preparar telas** em cada máquina que precise atualizar seu inventário. No notebook, confirme pessoalmente eventual diálogo do portal. Depois, no desktop Windows com o mouse físico, posicione as telas no rascunho da seção **Organizar cada tela**, por exemplo com o notebook entre A e B.
3. Use **Conferir prévia** e depois **Aplicar organização**. Para testar, abra **Ajustes avançados**, selecione o outro computador no desenho e acione **Testar passagem (5 s)**. O teste pode controlar o destino e pedir permissão do sistema. Depois de iniciá-lo, empurre a borda ligada e mova o mouse; resultado sem eventos é inconclusivo, e a volta deve ser conferida manualmente.

Desde a v0.5.0, o caminho normal é o `layout.toml` no formato v2, com as ligações por PeerId. O `[[input.barrier]]` manual do `config.toml`, com `peer` pelo nome, é o caminho legado do loopback: o teste aceita qualquer par conhecido e conectado, por exemplo dois perfis na mesma máquina. Ter barreiras manuais e ligações no `layout.toml` ao mesmo tempo é recusado, com qualquer ligação, e não apenas por os dois arquivos existirem.

A conexão aceita protocolo 1 ou posterior; a organização por monitor exige protocolo 4 nas duas pontas. Protocolo 4 não é a versão do aplicativo.

## Roteiro de 10 minutos

Dois perfis no mesmo notebook, `a` e `b`, em portas diferentes. Os passos 4 e 5 trocam o conteúdo do clipboard. Para não mexer nele, crie os perfis com `--fake-adapters` e pule `probe clipboard --write`.

```sh
export PATH="$PWD/target/release:$PATH"

# 1. identidade, nome e módulos explícitos; sem pergunta de clipboard
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

Para testar o módulo desligado, encerre o agente de `b` com Ctrl+C, defina `clipboard = false` na seção `[modules]` do `config.toml` desse perfil e inicie novamente `clipsync run --profile /tmp/cs-b`. Depois, repita o `send` de `a`: o envio é recusado com motivo legível. Editar o arquivo não recarrega os módulos do agente que já está rodando.

## Solução de problemas

Os comportamentos abaixo valem a partir da v0.5.3. Não houve validação em GNOME, Windows ou rede reais.

**O mouse não volta ao PC.** O PC espera resposta do notebook e, depois de cerca de 2 s sem resposta válida, encerra a sessão e devolve o controle a ele mesmo. Nessa hora o PC notifica `Controle devolvido a este computador`, com o motivo: daí em diante as teclas são digitadas no próprio PC, então confira a janela antes de continuar. O aviso chega depois da devolução e depende das notificações do sistema; o motivo também fica no app, em Último bloqueio registrado. No notebook, entrada que não pode ser aplicada também encerra a sessão, em vez de seguir sem aplicar. A exceção é a falta apenas de teclado, tratada no caso seguinte: nela o mouse continua. A devolução do controle é local: se o notebook estiver inacessível, a soltura do outro lado é uma tentativa e pode não chegar. Espere a sessão encerrar sozinha. Confira que os dois apps estão abertos e na mesma rede. Abra **Diagnóstico** e veja o log; o passo a passo está em [verificar por log](verificar-por-log.md).

**Aviso de teclado indisponível no notebook.** O aviso é `Teclado indisponível neste computador: as teclas não estão sendo aplicadas`. A interface do notebook recebe o aviso após o primeiro descarte. A apresentação na janela ainda não foi conferida. As teclas do episódio são descartadas e não são reaplicadas depois; o mouse continua funcionando e o aviso some quando o teclado volta. Use o teclado do próprio notebook. Se o aviso continuar, confira o diálogo do portal no notebook e entre de novo na sessão. A causa do teclado mudo na primeira travessia não foi provada.

**O par não é encontrado depois de o IP mudar.** O ClipSync guarda o endereço achado pelo nome em `known-addrs.toml`, separado do pareamento. O `peers.toml` não muda, e outro aparelho no endereço em cache só descarta o cache. A busca usa o nome do dispositivo como um rótulo, consultando `NOME` e `NOME.local`. Se a ligação não voltar, confira a regra de firewall em [Rede e firewall](instalar.md#rede-e-firewall). Endereço que a v0.5.2 tenha gravado errado exige parear de novo; fora desse caso, o pareamento não precisa ser refeito.

## Input Leap

Enquanto o módulo de entrada não estiver validado entre máquinas reais, o Input Leap continua com teclado e mouse. O ClipSync foi criado para substituir o Input Leap, mas essa substituição ainda depende da validação física. Os dois não devem capturar ao mesmo tempo; no Linux, `clipsync diagnose` detecta Input Leap e Barrier ativos e orienta a pausa. Nada no Input Leap é alterado por este projeto.
