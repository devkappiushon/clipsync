# Monitores independentes

Este guia descreve a **v0.5.0, publicada no GitHub em 31/10/2026** (commit `590c137`). A v0.4.0 anterior não lê o layout v2 usado por esta organização. O número do protocolo, v4, não é a versão do aplicativo. A publicação não comprova o funcionamento nas máquinas físicas.

## 1. Entenda o percurso

O recorte é um desktop **Windows com dois monitores**, A e B, e um **notebook GNOME Wayland com uma tela**, usando o mouse e o teclado físicos conectados ao desktop:

```text
Monitor A do Windows ↔ tela do notebook ↔ monitor B do mesmo Windows
```

A organização permite configurar A → notebook → B, B → notebook → A e retorno pelo lado de entrada. Ao sair do notebook pelo lado de A ou B, a sessão usa a rota correspondente oferecida pelo desktop. Isso é o comportamento contratado, ainda sem aprovação física.

A e B continuam pertencendo ao mesmo computador, com uma conexão e suas permissões existentes. O mapa do ClipSync não muda a disposição das telas nas configurações do Windows ou do Linux. Fora dos trechos configurados, a navegação local continua sob controle do sistema operacional.

A mesma jornada com mouse físico no notebook não está coberta: o retorno posicionado Linux e a captura de divisórias internas de um desktop Linux multitelas permanecem fora deste recorte. O compositor pode ignorar a posição sugerida no retorno. Não use a cópia do mapa como prova de suporte à jornada inversa.

## 2. Prepare as duas máquinas

1. Use uma versão compatível com **protocolo v4 nas duas pontas**. Pares de protocolo v1 a v3 podem continuar conectados para funções compatíveis, mas não recebem esta organização com retorno silenciosamente reduzido à tela de entrada.
2. Pareie as máquinas e confira a disponibilidade de teclado e mouse. No notebook, em **Dispositivos**, autorize o desktop a controlar sua entrada mediante a confirmação apresentada. Criar cartões, aplicar organização, copiar ou aceitar mapa não concede essa autorização.
3. No Layout, use **Preparar telas** em cada máquina que precise atualizar suas telas. O botão atua somente na máquina em que foi acionado; preparar no desktop não prepara o notebook à distância.
4. No notebook, confirme pessoalmente eventual diálogo do portal do sistema. A preparação consulta a geometria real em uma sessão de captura desativada; não aprova o diálogo nem instala barreiras por você. Se recusar, o rascunho permanece e a aplicação pode continuar indisponível.
5. Confira se A, B e a tela do notebook estão identificados. Geometria estimada e dados em cache ajudam a exibir um rascunho, mas não habilitam aplicar: são necessários inventários observados nas conexões atuais.

O notebook pode fornecer inventário sem criar ou migrar seu layout. Receber controle autorizado também não depende de aceitar uma cópia persistente do mapa.

## 3. Organize, confira e aplique

1. Comece no desktop Windows. Se houver mais de um notebook compatível, escolha explicitamente o destino; posições e ligações de outros pares devem ser preservadas.
2. Coloque a tela do notebook entre A e B. Use arrasto, setas, campos X/Y ou controles de encaixe. Esses movimentos alteram apenas o **rascunho**. Encoste as bordas que formarão as passagens sem sobrepor cartões; diferenças de escala e altura podem limitar os trechos disponíveis.
3. Acione **Conferir prévia**. A prévia vem do agente e identifica os trechos efetivos e as ligações preservadas ou substituídas. Ajuste conflitos antes de prosseguir. Abrir o editor ou conferir a prévia não migra o layout.
4. Confira A ↔ notebook e notebook ↔ B e acione **Aplicar organização** uma vez. A aplicação usa exatamente a revisão, os inventários e as posições conferidos, revalida o conjunto e grava posições e ligações em uma única revisão.
5. Leia o resultado antes de testar. **Salvo**, **ativação confirmada** e **travessia física comprovada** são estados distintos. Mesmo uma ativação confirmada pelo agente não comprova o destino real do cursor nem a soltura física de teclas.

Se a revisão ou o inventário mudar, preserve as posições do rascunho, atualize explicitamente a prévia e confira novamente antes de aplicar. Não reaplique uma prévia antiga. Ligações avançadas conflitantes bloqueiam a aplicação inteira em vez de serem apagadas silenciosamente.

## 4. Copie e aceite sem conceder controle

Copiar envia uma oferta do mapa por monitor para pares compatíveis com v4. O destinatário confere posições e ligações e escolhe aceitar ou recusar explicitamente. Oferta enviada não significa oferta aceita, organização ativada ou permissão concedida.

Aceitar aplica o mapa apresentado e pode migrar o layout local para v2; opções e atalhos locais são preservados. Identidade, pareamento e permissões não fazem parte da oferta. O mapa pode ser salvo no receptor sem habilitar a jornada com mouse físico naquele receptor. O retorno ao desktop encerra a sessão iniciada pelo desktop; não concede controle inverso ao notebook.

## 5. Trate ausência, ambiguidade e resultado incerto

| Situação | Conduta |
| --- | --- |
| Monitor ausente | A posição salva e o UID são mantidos, mas o monitor permanece inativo, sem geometria inventada quando suas dimensões são desconhecidas. O último rótulo só está disponível se observado nesta sessão. Reconecte e prepare as telas; não substitua por outra tela só porque tem o mesmo nome. |
| “Tela não identificada com segurança” | A aplicação fica bloqueada. Confira conexão e identificação e obtenha inventário inequívoco; não há escolha automática da primeira tela. |
| Geometria estimada ou inventário ausente | Prepare as telas e obtenha observação real. Cache e estimativa não comprovam capacidade de ativação. |
| Revisão ou inventário alterado | Atualize a prévia conservando o rascunho; confira o novo resultado antes de aplicar. |
| Protocolo antigo | Use versões compatíveis com v4 nas duas máquinas. Protocolo v4 não é a versão do aplicativo. |
| Organização salva, ativação não confirmada (`unconfirmed`) | A revisão já foi publicada, mas a ativação não está comprovada. Consulte novamente o estado e o Diagnóstico antes de decidir outra ação; não repita a escrita às cegas. |
| Durabilidade não confirmada | A revisão nova pode estar visível sem confirmação de que resistirá a uma queda do sistema. Não presuma rollback nem tente sobrescrever automaticamente. |

Falha anterior à substituição atômica do arquivo preserva a versão anterior. Falha posterior pode deixar a revisão nova visível com passagens suspensas. O texto do resultado e a revisão observada devem orientar a recuperação; erro não é sinônimo de “nada foi gravado”.

## 6. Backup, recuperação e ajustes avançados

Ao migrar um layout v1 existente para v2, o agente exige um backup privado válido desse layout. Em um perfil sem layout anterior, a primeira gravação v2 não cria um backup v1. Falha no backup cancela a migração; um backup anterior não é sobrescrito. Abrir, observar e preparar telas não migram o layout. A aceitação explícita de uma oferta v4 equivale à aplicação do mapa apresentado.

**A v0.4.0 não lê layout v2.** Para voltar ao formato anterior, use a recuperação v1 em **Ajustes avançados**, com um aplicativo que ofereça essa recuperação, antes de tentar abrir o layout com a versão antiga. O controle depende de organização presente e backup válido observado; disponibilidade exibida não garante restauração. Confira a confirmação, cujo foco inicial fica em **Cancelar**, e só confirme se quiser restaurar o backup. O agente revalida backup e revisão sob exclusão antes da escrita. Se o backup estiver indisponível ou houver conflito, não fabrique um substituto nem apague o layout.

Recuperar não altera o registro de monitores, a identidade do computador, os pares ou suas permissões. O resultado ainda precisa distinguir gravação, ativação e durabilidade; recuperação também não admite repetição automática diante de incerteza.

Os ajustes avançados conservam a edição por borda. No layout v2, os segmentos efetivos geridos são salvos na aplicação. **Mudar cantos mortos vale para a próxima compilação explícita de prévia/aplicação**: não recorta novamente os segmentos já salvos nem os endpoints recebidos em uma oferta aceita. Confira e aplique uma nova organização para gerar novos segmentos com a opção vigente. Ligações avançadas mantêm validação própria.

## 7. O que ainda precisa de prova

Nenhum resultado físico é aprovado por este guia. Continuam necessários os percursos A → notebook → B, B → notebook → A e retorno pelo mesmo lado em hardware real, com posição do cursor e soltura de teclas/botões observadas; também escalas, coordenadas negativas, pausa/revogação, UAC, desconexão e permissões reais. Leitor de tela e zoom do app exigem verificação própria. O incidente anterior de teclado permanece adiado.

Os oito [cenários DEV e limites do simulador](../crates/clipsync-app/ui/README.md#cenários) ajudam a verificar a interface. Testes Node, fakes, API real com adaptadores de teste e compilação cruzada não substituem Windows e GNOME físicos. A v0.5.0 está publicada no GitHub, mas isso não comprova o funcionamento físico; os gates do [contrato §16](contrato.md#16-s10--clareza-e-monitores-independentes) seguem como referência.

Referências de implementação: [design aprovado](superpowers/specs/2026-10-30-monitores-independentes-design.md) e [plano](superpowers/plans/2026-10-30-monitores-independentes.md).
