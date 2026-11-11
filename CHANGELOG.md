# Changelog

Todas as versões publicadas do ClipSync, da mais nova para a mais antiga. Cada release no GitHub traz o app de desktop (Linux: AppImage e `.deb`; Windows: `setup.exe` e `.msi`), a linha de comando (`.tar.gz` e `.zip`) e o `SHA256SUMS`. Nenhuma versão declara a travessia física do mouse entre máquinas reais como comprovada; veja [pendências da v0.5.0](docs/pendencias-v0.5.0.md).

## Não publicado

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
