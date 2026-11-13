# Changelog

Todas as versões publicadas do ClipSync, da mais nova para a mais antiga. Cada release no GitHub traz o app de desktop (Linux: AppImage e `.deb`; Windows: `setup.exe` e `.msi`), a linha de comando (`.tar.gz` e `.zip`) e o `SHA256SUMS`. Nenhuma versão declara a travessia física do mouse entre máquinas reais como comprovada; veja [pendências da v0.5.0](docs/pendencias-v0.5.0.md).

## [v0.5.3](https://github.com/devkappiushon/clipsync/releases/tag/v0.5.3) · 13/11/2026

O PC devolve o controle sozinho quando o notebook para de responder, e avisa. [Notas completas](docs/releases/v0.5.3.md).

- **O PC devolve o mouse e o teclado sozinho.** O PC pergunta ao notebook a cada 0,5 s. Sem resposta válida por cerca de 2 s, encerra a sessão e solta a captura, devolvendo o controle à própria máquina. A soltura no notebook é uma tentativa e pode não chegar se ele estiver inacessível. No notebook, entrada que não pode ser aplicada encerra a sessão em vez de continuar sem aplicar, com a exceção da falta de teclado descrita no item seguinte.
- **Aviso quando o controle volta sozinho.** Ao devolver o controle, o PC notifica `Controle devolvido a este computador`, com o motivo, e lembra que as próximas teclas serão digitadas nele. O aviso sai depois da devolução, depende da consulta de estado de 1 s e das notificações do sistema, e não impede a digitação na janela errada.
- **Teclado indisponível não derruba mais o mouse.** Teclas do episódio são descartadas e contadas, sem repetição depois. O mouse continua. A interface do notebook recebe o aviso `Teclado indisponível neste computador: as teclas não estão sendo aplicadas` após o primeiro descarte, e as solturas pendentes saem quando o teclado volta. A apresentação na janela ainda não foi conferida.
- **Registro do ciclo dos dispositivos de entrada no notebook.** O log passa a ter `eis.device` e `eis.loop`, só com ações, capacidades e contagens, para separar PC de notebook quando o teclado não responde. Nenhuma tecla ou código de tecla é registrado.
- **Reencontro pelo nome com cache separado.** O endereço achado fica em `known-addrs.toml`, ao lado do pareamento, e o `peers.toml` não muda. Outro aparelho no endereço em cache só descarta o cache. Os endereços respondidos são tentados em rodízio, com espera curta, e só o nome de um rótulo é consultado, como `NOME` e `NOME.local`.
- **Regra do UFW restrita à rede local.** [Instalar](docs/instalar.md#rede-e-firewall) passa a recomendar a regra do UFW restrita à faixa da rede local, no lugar da regra ampla da v0.5.2.

Ressalvas: a causa do teclado mudo na primeira travessia não foi provada; nada foi validado em GNOME, Windows ou rede reais; endereço que a v0.5.2 tenha gravado errado não é saneado, e o caso afetado exige parear de novo. O gancho do Windows que para de enviar (F3) não está corrigido nesta versão. A notificação de retorno não foi vista no Windows real nem com a janela minimizada.

## [v0.5.2](https://github.com/devkappiushon/clipsync/releases/tag/v0.5.2) · 11/11/2026

Reencontro automático do par quando o IP muda. [Notas completas](docs/releases/v0.5.2.md).

- Quando o endereço guardado do par para de responder (o DHCP deu outro IP), o ClipSync procura o par pelo nome da máquina na rede, conecta e grava o endereço novo. Antes, era preciso parear de novo.
- A identidade continua valendo: um endereço achado pelo nome só é aceito depois que o par prova a chave do pareamento. Outro aparelho no mesmo endereço é ignorado e não bloqueia o par.
- [Instalar](docs/instalar.md) explica a porta UDP 47800 e a regra do UFW no Linux.

## [v0.5.1](https://github.com/devkappiushon/clipsync/releases/tag/v0.5.1) · 10/11/2026

Log verificável da travessia e correção do mouse preso depois de suspender o notebook. [Notas completas](docs/releases/v0.5.1.md).

- Destino Linux: quando o GNOME encerra a sessão de controle remoto (suspensão ou bloqueio da tela), a próxima entrada abre uma sessão nova. Antes, o cursor ficava num limbo até reiniciar o app.
- Log em arquivo no perfil (`clipsync log caminho`) com tentativa, barreira, entrada, emulação e retorno, nas duas máquinas.
- `clipsync verificar-travessia` lê os logs da origem e do destino e diz onde a travessia parou. Guia em [verificar por log](docs/verificar-por-log.md).
- Pacote de suporte inclui a cauda do log, sem texto livre, conteúdo, tecla ou keycode.
- Interface: atalhos Win+Alt aparecem só onde o sistema os registra (Windows com o módulo de entrada ativo); tabela de Dispositivos sem rolagem horizontal; sem vão acima do primeiro aviso; cabeçalho de coluna na fonte de texto.
- Documentação: README com capturas e guias de [instalação](docs/instalar.md), [uso](docs/uso.md), [limitações](docs/limitacoes.md) e [desenvolvimento](docs/desenvolvimento.md); dados pessoais anonimizados nos documentos.

## [v0.5.0](https://github.com/devkappiushon/clipsync/releases/tag/v0.5.0) · 31/10/2026

Organização por monitor e disponibilidade mais clara. [Notas completas](docs/releases/v0.5.0.md).

- Editor de Layout de telas com monitores independentes: A ↔ notebook ↔ B num desktop com duas telas.
- Fluxo explícito: Preparar telas, Conferir prévia, Aplicar organização; Copiar oferece o mapa ao outro dispositivo.
- Layout no formato v2, com backup e recuperação do v1 em Ajustes avançados. A v0.4.0 não lê v2.
- Protocolo 4 (mínimo 1): a organização por monitor exige v4 nas duas pontas.
- Dispositivos, Pareamento, Clipboard e Diagnóstico distinguem suporte ausente, módulo desligado e estado não confirmado.
- Job Node no CI para os testes da interface.

## [v0.4.0](https://github.com/devkappiushon/clipsync/releases/tag/v0.4.0) · 30/10/2026

Configuração de módulos pelo app e correções de pausa, controle de entrada, pareamento e interface. [Notas completas](docs/releases/v0.4.0.md).

- Preferências > Módulos liga e desliga teclado e mouse ou clipboard sem recriar a identidade.
- Pausa e revogação invalidam sessões antigas e preservam a soltura de teclas.
- Windows guarda o registro de teclas possivelmente pressionadas em falhas parciais.
- Diagnóstico agrupa recusas repetidas; exportação de suporte remove nomes e endereços históricos.
- Atalhos de zoom na janela principal; ajustes de contraste e navegação por teclado.

## [v0.3.1](https://github.com/devkappiushon/clipsync/releases/tag/v0.3.1) · 28/10/2026

Configuração mais simples da passagem do mouse. [Notas completas](docs/releases/v0.3.1.md).

- Layout por posição: escolha onde fica o outro computador e clique em Ligar nessa posição; bordas ficam em Ajustes avançados.
- Aviso de protocolo antigo, teste de entrada com resultado explicado e `clipsync diagnose` com o contexto do perfil.
- O teste interrompe o envio e solta o que estava pressionado quando a sessão de origem é bloqueada.

## [v0.3.0](https://github.com/devkappiushon/clipsync/releases/tag/v0.3.0) · 28/10/2026

- Teclado e mouse no Windows: o cursor cruza entre Windows e Linux pelo trecho de borda ligado na tela Layout, nos dois sentidos, com volta pelo mesmo trecho.
- Atalhos Win+Alt+1 a 9, L e P no Windows.
- Protocolo 3.

## [v0.2.1](https://github.com/devkappiushon/clipsync/releases/tag/v0.2.1) · 28/10/2026

- Correções sobre a v0.2.0, com os mesmos pacotes.

## [v0.2.0](https://github.com/devkappiushon/clipsync/releases/tag/v0.2.0) · 28/10/2026

- Primeiro app de desktop (Tauri 2): nove telas, agente dentro do app e ícone na bandeja.
- Linux: AppImage e `.deb`. Windows: `setup.exe` e `.msi`, sem assinatura.

## [v0.1.0](https://github.com/devkappiushon/clipsync/releases/tag/v0.1.0) · 27/10/2026

- Só linha de comando, Linux e Windows.
- Linux (X11 e XWayland): malha QUIC, pareamento, clipboard e teclado e mouse experimentais.
- Windows: malha QUIC, pareamento e clipboard de texto experimental; sem teclado e mouse.
