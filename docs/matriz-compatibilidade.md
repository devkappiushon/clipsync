# Matriz de compatibilidade

Classificação: **validada** (visto funcionar nesta máquina, com log) · **parcial** (parte provada, parte pendente) · **não testada** · **falhou**.
Cada célula aponta a evidência. Nada aqui vale para outra máquina até rodar lá.

| Plataforma | Clipboard: observar | Clipboard: ler | Clipboard: escrever | Input: captura | Input: emulação |
|---|---|---|---|---|---|
| Ubuntu 26.04.1 · GNOME 50.1 · Wayland · x86_64 (notebook `kappxs-Aspire-A515-57`) | validada [C1] | validada [C2] | validada [C3] | parcial [I1] | validada [I2] |
| Windows (PC servidor do Input Leap, versão a confirmar) | não testada | não testada | não testada | não testada | não testada |
| PC de casa (SO a confirmar) | não testada | não testada | não testada | não testada | não testada |

## Evidência · Ubuntu 26.04 / GNOME 50.1 / Wayland (spikes de 2026-10-27)

**[C1] Observar.** Usa XFixes `SetSelectionOwner` do CLIPBOARD via XWayland (`DISPLAY=:0`). Não precisa de janela X11 focada.
- 10 de 10 cópias feitas em app Wayland nativo geraram evento (`/tmp/clipsync-spike-clipboard/watch.log`).
- A escrita do próprio programa também gera evento (~2 ms), até a re-escrita idêntica. Quando o app de origem morre, o Mutter assume a posse e gera um segundo evento com o mesmo conteúdo. O contrato trata os dois casos (§7, regras 2 e 3).
- Latência entre a cópia e o evento: o Flux mediu ≤ 200 ms. No `watch.log`, o campo `lat_ms_vs_texto` vai de 194 a 1776 ms e cresce cerca de 100 ms por evento. Esse padrão sugere deriva do método de medição, não latência real. **Medir de novo na P1** antes de publicar número.
- Sem DISPLAY (XWayland desligado), esta ponte não existe. O compositor não tem `zwlr_data_control` nem `ext_data_control`.

**[C2] Ler.** Lê TARGETS + UTF8_STRING em 35 a 63 ms depois do evento (`watch.log`).
- Clipboard limpo: TARGETS vazio. String vazia: só `application/x-zerosize`, sem UTF8_STRING (`w3.log`). Os dois casos contam como "vazio, não sincronizar".

**[C3] Escrever.** Escreve via arboard (X11). A leitura de volta é idêntica e apps Wayland leem o texto (relato do spike do Flux).
- Só texto UTF-8. Imagem, arquivos e PRIMARY não foram testados.

**[I1] Captura (parcial).** Portal InputCapture v1 (`/tmp/clipsync-spike-input/capture.log`, `cap_*.log`).
- Funcionou:
  - sessão criada com Keyboard e Pointer concedidos, depois do diálogo, aceito pelo usuário em 8 a 47 s;
  - 1 zona 1920x1080 em (0,0);
  - barreira só na metade superior da borda direita (`x=1920 y=0..539`) aceita, com `failed=[]`;
  - handshake EIS ok, Enable ok, dispositivos EIS virtuais recebidos.
- Pendente:
  - o evento `Activated` não chegou em 3 tentativas ("nunca ativou"). Numa delas o RemoteDesktop empurrou o ponteiro até a barreira. Hipótese a provar: o Mutter não dispara barreira com ponteiro emulado, e só o mouse físico ativa;
  - eventos por EIS depois de ativar e latência.
- O portal não dá nome à zona. O nome `eDP-1` precisa vir de outra fonte (contrato §10, item 3).
- InputCapture v1 não tem token de restauração. O diálogo deve voltar a cada sessão de login (a medir).

**[I2] Emulação (validada para movimento).** Portal RemoteDesktop v2 (`/tmp/clipsync-spike-input/emit.log`).
- Funcionou:
  - Start ok, com Keyboard e Pointer concedidos, depois do diálogo;
  - handshake EIS como emissor ok;
  - 486 movimentos relativos enviados (dx +50 por 5 s).
- Limitações:
  - o dispositivo não oferece PointerAbsolute. A posição de entrada na borda tem de ser alcançada com movimento relativo;
  - digitação ainda não testada ("ção" com dead keys e AltGr, ABNT2). Fica para a P4b;
  - o token de restauração do RemoteDesktop v2 ainda não foi usado (diálogo uma vez por máquina, a provar).

## Base (independe da plataforma, só loopback nesta máquina)

QUIC + TLS 1.3 mútuo com pino, pareamento com compromisso, fluxo e datagrama: validados em loopback, num processo só (`/tmp/clipsync-spike-base/RELATORIO.md`, ADR 0002 e 0003). Ainda não testados: processos separados, rede real, migração de IP e captura de pacotes.
