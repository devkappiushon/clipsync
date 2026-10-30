# 0003 · Pareamento por comparação numérica com compromisso

Estado: aceita (2026-10-27) · Autor: Keel · Contrato: `docs/contrato.md` §3, §4 (`PairMsg`) e §5 regra 3 · Evidência: `/tmp/clipsync-spike-base/RELATORIO.md` (log efêmero, não preservado)

## Contexto

Não há servidor central nem autoridade certificadora. Cada máquina tem um certificado autoassinado. O usuário confirma o pareamento comparando um código curto nas duas telas.

Um código de 6 dígitos tirado só do `export_keying_material` é forçável por quem está no meio durante o pareamento. O atacante termina as duas sessões TLS, e cada nova assinatura no CertificateVerify gera outro exporter. Com cerca de 10^6 tentativas ele faz os dois códigos coincidirem, em segundos (o spike raciocinou esse ataque, não o demonstrou).

## Decisão

1. Identidade: chave **ECDSA P-256** gerada pelo rcgen, certificado autoassinado. `PeerId` = SHA-256 do SPKI DER. Motivos: P-256 é o que o spike provou de ponta a ponta, e é o algoritmo que TPM, Windows CNG e o Secure Enclave do macOS guardam em hardware, para quando a chave sair do arquivo e for para o cofre. (Isto substitui o Ed25519 da primeira versão do contrato.)
2. O host abre uma janela de 120 s numa porta efêmera, com ALPN `clipsync-pair/1`. O TLS é mútuo e aceita qualquer pino, mas exige posse da chave.
3. No fluxo de controle, depois de trocar o Hello, nesta ordem:
   1. B (join) envia `Commit{SHA-256("clipsync-pair-commit-v1" ‖ nB)}`.
   2. A (host) envia `Nonce{nA}`.
   3. B revela `Reveal{nB}`.
   4. A confere o compromisso.

   nA e nB têm 32 bytes aleatórios novos por tentativa.
4. Código = u64 big-endian de `export_keying_material(8 bytes, "EXPORTER-clipsync-pair-v1", nB ‖ nA)` mod 1 000 000, mostrado como `NNN NNN`.
5. **Até 3 tentativas por janela** (revisto em 2026-10-27, contrato v1.10, achado S1-04 de `docs/seguranca.md`).
   - Uma tentativa começa no primeiro `Commit`, recebido numa conexão com handshake completo e Hello válido.
   - Antes disso nada conta: sonda, handshake falho, ALPN errado ou Hello inválido. Assim, um host qualquer da LAN não queima a janela.
   - Recusa, código divergente ou timeout depois do Commit contam como tentativa. A 3ª falha fecha a janela com `CloseCode::PairFailed`.
   - Reveal que não confere com o Commit fecha a janela na hora: é prova de ataque.
   - A implementação é pós-MVP. Até lá vale a primeira versão: uma tentativa, e qualquer falha fecha.
6. Cada lado mostra o código, o nome e o id curto do outro. Depois pergunta as permissões só dos módulos ligados. Tudo vem desligado por padrão, e o controle de teclado e mouse traz o aviso de confiança total. O `peers.toml` só é gravado se os dois lados aceitarem.

## Por que o compromisso resolve

B se compromete com nB antes de conhecer nA, e A escolhe nA depois de receber o compromisso. Quem está no meio precisa fixar o seu nB do lado de A antes de ver o nA que A vai sortear. Então o código daquele lado sai aleatório para ele, e a chance de coincidir é 1 em 10^6 por tentativa. Cada tentativa sorteia um nA novo depois do Commit, então é um sorteio independente. Com 3 tentativas, o teto por janela é 3 em 10^6. Um Reveal falso fecha a janela na hora.

## Consequências

- O pareamento custa 1 ida e volta a mais (~1,6 ms em loopback para handshake + compromisso).
- Se o usuário errar ou recusar, roda `pair host` de novo.
- A proteção depende de o usuário comparar o código de verdade. A interface diz isso.
- Chave privada em arquivo 0600 no perfil, **provisório**, até existir o cofre da plataforma (Secret Service, CNG, Keychain).
- Reconexões não repetem o pareamento: o pino em `peers.toml` decide. Se a chave do par mudar, ele fica bloqueado até um novo pareamento.
