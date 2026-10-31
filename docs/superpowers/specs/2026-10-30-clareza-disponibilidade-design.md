# Clareza da disponibilidade · proposta pós-v0.4.0

> Autorização posterior: João pediu em 30/10/2026 “faça tudo e faça o release, SÓ após estar tudo finalizado”. Os dois recortes apresentados estão autorizados para implementação; release condicionado à conclusão e evidências. Formulações de proposta abaixo preservam a origem das decisões.

Data: 30/10/2026. Estado vigente: design aprovado por João para implementação, conforme autorização acima; validação e release seguem os gates do plano. A formulação original da proposta é preservada abaixo.

## Resultado para o usuário
Explicar por que um controle está desabilitado, sem confundir uma função ainda ausente com módulo desligado, estado não confirmado ou autorização feita no outro computador. A versão instalada mais recente continua podendo conter opções futuras.

## Abordagem recomendada e alternativas
Usar o estado já fornecido pelo app e corrigir a apresentação nas telas existentes. Ocultar todos os controles futuros prejudicaria a descoberta e contrariaria o contrato atual. Implementar todos os recursos ausentes ampliaria a rodada para telemetria, atalhos e modos de clipboard, sem resolver primeiro as explicações incorretas. A recomendação do Maestro e do UX foi incluída nos dois recortes autorizados por João.

## Comportamento delimitado
1. **Dispositivos e Pareamento:** dentro de um ramo já desabilitado, explicar suporte explicitamente ausente, módulo local explicitamente desligado ou estado não confirmado, nessa ordem. Bloqueio de confiança e etapa do pareamento mantêm precedência. Só orientar Módulos em Preferências quando o módulo estiver comprovadamente desligado.
2. **Recursos remotos:** usar “Não foi possível confirmar os recursos de {nome}” quando o booleano recebido não prova observação atual. Não atribuir ao par a causa de uma disponibilidade agregada dos dois lados. Conexão e versão isoladas não demonstram o estado atual do módulo remoto.
3. **Autorização no destino:** substituir o interruptor sempre falso “Controlar {nome} a partir deste computador” por orientação estática no mesmo grupo: “Para controlar {nome} a partir deste computador, autorize este computador na tela Dispositivos de {nome}. O ClipSync não informa aqui se essa autorização já foi concedida.” O interruptor de conceder controle **aqui**, seu valor e confirmação permanecem.
4. **Clipboard:** módulo desligado continua marcado “desligado”; sua célula Plataforma informa “não avaliada com o módulo desligado”, em tom neutro. Estado não confirmado permanece desconhecido. Com módulo explicitamente ligado, apresentar a classificação recebida, sem afirmar que houve validação física.
5. **Diagnóstico:** remover a promessa de latência após parear. Mostrar estado e endereço disponível; explicar que medição de latência por computador ainda não foi implementada.

Textos completos, fontes e revisão UX: `.superpowers/brainstorming-pos-v0.4.0-2026-10-30/ux-proposta-continuidade.md` (APROVA COM RESSALVAS na proposta, sem aceite visual).

## Limites e decisões de implementação
- Nenhuma regra de habilitação, grant, confirmação, comando de gravação, API Rust, protocolo ou dependência muda. As variáveis de motivo participam do gate: conservar seus ramos nulos/não nulos.
- Preservar componentes, foco, anúncios, tokens, segurança de texto e recursos futuros visíveis/desabilitados. Não introduzir HTML de nomes recebidos.
- Registrar o adendo visual no contrato antes da implementação. Um único implementador terá as quatro telas e, se útil, um helper de apresentação; um revisor independente confere o delta.
- Manter mapa de monitores e investigação do teclado fora desta fatia. Não alterar instalação/perfil real, nem publicar automaticamente.

## Aceite
- Casos de apresentação exercitam os ramos de suporte, módulo, desconhecido, remoto sem observação, confiança bloqueada e etapas do pareamento; os mesmos controles continuam habilitados/desabilitados.
- A orientação remota não tem interruptor falso nem chama escrita; concessão local preserva confirmação e conteúdo enviado.
- Clipboard desligado não é descrito como incompatibilidade de plataforma; Diagnóstico vazio não promete latência.
- Testes focados de comportamento e regressões dos consumidores; revisão de qualidade e segurança do consentimento; inspeção visual em temas claro/escuro, 1280 e 960, por portal válido.
- Captura parcial, simulador e aprovação de código não provam aplicativo nativo, fala assistiva ou travessia física.
