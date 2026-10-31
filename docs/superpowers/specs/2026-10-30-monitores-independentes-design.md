# Monitores independentes: proposta de design

> Autorização posterior: João pediu em 30/10/2026 “faça tudo e faça o release, SÓ após estar tudo finalizado”. Os dois recortes apresentados estão autorizados para implementação; release condicionado à conclusão e evidências. Formulações de proposta abaixo preservam a origem das decisões.

**Estado vigente:** design aprovado por João para implementação em 30/10/2026, conforme autorização acima. Base da proposta: v0.4.0, código `6cc1384`, documentação `c1c9ea5`. Alternativas e formulações propositivas abaixo registram a decisão original; não reabrem aprovação.

## Resultado para João

Organizar cada tela separadamente no ClipSync, deixando **monitor A do desktop → notebook → monitor B do mesmo desktop**. Ao atravessar o notebook, o mouse retorna ao monitor correspondente ao lado escolhido. O desktop continua sendo um computador, com uma conexão e uma permissão. A organização dos monitores no Windows/Linux permanece intacta.

O usuário arrasta telas, ou usa controles equivalentes de posição, confere a prévia e escolhe **Aplicar organização**. Bordas e porcentagens ficam no avançado. Arrastar não muda passagens imediatamente.

## 1. Escolha e limites propostos

| Alternativa | Vantagem | Custo/limite |
| --- | --- | --- |
| **Mapa virtual por monitor, recomendado pelo autor** | Mostra diretamente A, notebook e B; deriva as ligações. | Exige posições individuais, migração e retorno por rota. |
| Manter grupos e acrescentar um assistente de ligações | Preserva mais do editor atual. | Não oferece independência visual; continua exigindo mudança do retorno. |

Primeiro recorte: **desktop Windows, dois monitores; notebook GNOME Wayland, um monitor; mouse e teclado físicos do desktop**. Inclui A → notebook → B, B → notebook → A e retorno pelo lado de entrada. Não investiga o incidente de teclado adiado.

Esse recorte não entrega a mesma jornada com o notebook como origem: seu `release_at` atual retorna ao ponto antigo, e captura/posicionamento Linux precisam de aceites separados. Tampouco habilita divisórias internas de um desktop Linux multitelas: o portal só admite barreiras externas à união das zonas. A posição sugerida no retorno pode ser ignorada pelo compositor. [Especificação InputCapture](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.InputCapture.html)

Fora dos trechos configurados, a navegação local continua pertencendo ao SO. Este mapa configura passagens entre computadores; não remapeia toda a área de trabalho nativa. A interface deve explicitar esses limites, sem transformar configuração salva em prova de funcionamento.

## 2. Telas, identidade e geometria

`layout.toml` v2 mantém dispositivos, opções, atalhos e links; acrescenta posições por `(peer_id, monitor_uid)`. Um monitor nunca vira outro peer. O inventário observado é separado das posições editáveis.

**Proposta técnica do Maestro:** cada máquina mantém `monitor-identities.toml` versionado no perfil, separado do layout. Guarda somente `monitor_uid` opaco e evidências de correspondência, sem posições, links ou grants. Após observação confiável, escreve atomicamente sob lock cooperativo, com proteção equivalente aos demais arquivos privados do perfil. Um UID nunca é remapeado a outra tela; mudanças de identidade/resolução de correspondência atualizam a revisão do inventário. O receptor pode produzir UIDs e inventário sem criar ou migrar `layout.toml`.

Nome amigável e índice são rótulos/aliases, nunca a identidade definitiva:

- Windows: correspondência exata pelo caminho de dispositivo já coletado; duplicidade ou mudança desse caminho exige reconciliação, sem escolha automática por nome.
- Linux, neste recorte: correlacionar somente quando houver um conector conectado e uma região real do compositor. Persistir conector e impressão do EDID, quando disponível, como evidência local. A coleta/correlação é trabalho novo, não capacidade já comprovada.
- Evidência insuficiente, ambiguidade, falha de leitura/escrita ou versão futura do registro: preservar os dados e o funcionamento legado; deixar o novo mapa indisponível com causa. Não fabricar UID persistente, sobrescrever o registro nem inferir identidade pelo índice.

Desconexão preserva cartão e links, inativos. Reconexão inequívoca recupera a mesma tela; ambiguidade mostra “Tela não identificada com segurança”. Caminhos e EDID não viajam na rede; apenas uid, rótulo, geometria e capacidades.

O registro não altera entrada, grants ou retorno e não integra ofertas de layout. O inventário v4 conserva a autorização de `Zones`: divulgar telas somente a pares com `input_in` concedido. Cache permite preparar rascunho; aplicar exige inventários observados nas conexões atuais.

Coordenadas nativas continuam assinadas, inclusive negativas: pixels físicos no Windows e lógicos no GNOME, conforme contrato §13.2. Dimensões virtuais usam `largura/escala` e `altura/escala` no Windows; dimensões lógicas observadas no GNOME. `x/y` virtuais são independentes: não dividir cada origem nativa pelo DPI individual.

O editor usa encaixe entre bordas e alinhamento, com alternativa acessível ao arrasto. O compilador de passagens identifica bordas opostas encostadas, calcula a interseção perpendicular e converte em frações por monitor. Rejeita sobreposição de cartões, trechos ambíguos e intervalos vazios após cantos mortos/conversão `f32`. Cálculos usam inteiros ampliados ou valores finitos limitados; canvas mantém o limite atual de ±1.000.000. Prévia e runtime consomem a mesma geometria efetiva.

Estimativa DRM não habilita aplicar a nova organização. Para obter geometria Linux, propor reutilizar uma sessão InputCapture desativada e consultar `GetZones`, sem instalar barreiras nem ativar captura. Se necessário, o usuário aceita o diálogo do sistema; recusa mantém o rascunho. Isso requer implementação e validação, sem aprovação automática de permissões.

## 3. Aplicação atômica e migração

Propor dois comandos locais: prévia de organização e aplicação. A aplicação recebe `base_rev`, revisões dos inventários observados e posições propostas; o backend recompila as ligações. Neste recorte cria/substitui somente as duas passagens desktop/notebook mostradas na prévia. Links avançados são preservados; conflito bloqueia a aplicação inteira.

Sob um único lock: revalidar revisão, identidades, capacidades e segmentos; invalidar sessões/passagens afetadas; gravar posições e links numa única revisão por arquivo temporário privado e rename. Rearmar somente após conferir novamente os inventários. Nunca executar duas gravações independentes de link.

`stale_rev` ou inventário alterado não grava nada e conserva o rascunho para revisão. Falha anterior ao rename preserva o arquivo anterior; passagens afetadas permanecem inativas até recarga validada. Falha após gravação responde **organização salva, ativação não confirmada**, sem repetir automaticamente a escrita ou alegar rollback. Se o rename ocorreu, mas a sincronização do diretório falhou, identificar também a durabilidade como não confirmada: a revisão nova já está visível, o que não equivale a confirmar persistência após uma queda do sistema.

Migração proposta:

1. Ler layout v1 e v2; versão desconhecida falha sem truncar. Abrir v1 não regrava nem recalcula links. A migração do layout só ocorre ao aplicar; observar/registrar identidades é independente e não migra o layout do receptor.
2. Ao iniciar o editor novo, posições antigas servem de semente para um rascunho; diferenças de escala exigem conferir a prévia. Links legados continuam vigentes até aplicação explícita.
3. Resolver refs antigas somente por correspondência inequívoca. Preservar refs não resolvidas e não substituir seu monitor por outro.
4. Antes da primeira gravação v2, criar backup privado exclusivo do v1; falha no backup cancela migração. Preservar opções, atalhos e ligações não substituídas.
5. Informar que v0.4.0 não lê v2. Recuperação usa o backup mediante ação explícita, sem alterar registro de monitores, identidade do computador, pares ou permissões. Nunca sobrescrever um backup anterior.

## 4. Saídas por sessão e protocolo

O v3 só informa um trecho de retorno; `Returned {session, pos}` devolve à barreira original. Não permite selecionar B após sair por A. Preservar a regra de o destino decidir a travessia exige informação adicional.

Propor protocolo v4, mínimo de conexão ainda 1, com variantes acrescentadas ao final do enum postcard; nenhuma variante antiga ganha campos. Novas mensagens abrangem inventário/capacidades, entrada com rotas, retorno por rota e oferta de layout por monitor. Nomes finais podem seguir convenções existentes, sem mudar essa semântica.

Para o primeiro recorte, cada sessão oferece **uma ou duas saídas**, com ids únicos, monitor, borda e intervalo válidos no inventário atual do receptor. O desktop conserva a tabela imutável `rota → monitor/barreira local`. Não envia ao notebook uma autorização para escolher qualquer destino.

Ao entrar no notebook pela esquerda, as saídas permitidas são esquerda → A e direita → B. Ao ultrapassar o limiar de saída, o notebook encerra a emulação, libera pressionados e envia sessão, rota e posição normalizada. O desktop valida par/conexão/sessão/rota e devolve o cursor à barreira correspondente. Monitor ausente ou ambíguo recusa entrada; não usa o fallback atual para a primeira tela.

Vincular rotas às revisões dos inventários e à geração da conexão. Descartar retorno duplicado, antigo, desconhecido ou incompatível. Trava manual continua impedindo saída. Limites existentes de zonas/links permanecem; vetores de rotas são limitados também na decodificação/validação.

Pares v1–v3 continuam conectando para funções compatíveis. A nova organização exige capacidade v4 nas duas pontas; não degradar silenciosamente A → notebook → B para retorno a A. Oferta antiga continua válida para layout legado. Copiar o novo mapa exige oferta v4 e aceitação explícita; nunca descartar posições individuais para fingir compatibilidade.

## 5. Consentimento e proteção S9

Configuração local na origem basta para aquela sessão; receber não exige aceitar layout persistente. A entrada exige `input_in` vigente concedido ao desktop. Criar cartão, mover tela ou aplicar mapa não concede permissão. Retorno apenas encerra uma sessão que a origem abriu; não constitui nova autorização de controle inverso.

Preservar §15.2–§15.3 e §15.13–§15.14: validar autoridade antes/depois de operações assíncronas; invalidar por pausa, revogação, troca de conexão, trava/UAC, alteração das zonas relevantes ou remoção de rota; liberar estado e pressionados antes de depender da rede. Eventos/aberturas atrasados não ressuscitam sessões. Filas cheias, timeout e recarga não mantêm captura antiga ativa.

Hooks permanecem condicionados ao módulo e à ligação local; eventos injetados continuam ignorados. O destino não usa recaptura de eventos emulados para atravessar. Uma única máquina mantém a captura física durante a sessão remota.

## 6. Fases e aceites propostos

1. **Contrato e apresentação:** revisar este design, prévia visual e mensagens. Aceite: João entende a sequência, origem física e limites; nenhuma borda obrigatória; confirmar o recorte inicial como decisão de produto.
2. **Identidade, arquivo e editor:** registro independente, migração recuperável e transação única. Aceite: receptor anuncia UID sem layout; falha/versão futura preserva dados e legado; monitores iguais não trocam links; `stale_rev` não produz metade do arranjo; abertura não grava layout; teclado de navegação, foco e cancelamento equivalem ao arrasto.
3. **Protocolo e sessão:** duas saídas, validação e compatibilidade. Aceite: modelos simulados cobrem ambos os retornos, versões antigas, recusa, alteração de inventário, pausa/revogação, contrapressão e sessão atrasada. Não equivalem a prova física.
4. **Validação integrada:** desktop Windows real e notebook GNOME real. Aceite: A → notebook → B e inverso, retorno pelo mesmo lado, escalas diferentes, coordenadas negativas e cantos mortos, sem reativação imediata. Conferir posição e soltura reais; estimativa relativa Linux permanece risco até essa prova. Revisão visual pelo portal e revisão independente precedem liberação.

Decisões de produto aprovadas com o recorte: origem desktop, aplicação explícita da organização, limites visíveis de plataforma e eventual diálogo de preparação no notebook. A autorização do design não substitui o consentimento do usuário em um diálogo real do sistema. Os detalhes técnicos seguem o contrato §16 e o plano de implementação.

## Base verificável

Evidências completas: [descoberta técnica](../../../.superpowers/brainstorming-pos-v0.4.0-2026-10-30/monitores-descoberta.md). Pontos centrais: `crates/clipsync-input/src/layout.rs:34`, `crates/clipsync-input/src/capture.rs:871`, `crates/clipsync-input/src/span.rs:111`, `crates/clipsync-ui/src/layout.rs:874`, `crates/clipsync-input/src/adapter/winmath.rs:150`, `crates/clipsync-input/src/adapter/portal.rs:343`, `crates/clipsync-proto/src/input.rs:110`. Contrato §§13.2, 13.6, 14.3 e 15. Nenhum código, teste, runtime ou mudança de perfil foi realizado para este documento.
