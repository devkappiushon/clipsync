# Contexto — clipsync

**Atualizado:** 2026-11-04 (sessão do Maestro · scaffold)

## Onde estou
Plano "diagnóstico verificável por log" (`docs/superpowers/plans/2026-11-04-diagnostico-por-log.md`, contrato §17) em execução para o usuário conseguir, com a IA, ler nos logs das duas máquinas onde a travessia do mouse para. Fatias 1, 2a, 2b, 3 e 5 commitadas na `main` local; fatia 4 (pacote de suporte e tela Diagnóstico) com o Rivet. Tudo sem push: aguarda o `push` do usuário para enviar a `main` e disparar o empacotamento manual (artefatos, sem release).

## Estado real (verificado)
- Commits locais desde `f179b4f`: `f7cb334`, `0db8a2d`, `9258ed6`, `65ca7c6`, `d1136b7`, `89aba99`, `ba0fcbd`, `239b715`, `f5e55aa`, `188b10d`, `954ff90` (`git log --oneline f179b4f..HEAD`).
- Input: 20 binários de teste verdes, `tests/s11_log.rs` 18/18; clippy e fmt limpos (`cargo test -p clipsync-input --locked`). Core, proto, CLI e UI: 397 testes verdes na T2 (Rivet), CLI 52/52 na T9; `cargo fmt --all -- --check` ok no workspace.
- `./target/debug/clipsync verificar-travessia` reproduz o caso do usuário com logs sintéticos: quebra em `rotas` (`no_remote_inventory`), parada em `inventario_destino`, código 1 (amostra em `docs/verificar-por-log.md`).
- Revisões: Sentinel PASSA T1 (2 Minor absorvidos) e T2-bis (`ba0fcbd`); Argus reprovou T1, T2, T3, T4, T5+T6 com achados tratados em `0db8a2d`, `ba0fcbd`, `f5e55aa` e `188b10d`; re-revisões de `188b10d`, `239b715` e `f5e55aa` pendentes em `docs/verdicts.md`.
- Time: Vector, Keel, Bolt, Flux e Pixel sem cota (OpenCode gratuito); Rivet e Argus (Codex) e Sentinel ativos. O Maestro fez as fatias do input e as decisões de plano; F4 passou de Pixel para Rivet.
- `portal.rs` (EventSink): reprovado pelo Argus com 3 Important; guardado no ramo `wip/portal-eis-link` (`505bcaa`), fora da `main`.
- Simulador em 5174 no ar para a F4 do Rivet (`curl` 200); derrubar ao fim.
- Repositório público desde 04/11; `main` protegida contra force-push e deleção; v0.5.0 com notas de links absolutos; v0.5.1 não publicada.

## Próximo passo
Com o `push` do usuário: `git push origin main`, depois `gh workflow run release.yml --ref main` para gerar os artefatos; o usuário instala nas duas máquinas, tenta a travessia, copia `logs/clipsync.jsonl` de cada uma e roda o verificador (`docs/verificar-por-log.md`). Em paralelo: fechar F4 (Rivet) e as re-revisões do Argus; só então a tag v0.5.1.

## Decidido (não reabrir)
- Licença continua "Todos os direitos reservados" até o usuário escolher.
- Notebook é só destino; o PC Windows é a origem com o mouse físico.
- `[falhou]` só com prova explícita; ausência é `[sem dado]` e vira `parada`; códigos 0/1/2/3 conforme §17.3.
- Sem crate novo; escritor único fechado com o agente; conteúdo, tecla e keycode nunca entram no log.
- Pareceres do Argus ficam em `docs/verdicts.md` (o `ask` dele não alcança o Maestro).
- `hooks.installed/dropped` do Windows e `ignorados` por sessão ficaram fora desta rodada (plano, seção Decisões).

## Aberto
- Prova física nas duas máquinas: depende dos artefatos e do usuário.
- O script `resumo-diario` continua sobrescrevendo este arquivo (aconteceu de novo em 04/11 07:34); decidir se sai do git ou se o script muda.
- Figma não reflete a v0.5.0; `pendencias-v0.5.0.md` §1 segue aberta.
