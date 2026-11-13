# Limitações

A v0.5.3 está publicada no GitHub. Publicar não comprova o funcionamento físico. Este guia reúne o que ainda falta validar e os avisos por plataforma. Os detalhes de instalação ficam em [instalar](instalar.md) e os passos de uso em [uso](uso.md).

O clipboard transporta apenas texto. Não há suporte a imagens, arquivos ou rich text.

**Privacidade:** o clipboard pode transmitir conteúdo marcado como sensível nas duas plataformas. Use `clipsync pause` ao copiar senhas. Os detalhes estão abaixo e no histórico de [segurança](seguranca.md).

## Windows

Avisos da revisão de segurança S5, preservados de [segurança](seguranca.md):

1. O perfil fica em `%LOCALAPPDATA%\clipsync` e guarda a chave privada. Não aponte `--profile` nem `CLIPSYNC_PROFILE` para raiz de disco, `C:\Users\Public`, `C:\ProgramData`, compartilhamento de rede ou pasta sincronizada (OneDrive, Dropbox). Só pasta dentro do seu perfil de usuário.
2. A proteção da chave é a ACL do seu usuário: qualquer programa que rode como você, e qualquer administrador, lê a chave.
3. Em máquina com mais de um usuário (compartilhada, RDP), o Windows ainda é experimental. Na CLI, pare o agente com Ctrl+C; no app, encerre pelo **Sair** do menu da bandeja. Se o console da CLI fechar sem passar pelo Ctrl+C, rode `clipsync run` de novo antes de usar `clipsync send`. O app fala com o agente pelo mesmo pipe nomeado da CLI, então a mesma limitação de pipe vale para a janela.
4. O clipboard do Windows lê o texto copiado, inclusive o texto que gerenciadores de senha marcam como sensível. Use `clipsync pause` ao copiar senhas.
5. O texto recebido de outro dispositivo é gravado com os marcadores que pedem ao Windows para não guardar no histórico (Win+V) nem na nuvem. Isso depende de o Windows respeitar o marcador.
6. O clipboard do Windows é experimental: polling de 250 ms, só texto, ainda não testado em máquina real.
7. Teclado e mouse no Windows usam ganchos de baixo nível do sistema. O ClipSync só os instala com o módulo de entrada ligado e uma ligação desta máquina no layout, tira os ganchos ao pausar e não registra tecla nenhuma. O controle remoto fica bloqueado na tela do UAC e com a tela bloqueada; a soltura de teclas e botões ainda é permitida. Programas abertos como administrador não recebem o teclado e o mouse de outra máquina.

## Linux

O `x-kde-passwordManagerHint` existe e o arboard o suporta, mas o ClipSync não o usa: no ClipSync para Linux esse tratamento não está implementado. O texto recebido é gravado como texto comum e a leitura não consulta marcador de conteúdo sensível, então o que o gerenciador de senhas marcar segue sujeito a ser lido e enviado. A pendência S5-03, em [segurança](seguranca.md), cobre a leitura no Windows.

## Plataformas

A matriz abaixo resume o que está disponível nesta versão:

| Recurso | Linux (GNOME Wayland, XWayland ativo com `DISPLAY` definido) | Windows 10+ x86_64 |
|---|---|---|
| Malha QUIC e pareamento | sim | sim |
| Clipboard de texto | sim, só texto | sim, só texto |
| Teclado e mouse por posição | experimental no GNOME Wayland | experimental; atalhos Win+Alt+1 a 9, L e P quando disponíveis |

Só a sessão gráfica do Ubuntu 26.04 com GNOME 50.1 foi testada; isso é evidência histórica, não um requisito universal. O binário Linux do release é compilado no `ubuntu-24.04`; o CI usa `ubuntu-latest`. O portal InputCapture existe desde o GNOME 45, mas versões anteriores não foram testadas; KDE e X11 puro não foram testados.

## Compatibilidade

O layout da v0.5.0 usa formato v2. A v0.4.0 não lê esse formato; para voltar, use a recuperação v1 em **Ajustes avançados** de um aplicativo que ofereça essa recuperação, com backup válido. Abrir ou preparar telas não migra o layout. O passo a passo está em [monitores independentes](monitores-independentes.md#6-backup-recuperação-e-ajustes-avançados).

A conexão aceita protocolo 1 ou posterior; a organização por monitor exige protocolo 4 nas duas pontas. Protocolo 4 não é a versão do aplicativo.

## O que falta validar

A publicação da v0.5.3 não comprova a travessia física A ↔ notebook ↔ B, a posição real do cursor nem a soltura de teclas e botões. Também faltam validação da interação Tauri nas duas máquinas, de escalas e coordenadas negativas, de pausa e revogação, de UAC e tela bloqueada, de desconexão e de permissões reais. Leitor de tela e zoom do app exigem verificação própria. O teclado na primeira travessia ainda exige validação física.

Testes automáticos, adaptadores falsos, CI e simulador não substituem Windows e GNOME físicos. As pendências da v0.5.0 estão em [pendências da v0.5.0](pendencias-v0.5.0.md); as da v0.4.0 continuam em [pendências da v0.4.0](pendencias-v0.4.0.md), e as notas históricas estão em [v0.4.0](releases/v0.4.0.md).

O achado menor T4-SEG-01 continua diferido: há uma corrida local entre verificar e abrir o caminho de backup, sem vetor remoto demonstrado nem bypass da recuperação. Esta publicação não encerra todas as pendências de segurança; o histórico está em [segurança](seguranca.md).
