# Contexto — clipsync

**Atualizado:** 2026-11-04 (sessão do Maestro · scaffold)

## Onde estou
A v0.5.0 está publicada (31/10/2026, commit `590c137`, o mesmo da `main`). Esta sessão preparou o repositório para ficar público, fechou a documentação da versão (andar DOC: README e quatro guias) e concluiu a revisão visual da interface. Tudo está no checkout, nada commitado.

## Estado real (verificado)
- `main` = tag `v0.5.0` = `590c137`; release com 6 pacotes + `SHA256SUMS`, CI `41161430657` e empacotamento `41161435861` verdes (`gh release view`, `gh run list`).
- Árvore sem segredo, chave ou token; caminhos `/home/...`, nome de host, IP da LAN e ids reais anonimizados em `docs/` (`git grep`: zero ocorrências). O histórico do git ainda os contém; reescrever invalidaria as 7 tags e os links dos releases.
- `.superpowers/` (834 MB) e `retomada-b07/` entraram no `.gitignore`.
- Revisão visual feita em 04/11 pelo portal `Telas Maestro` (10 capturas, 1280×800 e 960×640, claro e escuro). Quatro ajustes de CSS e a versão do mock em 0.5.0, registrados em `docs/contrato.md` §16.6; tabelas com `scrollWidth` igual a `clientWidth` em 1280 e 960 (portal `evaluate`).
- Capturas novas em `docs/telas/v0.5.0/` (sete telas, webp); o README mostra Dispositivos e Layout.
- Node 268/268; `conferir-unsafe.sh` e `conferir-privacidade.sh` ok; grep do critério §12.9 item 5 (estilo em linha, `innerHTML`, travessão) vazio em `ui/`.
- Portal criado novo pelo CLI fica fora da área visível do canvas e não renderiza para `screenshot`; o portal antigo `Telas Maestro` (do Maestro #3) captura normalmente.
- Guias do andar DOC aplicados a partir de `/tmp/clipsync-doc-r3-2026-11-03` (hashes 6/6, 0 links quebrados, Sentinel PASSA 5/5, Fact e Argus sem achado remanescente).

## Próximo passo
Usuário decide o commit (sugestão: um commit de docs e capturas, um da interface) e o push; depois do push, atualizar o corpo do release v0.5.0 no GitHub com a versão de links absolutos (`gh release edit v0.5.0 --notes-file`).

## Decidido (não reabrir)
- Licença continua "Todos os direitos reservados" até o usuário escolher.
- Input Leap fica até o módulo de entrada do ClipSync rodar nas máquinas reais.
- Borda por monitor e por segmento é requisito central.
- Mock e galeria de componentes ficam fora do pacote (`ui-dev`), achado S6-03.
- Incidente do teclado que não digita foi adiado em 2026-10-30.
- T4-SEG-01 (corrida no backup) fica diferido e não bloqueia release.
- Nenhum agente sobe servidor: o simulador em 5174 é do Maestro.
- Recurso sem backend permanece visível e desabilitado (regra do `ui/README.md`): a tabela de Atalhos no Linux não vira um aviso único.

## Aberto
- Tornar o repositório público: decisão do usuário. Se sim: proteger a `main` e decidir se aceita o histórico com caminhos antigos.
- Figma não reflete a v0.5.0.
- Teste físico nas duas máquinas: lista única em `docs/pendencias-v0.5.0.md`.
- O script `resumo-diario` sobrescreve este arquivo com conteúdo de outros projetos (Roblox apareceu aqui em 03/11); decidir se o arquivo sai do git ou se o script passa a escrever só sobre este projeto.
