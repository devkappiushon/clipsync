# Pendências de validação da v0.4.0

**Código revisado e teste automatizado não significam mouse físico validado.** Esta lista deve continuar aberta até alguém registrar o resultado real, com sistema, versão e cenário. Não marque um item como concluído apenas porque o CI passou.

| Pendente | Como comprovar |
|---|---|
| Travessia Windows → Linux e Linux → Windows | Mesma 0.4.0 instalada; atravessar e voltar em cada posição configurada, registrar entrada proporcional e eventual salto ao centro. |
| Teclado, roda e soltura | Testar Shift/tecla pressionada ao atravessar, Pausar, bloquear origem/destino e desconectar. Nenhuma tecla pode permanecer pressionada. |
| Ambas as máquinas autorizando entrada | Conferir as direções de saída configuradas; verificar controle e retorno sem sessões sobrepostas inesperadas. |
| Permissões e plataformas reais | GNOME/Wayland e portais do sistema; Windows/UAC, desktop seguro, atalhos, notificações e mudança de DPI/monitor. |
| App Tauri interativo | Abrir os instaladores publicados e conferir janela, bandeja e avisos; build/teste no runner não prova interação no desktop. |
| Acessibilidade assistiva | Tab/Shift+Tab/Enter/Esc e foco visível no app, zoom/reflow real, Orca/NVDA e anúncio único. |
| Lacunas da coleta em portal | Foco em largura 960; Conectar → erro → Corrigir endereço; segunda observação independente. O mock não alcança a composição de duas falhas de consulta nem todos os casos de cancelamento. |

Roteiro completo: [contrato §13.12](contrato.md#1312-roteiro-manual-do-usuário-pc-windows-e-notebook). Se a âncora mudar, procure a seção 13.12 no documento. A atualização preserva o perfil: não apague config/chaves/pares para testar.

## Evidência disponível e limites

- Validação local: 597 testes Rust principais aprovados, cinco execuções auxiliares de processos aprovadas, 164 Node; três testes gráficos ignorados.
- Revisão do delta de integração: aprovada. [CI nativo Linux/Windows e publicação](evidencias/release-v0.4.0.md) concluídos no commit `6cc1384`: 624/614 testes principais aprovados, respectivamente, incluindo o app Tauri. Build/testes no runner não comprovam interação física.
- Geometria: 60 casos medidos no simulador e revisados por artefatos. Foco/semântica: cinco casos com aceite completo no recorte DOM e seis inconclusivos. isTrusted=false/hasFocus=false não comprovam teclado ou foco nativos.
- Não existe certificação WCAG, legal ou de uso físico. Não há decisão nova de licença, suporte a terceiro computador ou remoção de perfil/desinstalação nesta rodada.

Para reportar o teste: informe versão das duas instalações, origem/destino, posição no Layout, ação feita e resultado observado. Use texto fictício ao testar o clipboard; nunca envie segredo ou conteúdo pessoal no diagnóstico.
