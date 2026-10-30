# 0002 · Transporte: QUIC com TLS 1.3 mútuo

Estado: aceita (2026-10-27) · Autor: Keel · Contrato: `docs/contrato.md` §4 e §5 · Evidência: `/tmp/clipsync-spike-base/RELATORIO.md` (spike do Rivet; log efêmero, não preservado)

## Contexto

O clipboard precisa de entrega confiável. A entrada precisa de latência baixa, e um evento atrasado não pode segurar os seguintes. As duas pontas se autenticam sem autoridade central. O IP do notebook muda entre redes.

## Decisão

1. **Um transporte só: QUIC (quinn 0.11.12, rustls 0.23.46, ring).** ALPN `clipsync/1` para a malha e `clipsync-pair/1` para o pareamento.
2. Canais numa mesma conexão:
   - um fluxo bidi de controle, aberto por quem disca (Hello, Ping, Bye);
   - um fluxo uni por mensagem de módulo, para não haver bloqueio de cabeça de fila entre mensagens;
   - datagramas para eventos de entrada.
3. Enquadramento: u32 big-endian + corpo postcard. O tamanho é checado **antes** de alocar. Limites fixos:
   - 64 KiB no controle e 1 MiB + 64 KiB nos fluxos de módulo;
   - 1200 B e até 32 eventos por datagrama.
   Quadro grande demais: `stop` + RESET do fluxo.
4. TLS mútuo com um verificador único (`PinVerifier`) que faz os dois papéis:
   - valida o X.509 com o próprio certificado como âncora;
   - verifica a assinatura TLS 1.3 de verdade;
   - o pino é o SHA-256 do SPKI.
5. **Retomada TLS 1.3 desligada.** No servidor: `NoServerSessionStorage` e `send_tls13_tickets = 0`. No cliente: `Resumption::disabled()`.
6. O par só conta como conectado depois de um Hello de aplicação autenticado nos dois sentidos.
7. O CONNECTION_CLOSE e o Bye levam só um `CloseCode` curto e fixo. Texto de erro do rustls ou do quinn nunca vai ao par.

## Alternativa rejeitada

TLS sobre TCP para o clipboard + UDP separado para a entrada. Seriam duas pilhas, dois handshakes e dois caminhos de autenticação para revisar. O UDP precisaria de DTLS ou de cifra própria, e a conexão não migra quando o IP muda. O QUIC entrega fluxos confiáveis, datagramas, TLS 1.3 mútuo e migração num lugar só.

## Evidência (spike, loopback 127.0.0.1, 5 rodadas em release)

- Fluxo, quadro de 1 KiB ida e volta: p50 de 25 a 51 µs, p95 de 56 a 73 µs.
- Datagrama de 64 B: p95 de 23 a 89 µs, 0 perdidos em 1000. `max_datagram_size` = 1414 B.
- Quadro anunciado com 64 KiB + 1: recusado sem ler o corpo, RESET_STREAM.
- Com a retomada no padrão do rustls, a 2ª e a 3ª conexão retomam por PSK e o verificador **não roda** (0 chamadas em 5 de 5 rodadas). Por isso a retomada fica desligada.
- O `connect()` do cliente volta antes de o servidor validar o certificado do cliente. Por isso o Hello nos dois sentidos é obrigatório.
- No padrão, o texto Debug do erro do rustls vai no campo reason do CONNECTION_CLOSE. Por isso o `CloseCode` é obrigatório.

## Consequências

- Sem 0-RTT: toda reconexão faz o handshake completo, ~1,6 ms em loopback.
- **Exceção aceita no MVP (contrato v1.7):** em falha de handshake, o reason do CONNECTION_CLOSE ainda é o texto do rustls. Para ele não vazar nada, o PinVerifier devolve só erros fixos (`ApplicationVerificationFailure`, `UnknownIssuer`), e o reason fica uma string constante, sem pino nem Debug. Trocar esse texto por `CloseCode::reason()` exige embrulhar `quinn::crypto` via quinn-proto: pendente para depois do MVP.
- O datagrama limita a rajada de entrada a 32 eventos. Acima disso, a rajada vai em vários datagramas.
- Não verificado ainda: migração de conexão com troca real de IP; ausência de texto em claro numa captura (tshark ou tcpdump); se o CONNECTION_CLOSE de recusa sai em pacote Initial visível na rede; latência em Wi-Fi (o loopback não representa a rede real).
