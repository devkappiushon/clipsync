# S9 · plano de implementação de segurança

> Para agentes: executar por fatias com superpowers:subagent-driven-development, TDD e revisão independente. Autorização de execução: João, itens 2/3/4. Sem commit automático.

**Objetivo:** corrigir H-01 e B-02 sem mudar o wire nem prender o controle local por espera de rede.
**Arquitetura:** revogar entrada invalida estado antes de notificar; transporte confiável tem prazo total e reset ao cancelar; captura processa controle enquanto abertura está pendente.
**Stack:** Rust/Tokio/Quinn nas versões já fixadas.
**Spec:** docs/contrato.md §15.1–15.3 e docs/superpowers/specs/2026-10-28-s9-seguranca.md, incluindo adendo 7.

## Global Constraints
- Protocolo 3, mínimo 1, sem variantes/campos wire novos; nada v3 para pares v1/v2.
- Sem dependências, Cargo.lock, unsafe novo, perfis reais, logs de tecla/payload, servidor de dev ou consentimentos do sistema.
- Um escritor por arquivo; preservar os 16 docs do DOC. Sem commit/push/release.
- Prazo total de envio: 2.000 ms. Fila transitória da abertura: 256 eventos; overflow cancela e libera.
- Leitura de input_in só governa entrada. Revogação não transforma autorização de saída.
- Testes locais não aprovam Windows/Tauri/Wayland físicos.

## Review Focus
- Revogação entre autorização no transporte e consumo do Enter não reabre sessão.
- Reautorização, sessão antiga e par diferente não compartilham eventos ou resposta.
- Cancelamento no meio de write_all reseta prefixo sem finalizar mensagem truncada como sucesso.
- Pausa/Stop/SessionLocked durante Enter pendente recuperam controle sem aguardar timeout.
- Captura liberada sob Leave bloqueado não conserva fila antiga nem tarefa destacada ao desligar.

### Task 1: revogação e contrapressão de input

**Arquivos:** posse exata ampliada em §15.1: core transport; proto module/testing/lib; input module/capture/msg/network/lib; padrões locais em clipboard module; testes/fixtures de proto/core/input/clipboard. Preservar posse de profile.rs/UI e docs dos outros builders.
**Interfaces:** wire e InputMsg preservados; ampliar ModuleInput com contexto LOCAL e ModuleHost com snapshot vigente/envio fixado/rejeição antiga conforme spec. Manter send_reliable existente. v1/v2 conservam Frame::Rejected; Refused é v3. Não alterar adaptadores/ffi sem coordenar.

- [ ] Ler contrato e as sondas originais em ~/.cache/bastion-seg-d41aa65; não copiar arquivos do perfil real.
- [ ] Incorporar regressões com expectativas corrigidas: revogar solta tecla/botão; Enter previamente enfileirado não reabre; eventos antigos não atuam; novo Enter válido após reautorização funciona; saída local não cai por retirar input_in.
- [ ] Registrar RED dessas regressões contra a base, incluindo o motivo preciso da falha. Testar versões 1/2/3 e controles pausa/desconexão.
- [ ] Cobrir os sete grupos RED da spec: épocas durante start/enter, autoridade do host antes dos eventos, reordenação de streams, sucesso tardio cancelado, todas as causas/fronteiras rx, tarefas de saída e saturação de 32 leitores/entregas.
- [ ] Implementar épocas locais, contexto vigente, fronteiras de stream e transição de revogação antes de qualquer aviso.
- [ ] Acrescentar regressões reais de QUIC sob crédito esgotado e escrita parcial; prazo observado limitado com tolerância de agendamento, reset no cancelamento e ausência de mensagem válida parcial no receptor.
- [ ] Acrescentar host/adaptador fake com envio bloqueado: encerramento libera localmente, pausa/stop/session lock cancelam Enter pendente; fila >256 cancela, eventos antigos não reaparecem. Registrar RED antes do código correspondente.
- [ ] Implementar deadline único, reset-on-drop/erro e abertura pendente no select; avisos de encerramento limitados e desligados de liberação local. Não criar tarefa órfã.
- [ ] Rodar testes afetados de core/input e clippy dos crates; registrar comandos, contagens e limites. Build em target do repo ou cache, nunca /tmp/target.
- [ ] Produzir relatório com arquivos, RED/GREEN, decisões e dúvidas em /tmp/clipsync-s9-seguranca-report.md. Não commitar.
- [ ] Revisão independente com contrato, relatório e diff da fatia; corrigir achados importantes antes de encerrar.

## Validação integrada (janela única depois das demais fatias)
- [ ] cargo fmt --all --check; scripts/conferir-unsafe.sh; scripts/conferir-privacidade.sh.
- [ ] cargo test --workspace --exclude clipsync-app --locked e clippy equivalente com -D warnings.
- [ ] Cross Windows de input/win/ui/cli com toolchain existente; não declarar execução de testes Windows por compilação.
- [ ] Registrar as provas físicas faltantes e entregar roteiro ao andar TESTES.

## Registro
- Base: d41aa65; docs de Quill preservados, entrega encerrada.
- Decisão do Maestro: iniciar por H-01/B-02, pois liberação local e autorização são pré-requisitos para validar o fluxo de mouse.
- Decisão do Maestro: 2.000 ms total e 256 eventos pendentes limitam espera/memória; excesso falha liberando controle local.
- Estado: ressalvas Sentinel incorporadas (época local, ordem de streams, rx, donos e orçamento de leitura/entrega). Implementação liberada com TDD; diff ainda passará por revisão independente.
