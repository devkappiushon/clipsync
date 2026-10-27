# Prompt mestre — ClipSync: área de transferência compartilhada entre computadores

> Este documento é um prompt de desenvolvimento: os recursos descritos são requisitos e propostas, não funcionalidades já implementadas nem garantias de compatibilidade.

## 1. Seu papel e o resultado esperado

Atue como uma equipe integrada de engenharia de software, arquitetura de sistemas distribuídos, segurança, desenvolvimento desktop, experiência do usuário, acessibilidade, testes e distribuição. Projete e implemente um aplicativo real para compartilhar a área de transferência entre meus computadores. Não entregue apenas uma interface bonita, uma página de apresentação, um protótipo com dados fictícios ou uma lista de recomendações.

Quero uma solução executável, documentada, testável e progressiva. Faça primeiro uma versão pequena que funcione de ponta a ponta, mantendo uma arquitetura que permita incorporar os recursos avançados deste documento. Diferencie claramente o que foi implementado, o que foi testado, o que depende de validação em outro sistema e o que pertence apenas ao planejamento.

Responda em português do Brasil. Use nomes de código consistentes, preferencialmente em inglês, e mensagens da interface em português, com estrutura preparada para tradução. Explique decisões técnicas relevantes sem transformar cada escolha em uma discussão interminável.

Não invente bibliotecas, APIs, parâmetros de configuração, resultados de testes ou garantias de segurança. Consulte documentação oficial atual quando houver ferramentas de pesquisa disponíveis. Sem acesso a uma plataforma, registre a limitação e forneça testes reproduzíveis; não declare suporte comprovado apenas porque o código compila.

## 2. Meu contexto e as premissas iniciais

Uso Input Leap em um notebook, um PC pessoal e um PC de casa. Quero continuar usando o Input Leap para teclado e mouse, mas ter um aplicativo dedicado à área de transferência. A experiência principal deve ser: copiar em qualquer uma das três máquinas e colar nas outras máquinas autorizadas, sem precisar enviar mensagens para mim mesmo, criar arquivos temporários ou abrir uma página web.

Os sistemas operacionais, suas versões e a topologia da rede ainda não foram informados. Não assuma que todos os computadores usam Windows, que estão sempre ligados ou que estão sempre na mesma rede. Registre essas informações como premissas editáveis:

```text
Nome do aplicativo: ClipSync
Dispositivos iniciais: Notebook, PC pessoal, PC de casa
Sistemas operacionais: a confirmar
Cenário inicial: rede local autorizada
Cenário adicional: redes diferentes, mediante configuração explícita
Quantidade inicial: 3 dispositivos
Expansão desejada: pequenos grupos de dispositivos pessoais
Idioma inicial: português do Brasil
Conta em serviço externo: não obrigatória
Servidor central permanente: não obrigatório
Sincronização de teclado e mouse: responsabilidade do Input Leap
```

Se faltarem informações, escolha padrões conservadores e declare as suposições. Não transforme a ausência de respostas em motivo para não avançar. Priorize a plataforma disponível no ambiente de desenvolvimento, mas preserve adaptadores separados para as demais. Não acrescente Android, iOS, servidores comerciais ou inteligência artificial ao escopo inicial por conta própria.

## 3. Objetivo do produto e limites de escopo

O produto deve oferecer sincronização bidirecional ou multidirecional, identificação clara dos dispositivos, pareamento seguro, controle de envio e recebimento, prevenção de ciclos, recuperação após desconexões e funcionamento discreto em segundo plano durante a sessão do usuário.

Independência significa que ele deve funcionar sem o Input Leap aberto e sem depender de qual computador está recebendo teclado e mouse. A integração com o Input Leap deve ser uma conveniência opcional, nunca um requisito para o mecanismo básico.

Não crie um novo software de controle remoto. Não inclua captura de tela, gravação de teclas, monitoramento oculto, acesso administrativo permanente, abertura remota de programas ou execução de comandos recebidos. O aplicativo deve operar de forma visível e consentida nas minhas máquinas e nas sessões autorizadas.

Não confunda sincronizar com colar automaticamente. Por padrão, o aplicativo prepara o clipboard do destino; eu continuo decidindo onde e quando pressionar o comando de colar. Uma funcionalidade futura de colagem assistida precisa de consentimento próprio, avaliação de segurança e indicação das permissões necessárias.

## 4. Priorização e evolução do projeto

Organize o trabalho em quatro níveis. O primeiro é uma prova técnica dos riscos: observar e escrever o clipboard na plataforma escolhida, estabelecer comunicação autenticada entre dois processos e comprovar que não há ciclo ao aplicar conteúdo recebido.

O segundo é o MVP utilizável: texto Unicode entre três computadores, pareamento, comunicação criptografada, identificação dos pares, envio automático ou manual, pausa, controles de direção, reconexão, diagnóstico, persistência segura das configurações e prevenção de repetição indevida. A versão inicial pode começar com endereço manual; descoberta automática não deve atrasar o fluxo principal.

O terceiro acrescenta histórico local opcional e protegido, pesquisa, favoritos, imagens, formatos enriquecidos, descoberta automática, regras mais detalhadas e melhor experiência de instalação. Histórico não deve aparecer como implementado enquanto existir apenas uma lista em memória sem a política de retenção prometida.

O quarto reúne recursos avançados: transferência real de arquivos, acesso entre redes, relay auto-hospedado, grupos mais complexos, sincronização opcional de histórico, integrações e clientes móveis. Separe ainda ideias experimentais que não são compromissos de entrega.

Todo requisito deve ter prioridade, dependências, critério de aceite e situação. Não implemente todos os níveis de uma vez nem remova requisitos essenciais de segurança para fazer uma demonstração mais rápida.

## 5. Convivência com o Input Leap

Verifique os recursos e a configuração da versão efetivamente instalada do Input Leap antes de orientar qualquer alteração. O projeto deve considerar que outro software pode observar e modificar o mesmo clipboard. Não invente uma API oficial de integração, um evento de troca de tela ou uma opção de configuração sem comprovação.

O modo recomendado de convivência deve manter o Input Leap responsável por teclado e mouse e deixar apenas um aplicativo responsável pela sincronização automática do clipboard. Documente como desativar somente o compartilhamento de clipboard do Input Leap quando a versão utilizada permitir, preservando sua configuração original e explicando como reverter.

Não edite arquivos do Input Leap, encerre processos ou altere opções sem autorização. Não desative a criptografia de nenhum dos aplicativos como solução para problemas de comunicação.

Considere também conflitos com gerenciadores de clipboard, histórico e sincronização do próprio sistema, ferramentas de acesso remoto e outros aplicativos semelhantes. Ofereça uma verificação assistida de conflito e um modo conservador de recebimento manual. A prevenção interna de ciclos não deve ser apresentada como garantia contra todos os programas externos.

Não acople o produto ao ciclo de manutenção do Input Leap. Caso uma futura integração para enviar apenas à máquina ativa dependa de interface não documentada, marque-a como experimental e preserve a seleção manual de destinos.

## 6. Semântica: o que exatamente será sincronizado

Defina separadamente quatro conceitos: clipboard ativo do sistema, histórico local do aplicativo, caixa de itens recebidos e fila de transmissão. Um item pode existir no histórico sem ser enviado, ser recebido sem substituir o clipboard e ficar pendente sem ser considerado entregue.

Disponibilize modo automático para pares autorizados, modo de envio manual, modo de recebimento com confirmação e configurações por direção. O usuário deve poder deixar um computador apenas enviando, apenas recebendo, completamente pausado ou participando somente de um grupo específico.

No primeiro uso, explique o alcance da sincronização antes de ativá-la. Não transmita o conteúdo que já estava no clipboard ao instalar, iniciar, parear ou retomar o aplicativo, salvo ação explícita ou política habilitada conscientemente. Uma máquina recém-pareada não recebe automaticamente todo o histórico antigo.

Diferencie pausa de rede, pausa de captura, pausa de histórico e modo privado. O comando principal de privacidade deve interromper captura, persistência e envio, e impedir que conteúdo recebido seja aplicado durante a pausa. Mostre quando uma fila foi descartada e não a publique automaticamente ao retomar.

## 7. Adaptação aos sistemas operacionais

Crie uma interface de clipboard com operações para descobrir capacidades, observar mudanças, ler uma fotografia consistente dos formatos disponíveis, escrever representações compatíveis e acompanhar alterações geradas pelo próprio aplicativo. Separe essa interface do transporte, da interface gráfica e do banco de dados.

No Windows, avalie as APIs nativas de notificação e sequência do clipboard, a concorrência com outros programas, dados fornecidos sob demanda, bloqueios temporários e as diferenças de formatos. O agente deve funcionar na sessão interativa do usuário. Não presuma que um serviço de sistema tem acesso ao clipboard de todas as sessões.

No macOS, avalie NSPasteboard, changeCount e as regras de acesso aplicáveis às versões suportadas. Não solicite permissões de acessibilidade ou captura de tela sem uma funcionalidade que realmente as exija. Se for necessário consultar mudanças periodicamente, use uma estratégia adaptativa e mensure o impacto em bateria e latência.

No Linux, trate X11 e Wayland como ambientes distintos. Diferencie CLIPBOARD e PRIMARY; a seleção PRIMARY não deve ser sincronizada por padrão. Considere propriedade dos dados, transferência sob demanda, encerramento da aplicação de origem, múltiplos formatos e ambientes com mais de uma sessão gráfica.

No Wayland, verifique compositor, protocolos expostos, portais, versão do backend e permissões. Não prometa observação global irrestrita em todos os ambientes. O portal de clipboard deve ser avaliado dentro das sessões e restrições que ele realmente suporta, não como uma autorização universal independente.

Quando a observação automática não estiver disponível, ofereça operação manual permitida pelo sistema e diagnóstico explicativo. Não tente contornar a segurança do compositor. Classifique cada combinação de plataforma e recurso como validada, experimental, indisponível ou ainda não testada.

## 8. Captura consistente e aplicação segura

Capture uma fotografia coerente do clipboard, evitando misturar texto de uma cópia com imagem de outra. Consulte a geração ou sequência nativa antes e depois da leitura quando possível, descarte fotografias inconsistentes e limite tentativas de leitura.

Use eventos nativos quando disponíveis. Não trate a existência de uma função readText como prova de que há monitoramento confiável em segundo plano. Quando a plataforma só permitir sondagem, documente que mudanças muito rápidas entre leituras podem não ser observadas.

Agrupe formatos pertencentes à mesma cópia sem eliminar cópias distintas por um debounce excessivo. Defina como lidar com conteúdo vazio, formato indisponível, bloqueio por outro programa e aplicação de origem que deixa de fornecer dados.

Ao receber, valide e prepare os dados antes de modificar o clipboard. Reavalie a política e a geração local imediatamente antes de aplicar. Se o usuário tiver copiado outro conteúdo enquanto uma imagem ou um arquivo era recebido, não sobrescreva silenciosamente essa cópia mais recente.

Respeite as limitações de atomicidade de cada API. Uma falha de escrita precisa ser identificável, não pode gerar uma sequência infinita de reaplicações e não deve ser reportada como sucesso.

## 9. Tipos de conteúdo e fidelidade

O MVP deve suportar texto Unicode, acentos, emojis, caracteres combinantes, idiomas diferentes, múltiplas linhas, tabulações, espaços significativos, URLs e trechos de código. Preserve o conteúdo; não remova espaços, altere quebras de linha ou normalize texto silenciosamente.

Na evolução, modele uma mesma cópia com múltiplas representações: texto simples, HTML, RTF e imagens compatíveis. Negocie os formatos que cada destino consegue receber. Quando houver perda de formatação, ofereça um fallback explícito e mostre a limitação.

Para imagens, preserve transparência e qualidade dentro do formato escolhido. Imponha limites tanto ao tamanho comprimido quanto às dimensões e ao consumo após decodificação. Não converta silenciosamente todas as imagens para um formato com perdas. Separe miniatura, representação de exibição e conteúdo transferido.

Teste conteúdo vindo de navegadores, editores de texto, terminais, ferramentas de desenvolvimento e planilhas. Fórmulas, macros, links e texto recebido nunca devem ser executados automaticamente pelo aplicativo.

Formatos privados e binários desconhecidos ficam fora da sincronização por padrão. Não transmita todos os formatos apenas porque estão disponíveis: eles podem carregar dados extras, referências locais ou conteúdo incompatível. Use uma lista explícita de formatos suportados e documente conversões.

## 10. Dispositivos, grupos e regras de autorização

Cada dispositivo deve ter identidade criptográfica própria e um nome amigável editável. O nome não é prova de identidade. Renomear uma máquina não deve revogar suas permissões; uma máquina diferente com o mesmo nome não deve herdar confiança.

Modele permissões por par e por direção, incluindo envio, recebimento, formatos, limites e aplicação automática. A política efetiva deve respeitar simultaneamente as restrições do remetente e do destinatário.

Prepare grupos como “mesa”, “pessoal” e “temporário”, sem impor esses nomes. Entrar em um grupo exige autorização. Parear A com B e B com C não autoriza automaticamente A a enviar para C nem B a repassar dados entre eles.

Não use o notebook como ponte automática entre ambientes que o usuário pretendia manter separados. Reencaminhamento, quando existir, deve preservar origem autenticável e respeitar uma política explícita. No MVP, prefira transmissão direta aos destinatários autorizados, sem roteamento por outros pares.

A interface deve mostrar quem pode enviar, quem pode receber e quais restrições estão ativas. Ofereça revogação, bloqueio temporário, remoção e novo pareamento, diferenciando essas ações.

## 11. Descoberta e pareamento seguro

Ofereça descoberta local opcional e conexão por endereço configurado manualmente. Avalie mDNS/DNS-SD ou mecanismo equivalente, com comportamento explícito quando multicast estiver bloqueado. Não trate descobrir um dispositivo na rede como autorização para acessá-lo.

Mostre nome, identidade verificável, estado e solicitação de pareamento. Exija confirmação nas duas pontas, com comparação de código de autenticação, impressão digital ou fluxo equivalente baseado em protocolo estabelecido. A confirmação precisa estar vinculada às chaves e à sessão de pareamento, não apenas à igualdade de nomes.

Um código curto não deve funcionar como chave estática de criptografia. Caso seja usado um segredo de baixa entropia, escolha um protocolo de pareamento adequado e uma implementação consolidada, com limites de tentativas e proteção contra adivinhação. Não projete um protocolo criptográfico novo.

Permita convite de uso único, com expiração curta, cancelamento e escopo definido. QR code e importação manual podem ser opções futuras; não torne uma câmera obrigatória para parear computadores. O próprio convite deve ser tratado como informação sensível e não entrar no histórico sincronizado.

Depois do pareamento, autentique os pares novamente em cada conexão usando as identidades autorizadas. Mudança de chave deve gerar alerta e exigir verificação, salvo uma rotação autenticada prevista no protocolo. Reinstalações, cópias de perfis e clones de máquinas precisam de comportamento documentado.

## 12. Rede local, endereçamento e conectividade

Comece com conexões diretas entre os três computadores e sem exigir um servidor central sempre ligado. Escolha um único transporte inicial, confiável e bem suportado, justificando a decisão. Não implemente simultaneamente múltiplas tecnologias de transporte sem necessidade demonstrada.

Considere IPv4, IPv6, mudanças de IP, Wi-Fi e Ethernet, várias interfaces, suspensão do notebook, troca de rede, descoberta bloqueada, firewall e isolamento entre clientes do roteador. Permita escolher interfaces e endereços de escuta. Documente portas e regras mínimas necessárias.

O padrão deve limitar a exposição às interfaces e redes autorizadas. Não abra portas no roteador, não ative UPnP e não crie regras amplas de firewall silenciosamente. Em uma rede nova, adote postura conservadora e explique o estado de descoberta e sincronização.

Use conexão autenticada também em rede doméstica. Endereço privado, nome de rede e proximidade não substituem autenticação. Não envie conteúdo ou hashes do conteúdo por anúncios de descoberta.

Se dois pares tentarem conectar simultaneamente, resolva conexões duplicadas de forma determinística. Use limites de conexões, backoff com aleatoriedade, cancelamento e controle de consumo. A perda de um par não pode interromper a comunicação entre os demais.

## 13. Comunicação entre redes diferentes

Mantenha acesso remoto fora da dependência obrigatória do MVP. Como primeiro caminho opcional, documente operação sobre uma rede privada virtual já autorizada pelo usuário, sem pressupor que descoberta multicast atravesse essa rede.

Se houver conexão direta pela internet, pesquisa de endereços ou relay, detalhe NAT, restrições de firewall, disponibilidade, custos operacionais, política de metadados e situações em que a conexão não será possível. Não prometa conexão universal em qualquer rede.

Um relay deve ser opcional, poder ser auto-hospedado e não ter acesso ao conteúdo em claro. Autenticar a conexão entre cliente e relay não equivale, por si só, a criptografia de ponta a ponta entre os computadores. Preserve autenticação dos dispositivos e confidencialidade ponta a ponta no desenho completo.

Defina se o relay apenas encaminha tráfego ou também armazena mensagens. Armazenamento temporário requer prazo de expiração, quotas, exclusão, tratamento de dispositivos revogados e política de entrega atrasada. Documente os metadados inevitavelmente visíveis ao operador.

Não inclua infraestrutura de nuvem, conta externa ou custo mensal como requisito escondido. O modo exclusivamente local precisa continuar funcionando sem internet, sem consulta a serviço de licença e sem resolução de serviços externos.

## 14. Protocolo de aplicação

Especifique protocolo versionado, negociação de capacidades e limites negociados. Modele identificação e capacidades, oferta de conteúdo, solicitação de representação, transmissão, confirmação, cancelamento, rejeição e erro. Não confunda especificar esses conceitos com criar uma mensagem diferente para cada operação do sistema operacional.

Um evento deve conter, conforme necessário, identificador único, origem autenticada, identificação da sessão ou época, sequência, informação causal, tipo de conteúdo, representações, tamanhos, destinatários ou escopo, expiração e referência de integridade. Defina quais campos são obrigatórios, quais são opcionais e quais podem ser adicionados futuramente.

Não aceite um campo de origem arbitrário como prova de autoria. Em conexão direta, a origem deve corresponder ao par autenticado; em encaminhamento futuro, a autenticidade original precisa ser preservada por mecanismo definido e revisado.

Metadados de conteúdo, nomes de arquivo e verificadores de integridade devem viajar dentro do canal ou envelope protegido. Um hash de texto curto não é proteção de privacidade e pode permitir tentativa de adivinhação se exposto.

Defina enquadramento das mensagens, tamanho máximo de cabeçalho e corpo, limites de campos, validação de números, UTF-8, tipos MIME e comportamento diante de versão incompatível. Não aloque memória com base irrestrita em um tamanho enviado pelo outro lado.

Confirmações devem distinguir recebido, validado, armazenado, aplicado ao clipboard, rejeitado pela política, expirado e cancelado. Não mostre “pronto para colar” quando somente a transmissão de rede terminou. Explicite quais estados não existem em uma plataforma ou modo sem histórico.

## 15. Deduplicação, prevenção de ciclos e repetição legítima

Implemente o fluxo de modo que um evento recebido e aplicado não seja capturado como uma nova cópia local e reenviado indefinidamente. Combine identidade do evento, sequência ou geração local, registro de operações de escrita e, quando necessário, impressão do conteúdo dentro de uma janela controlada.

Não use apenas “o texto é igual ao último” como regra global. Copiar deliberadamente o mesmo texto novamente pode ser uma nova ação válida; um mesmo conteúdo pode ter representações diferentes e permissões distintas. A deduplicação de armazenamento não deve apagar a semântica dos eventos.

Use marcação privada de origem no clipboard somente quando a plataforma permitir e sem tratá-la como fronteira de segurança: outro aplicativo pode removê-la, modificá-la ou imitá-la. Mantenha proteção interna independente dessa marca.

Defina validade e limites de memória para eventos já vistos, tratamento após reinício e proteção contra replays antigos. Se houver reencaminhamento futuro, acrescente controle de saltos e rota sem transformá-lo no único mecanismo antirrepetição.

Teste explicitamente A → B → C → A, duas conexões entre os mesmos pares, entregas duplicadas, retransmissão depois de uma queda, mudança de representação por outro gerenciador e cópia local idêntica ao conteúdo recebido.

## 16. Concorrência, conflitos e ordenação

Defina uma política verificável para duas pessoas ou processos copiarem conteúdos diferentes em máquinas diferentes quase ao mesmo tempo. Não declare “o mais recente vence” sem explicar como a ordem é estabelecida e o que significa quando os relógios estão incorretos ou os dispositivos estavam desconectados.

Escolha uma estratégia causal ou lógica adequada ao pequeno conjunto de dispositivos, com desempate determinístico quando necessário. Explique suas propriedades, limites e proteção contra um par que envie contadores ou horários absurdos. Horário de parede pode ser usado para exibição, mas não deve ser a única base da correção.

Separe ordenação dos eventos da decisão de aplicá-los ao clipboard. Convergência do registro lógico não exige sobrescrever uma cópia local recente quando o modo exige confirmação ou proteção da atividade do usuário.

Mantenha alternativas recuperáveis quando o histórico estiver habilitado; sem histórico, respeite a política de memória temporária e não prometa recuperar conteúdo que foi descartado. Apresente conflitos relevantes sem abrir uma notificação a cada evento comum.

A máquina de estados deve impedir que uma transferência antiga, concluída lentamente, ganhe prioridade apenas porque terminou depois. Uma ação explícita do usuário para reaplicar um item pode gerar um novo evento, com nova autorização e identidade.

## 17. Desconexão, suspensão e recuperação

No MVP, não acumule silenciosamente um histórico persistente de tudo que deveria ter sido enviado a uma máquina desligada. Defina claramente uma política limitada para destinos offline; prefira descartar eventos ao vivo não entregues ou mantê-los apenas pelo período curto explicitamente configurado.

Ao reconectar, não reproduza uma sequência inteira de cópias antigas sobre o clipboard atual. Ofereça solicitar o último item elegível ou consultar a caixa de recebidos, dependendo do modo escolhido. Nunca republique automaticamente conteúdo expirado, privado ou capturado durante uma pausa.

Filas persistentes futuras devem ser criptografadas, limitadas por quantidade, bytes e idade, e reavaliadas depois de qualquer mudança de política ou revogação. Uma fila não pode ser usada para contornar uma permissão que deixou de existir.

Após suspensão, logout, bloqueio de sessão, hibernação, reinício do serviço ou troca de rede, revalide conectividade, identidade e permissões. O comportamento durante a tela bloqueada deve ser conservador, configurável e compatível com os sinais realmente fornecidos pelo sistema.

Garanta encerramento organizado, gravações transacionais e retomada sem duplicar operações. Uma falha de banco, disco cheio ou falta de permissão precisa produzir um estado seguro e visível, não um fallback para arquivos de conteúdo em claro.

## 18. Modelo de ameaças

Produza um modelo de ameaças com ativos, adversários, fronteiras de confiança, hipóteses, impactos e mitigações. Inclua atacante na rede local, dispositivo desconhecido, dispositivo autorizado que se torna malicioso, máquina perdida, relay comprometido, biblioteca vulnerável, conteúdo malformado e acesso indevido ao perfil local.

Considere interceptação, adulteração, falsificação de dispositivo, pareamento enganoso, replay, downgrade, vazamento por notificações e logs, abuso de recursos, arquivos maliciosos e substituição indesejada do clipboard. Diferencie confidencialidade do transporte, proteção de armazenamento e confiança em cada endpoint.

Não prometa proteger conteúdo de um sistema operacional comprometido nem impedir que um aplicativo com acesso legítimo ao clipboard leia o que foi colocado nele. Criptografia de transporte não torna o clipboard do destino privado em relação a todos os programas locais.

O conteúdo recebido deve ser tratado como entrada não confiável, inclusive quando veio de uma máquina pareada. Pareamento autoriza operações específicas, mas não remove validação, limites ou necessidade de consentimento para ações sensíveis.

Adote menor privilégio e segurança por padrão. Não exija administrador ou root para o uso normal. Recursos que realmente dependam de privilégios adicionais devem ser isolados, justificados e dispensáveis para a função básica.

## 19. Criptografia, identidade e armazenamento de chaves

Escolha protocolos e bibliotecas consolidados para transporte autenticado e criptografado, como TLS moderno com verificação explícita da identidade dos pares, ou alternativa estabelecida justificada. Não construa cifra, assinatura, troca de chaves ou gerenciamento de nonces artesanalmente.

No modo direto, explique precisamente quais garantias o canal fornece entre os computadores. Não adicione camadas redundantes de criptografia apenas para parecer mais seguro. Quando houver intermediário que termine o transporte, preserve uma camada ponta a ponta apropriada ao modelo.

Não aceite certificados desconhecidos silenciosamente, não desative validação e não use uma senha universal embutida no aplicativo. Se usar certificados autoassinados, vincule a confiança à identidade verificada no pareamento e implemente a validação correspondente.

Proteja segredos usando os mecanismos adequados de cada plataforma, com política para cofre bloqueado ou indisponível. Nunca faça fallback silencioso para uma chave em texto simples ao lado do banco. Separe identidade, chaves de sessão, chaves de armazenamento e segredos de convite conforme o desenho escolhido.

Defina geração aleatória segura, rotação, revogação, perda de dispositivo, reinstalação, recuperação e exportação. Não compartilhe automaticamente a mesma identidade privada entre todas as máquinas. A recuperação de um backup não deve clonar uma identidade de forma acidental.

Diferencie apagar uma permissão de apagar dados já entregues. Revogar um computador impede comunicação futura conforme a política, mas não garante recuperar nem destruir o conteúdo que ele recebeu anteriormente.

## 20. Dados sensíveis, modo privado e consentimento

Forneça pausa imediata, modo privado, envio manual de item específico, lista de aplicativos excluídos quando a atribuição de origem for confiável e regras locais de conteúdo. Padrões como chaves privadas, tokens e códigos temporários podem gerar bloqueio ou confirmação, mas não devem ser apresentados como detecção perfeita de segredos.

Respeite sinalizações de conteúdo sensível e de exclusão de histórico ou sincronização disponibilizadas pelos sistemas e aplicativos. Implemente a semântica de cada sinal corretamente: impedir histórico não é necessariamente o mesmo que proibir toda cópia manual. Sem certeza, adote o comportamento mais conservador e explique-o.

Não use apenas o aplicativo em primeiro plano como prova de quem produziu o clipboard. Quando a origem não puder ser determinada, informe “origem desconhecida”. Não colete títulos de janelas, URLs de páginas ou nomes completos de documentos sem necessidade e autorização específica.

A filtragem deve ocorrer antes de persistir, produzir miniaturas, enviar ou registrar informações sobre o conteúdo. O modo privado não pode manter o texto em um log de depuração, cache de busca ou fila esquecida.

Permita expiração curta para itens sensíveis autorizados manualmente. Se houver limpeza automática do clipboard, limpe apenas quando ele ainda corresponder ao item e à geração gerenciados; não apague uma cópia posterior do usuário. Informe quando a API não permite garantir essa condição de forma atômica.

Um bloqueio de sincronização deste aplicativo não impede que outro sincronizador transmita o mesmo conteúdo. Explique esse limite na configuração de privacidade, especialmente quando o compartilhamento do Input Leap ou do próprio sistema continuar ativo.

## 21. Histórico local, pesquisa e organização

O histórico persistente deve ser opcional, com escolha explícita no onboarding e retenção configurável. O modo sem histórico continua sendo um produto completo para sincronização ao vivo. Não transforme favoritos ou estatísticas em uma forma disfarçada de persistir conteúdo que o usuário optou por não guardar.

Quando habilitado, ofereça pesquisa, filtros por tipo e dispositivo, favoritos, etiquetas, exclusão individual e limpeza por período. Mostre origem, momento de recebimento e situação, sem apresentar horário remoto como uma verdade absoluta quando houver divergência de relógios.

Separe deduplicação de conteúdo de registro de ações. Duas cópias iguais podem compartilhar armazenamento, mas ter origens, tempos e permissões diferentes. Limite crescimento por quantidade de itens, bytes, idade e espaço livre. Defina a exceção dos favoritos e evite que ela produza crescimento ilimitado.

Proteja banco, anexos, miniaturas, índices de busca, arquivos temporários, backups e diário transacional. Não declare o histórico criptografado se apenas o campo principal estiver protegido e a busca mantiver uma cópia legível em disco.

Para um histórico pessoal pequeno, avalie busca em memória após desbloqueio ou armazenamento criptografado apropriado antes de adotar mecanismos complexos. Documente o que permanece em memória, quando as chaves ficam acessíveis e o comportamento ao bloquear a sessão.

Diferencie excluir do histórico local, pedir exclusão aos pares e limpar o clipboard ativo. Pedidos remotos de exclusão são operações limitadas aos dados do aplicativo e não oferecem garantia de apagar conteúdo já colado, exportado ou copiado por terceiros.

## 22. Arquivos e pastas: funcionalidade avançada separada

Trate transferência de arquivos como um subsistema próprio. Copiar o texto de um caminho não equivale a transferir um arquivo. Uma lista de caminhos locais só deve gerar envio de arquivos quando esse modo estiver habilitado e o usuário tiver autorizado o conjunto selecionado.

Modele manifesto, tamanho total, quantidade de arquivos, progresso, cancelamento, integridade, retomada e limites. Comece por arquivos regulares selecionados explicitamente; diretórios, muitos arquivos e dados virtuais fornecidos sob demanda entram depois.

O destinatário não pode solicitar qualquer caminho do computador de origem. Somente objetos presentes em uma oferta autorizada e ainda válida podem ser lidos. Revalide o objeto real aberto para reduzir riscos de troca de arquivo, links e mudanças entre verificação e leitura.

Receba em área controlada, com nomes seguros e sem sobrescrever arquivos existentes automaticamente. Trate caminhos absolutos, segmentos de subida de diretório, links simbólicos, junctions, nomes reservados, colisões por maiúsculas e minúsculas, nomes Unicode e extensões enganosas.

Não execute nem abra arquivos automaticamente. Respeite mecanismos de quarentena e sinalização de origem disponíveis na plataforma. Não remova proteções do sistema para facilitar a experiência. Arquivos compactados não devem ser extraídos automaticamente; se a extração futura existir, terá limites próprios e defesa contra caminhos maliciosos e expansão excessiva.

Inicialmente, implemente cópia, não recorte distribuído. Não apague o arquivo de origem para imitar uma movimentação entre máquinas. Uma futura operação de mover exige confirmação, protocolo de conclusão e recuperação cuidadosamente definidos.

Depois de receber e validar, ofereça “abrir pasta” ou preparar referências locais compatíveis com o clipboard do destino. Não prometa Ctrl+C/Ctrl+V universal de arquivos entre todos os gerenciadores antes de testar cada combinação. Em caso de limitação, use uma caixa de recebidos funcional e explícita.

## 23. Regras, filtros e transformações

Implemente um mecanismo de regras compreensível, com prioridades e resolução de conflitos. Condições podem incluir dispositivo, grupo, direção, tipo, tamanho, origem conhecida, rede autorizada e marcação manual de sensibilidade. Ações podem bloquear, pedir confirmação, manter apenas localmente ou permitir para destinos específicos.

A negação explícita deve prevalecer sobre permissões genéricas. Uma transformação não pode remover uma marca de sensibilidade e, com isso, liberar um item anteriormente bloqueado. Reavalie a política sobre a representação efetivamente enviada.

Ofereça um simulador local que explique qual regra seria aplicada, usando texto de teste ou conteúdo fornecido conscientemente. O simulador não deve publicar esse conteúdo nem guardá-lo em logs.

Transformações futuras podem incluir colar como texto simples, remover formatação, converter quebras de linha, formatar JSON e limpar parâmetros de rastreamento em links. Elas devem ser ações explícitas ou regras opt-in, preservando o original quando a retenção estiver autorizada.

Expressões regulares precisam de limites de complexidade e tempo. Não aceite comandos de shell ou scripts arbitrários como regras padrão. Um sistema futuro de extensões deve ter permissões delimitadas, isolamento, revisão de riscos e nenhuma capacidade de exfiltração herdada silenciosamente.

## 24. Interface e experiência de uso

Crie um aplicativo desktop discreto, com bandeja ou menu de sistema quando o ambiente oferecer suporte, janela de gerenciamento e acesso rápido à pausa. Fechar a janela não deve encerrar a sincronização sem explicação; sair do aplicativo deve realmente encerrar o agente ou informar a diferença.

O onboarding deve explicar o acesso ao clipboard, permitir nomear a máquina, escolher modo automático ou manual, autorizar pares, configurar histórico e executar um teste com conteúdo fictício. Use dados de teste gerados para essa finalidade, não o conteúdo pessoal que já estiver copiado.

A tela principal deve mostrar dispositivos, estado da conexão, envio e recebimento, modo de privacidade, última operação e motivo de bloqueios. Evite exibir texto copiado como decoração permanente. Forneça uma visualização protegida ou recolhida por padrão para conteúdos potencialmente sensíveis.

Planeje telas de dispositivos, pareamento, histórico opcional, recebidos e transferências, regras, preferências e diagnóstico. Elas podem ser incorporadas progressivamente; não implemente telas vazias com botões sem efeito para simular completude.

As ações mais frequentes devem ser diretas: pausar, retomar, enviar uma vez, escolher destino, permitir uma vez, copiar um item recebido e revogar um dispositivo. Ações destrutivas ou de ampliação de confiança precisam de confirmação proporcional ao risco.

Exiba erros úteis: o que falhou, qual foi o impacto e o que o usuário pode verificar. Diferencie indisponibilidade de rede, recusa de autorização, limite de tamanho, versão incompatível e limitação de plataforma. Não exponha detalhes sensíveis em mensagens genéricas.

## 25. Atalhos, notificações e acessibilidade

Permita configurar atalhos para abrir o aplicativo, pausar, enviar manualmente e acessar itens recebidos. Detecte conflitos quando a plataforma permitir, ofereça alternativas e evite atalhos que interfiram no Input Leap ou no uso comum do sistema.

Não exija um monitoramento global de teclas para implementar um atalho registrado pelas APIs apropriadas. Quando um atalho global não estiver disponível, mantenha caminhos pela interface e documente a limitação.

Notificações devem ser discretas, agrupáveis e desativáveis por tipo. Por padrão, mostre “item recebido de Notebook” em vez do conteúdo. Respeite modo não perturbe, tela bloqueada e preferência de privacidade. Nomes de arquivos também podem revelar informação sensível.

Garanta navegação por teclado, foco visível, nomes acessíveis, leitura por tecnologias assistivas, contraste adequado, escalonamento, suporte a múltiplos monitores e redução de movimento. Estados não podem depender apenas da cor.

Inclua temas claro e escuro e uma estrutura de tradução sem espalhar textos fixos pelo código. Use formatos locais para a interface, mas mantenha representação de dados e protocolo independente do idioma. Conteúdo com escrita da direita para a esquerda deve ser exibido com cuidado para não mascarar identidade ou extensão de arquivo.

## 26. Arquitetura e escolha de tecnologias

Adote arquitetura modular simples, evitando microserviços desnecessários para um aplicativo pessoal. Separe domínio, políticas, adaptadores de clipboard, descoberta e pareamento, transporte, identidade, armazenamento, observabilidade e interface.

Considere como hipótese inicial um núcleo em Rust, uma interface desktop com Tauri e TypeScript e persistência local apropriada ao modelo de proteção escolhido. Compare brevemente com uma alternativa desktop nativa, como Qt, apenas nos pontos materiais: integração com clipboard, suporte real por plataforma, acessibilidade, distribuição, memória e manutenção.

A escolha definitiva deve decorrer de uma prova técnica dos requisitos críticos. Não imponha uma biblioteca multiplataforma se ela não cobrir observação, formatos, ciclo de vida e permissões necessários. Prefira um adaptador nativo pequeno e testado a fingir que uma abstração incompleta resolve tudo.

Se usar uma interface baseada em WebView, o núcleo deve controlar criptografia, rede, políticas e acesso ao sistema. A interface recebe somente os dados necessários e não tem acesso indiscriminado a arquivos, shell, segredos ou conexões externas. Configure permissões e fronteiras de IPC explicitamente.

Defina se o agente e a interface ficam no mesmo processo ou em processos separados. Comece com a opção mais simples que cumpra o ciclo de vida esperado. Separação em processos exige autenticação e permissões do IPC, prevenção de instâncias duplicadas e tratamento de versões divergentes.

Mantenha uma única autoridade de escrita do clipboard por sessão do usuário. Evite que duas instâncias do aplicativo disputem a mesma área de transferência. Estruture interfaces para testes sem transformar cada função em uma abstração sem necessidade.

## 27. Segurança da interface e das integrações locais

Nunca renderize HTML recebido como conteúdo confiável com acesso a APIs nativas. Prefira visualização textual ou sanitizada, sem scripts, navegação automática, carregamento de imagens remotas, fontes externas ou outros pedidos de rede.

Separe a fidelidade de um formato enviado ao clipboard da segurança da prévia mostrada no aplicativo. Um preview seguro não é justificativa para alterar silenciosamente todo o conteúdo original; as transformações precisam de política explícita.

Aplique política restritiva de carregamento de conteúdo e valide todas as chamadas entre interface e núcleo. O backend deve verificar argumentos e autorização mesmo quando a ação aparentemente veio de uma tela confiável.

Uma API local deve ser opcional, com privilégios mínimos, sem exposição remota por padrão e resistente a acessos indevidos por páginas web ou outros usuários do computador. Prefira canais locais protegidos quando forem adequados. Não crie uma API HTTP sem autenticação apenas porque escuta em localhost.

Dados do clipboard nunca devem ser interpretados como instruções para o aplicativo, para agentes de IA ou para automações. Texto contendo comandos, HTML, instruções de prompt ou caminhos deve continuar sendo dado até que exista uma ação específica e autorizada do usuário.

## 28. Modelo de dados e persistência

Proponha entidades para dispositivo, vínculo de confiança, permissão, grupo, evento de clipboard, representação, referência de conteúdo, estado de entrega, política, configuração e transferência. Acrescente histórico, favoritos e exclusões distribuídas somente nas fases que os utilizarem.

Separe identidade estável, endereço de rede mutável e nome de exibição. Diferencie evento, blob de conteúdo e projeção do clipboard atual. Não replique indiscriminadamente toda a base local para realizar uma sincronização de texto.

Use transações, índices apropriados, migrações versionadas e coleta de objetos órfãos. Evite gravar várias cópias do mesmo blob, mas mantenha controle de referências e permissões. A remoção de um item não pode apagar um conteúdo ainda referenciado por outro item autorizado.

Defina tratamento de corrupção, falta de espaço, interrupção durante migração e restauração. Faça cópia de segurança quando apropriado, sem duplicar conteúdo sensível em arquivo desprotegido. Migrações não podem descartar silenciosamente identidades ou abrir novamente pareamentos revogados.

Explique exclusão lógica, limpeza de arquivos, limitações de apagamento físico e eventual destruição de chaves. Não prometa eliminação forense garantida em SSDs, snapshots, backups ou dispositivos remotos.

## 29. Configuração, CLI e automação

Armazene configurações com esquema versionado, validação, valores padrão e migrações. Separe opções não sensíveis de segredos. Não exporte chaves privadas junto com um relatório de configuração comum.

Ofereça configuração pela interface e, quando útil, por arquivo e linha de comando. Defina precedência, como evitar conflitos e quais mudanças exigem reconexão. Configuração inválida deve gerar diagnóstico legível sem redefinir todo o perfil silenciosamente.

Uma CLI pode oferecer status, dispositivos, pausa, retomada, diagnóstico e envio explicitamente solicitado. Use entrada padrão ou arquivo autorizado para conteúdo sensível, evitando texto secreto em argumentos que possam aparecer no histórico do shell ou na lista de processos.

Comandos que exibem conteúdo ou exportam histórico devem exigir intenção explícita. Os comandos de status e diagnóstico não devem despejar o clipboard. Saídas estruturadas precisam de versão e códigos de retorno documentados.

Não crie webhooks ou integrações externas ativados por padrão. Qualquer futura integração deve indicar destino, tipos de dados e alcance, e passar pelas mesmas regras de privacidade e autorização do restante do aplicativo.

## 30. Desempenho, limites e consumo de recursos

Estabeleça metas iniciais mensuráveis, não promessas sem teste. Para texto de até 10 KiB em uma LAN com RTT de até 20 ms e observação nativa por eventos, proponha inicialmente p95 de até 300 ms entre observação da cópia e aplicação elegível no destino. Plataformas com sondagem devem ter orçamento separado, explicitando seu intervalo.

Como orçamento inicial a validar, busque consumo ocioso inferior a 1% de um núcleo e memória total inferior a 200 MiB no cenário de referência, contando agente e processos da interface. Defina hardware, duração da medição, sistema operacional, estado da interface e tamanho do histórico. Ajuste metas com justificativa se a plataforma exigir.

Proponha limites conservadores, configuráveis e negociados: texto até 1 MiB e imagens até 20 MiB codificados como ponto de partida, além de limites independentes de pixels e memória decodificada. Arquivos precisam de limite próprio, confirmação e quota de disco; não herdam automaticamente o limite do clipboard textual.

Implemente filas limitadas, controle de vazão, cancelamento e prioridade para eventos pequenos. Uma transferência grande não pode congelar a interface ou atrasar indefinidamente um texto novo. Evite carregar o arquivo inteiro em memória para transmiti-lo.

Limite descompressão, paralelismo, largura de banda, cache, logs e tentativas de conexão. Meça bateria e despertares de CPU no notebook. Eventos frequentes não devem causar regravação completa do banco ou renderização integral de um histórico extenso.

Benchmarks devem usar dados sintéticos e produzir resultados reproduzíveis, com percentis e contabilidade clara dos processos. Não publique apenas a melhor execução nem declare que uma meta foi atingida sem medição.

## 31. Diagnóstico, logs e privacidade operacional

Use logs estruturados com identificadores de correlação não derivados diretamente do conteúdo, códigos de erro e estados de conexão. Registre o necessário para diagnosticar problemas, sem texto copiado, imagens, tokens, chaves, convites, caminhos completos ou títulos de documentos.

Não publique hashes de conteúdo como se fossem dados anônimos. O modo de depuração deve continuar seguro por padrão. Qualquer captura excepcional de payload precisa ser local, temporária, claramente autorizada e impossível de ativar acidentalmente em uma versão distribuída.

Ofereça uma tela de diagnóstico com capacidades do sistema, permissões, adaptador usado, conectividade, versão do protocolo, conflitos de instância e motivos recentes de rejeição. Um teste deve gerar conteúdo sintético e explicar que pode modificar o clipboard, solicitando autorização antes disso.

Permita exportar um pacote de suporte com prévia do que será incluído e remoção de identificadores sensíveis. Limite tamanho e retenção dos logs. Crash dumps também podem conter memória sensível; não os envie automaticamente.

Telemetria e relatórios externos devem estar desativados por padrão. Sem conta, a aplicação não deve criar identificadores de rastreamento ocultos. Documente toda comunicação externa, inclusive verificação de atualização, e ofereça operação totalmente local.

Avalie licenças das dependências, política de privacidade e eventuais exigências aplicáveis ao contexto real de uso. Não alegue conformidade legal ou certificação de segurança sem o trabalho necessário. Não ofereça mecanismos para contornar políticas de um equipamento administrado por uma organização.

## 32. Estratégia de testes automatizados

Crie testes unitários para políticas, limites, serialização, identidade de eventos, deduplicação, expiração, autorização, retenção e máquina de estados. Teste propriedades invariantes: eventos aplicados não geram ciclos; pares não autorizados não recebem conteúdo; revogação impede novos envios; filas nunca crescem sem limite.

Use testes de integração com transportes e adaptadores reais quando disponíveis, além de implementações simuladas claramente identificadas. Um teste com clipboard simulado valida a lógica, mas não comprova suporte do sistema operacional.

Monte uma rede de teste com três instâncias e verifique comunicação em todas as direções. Injete perda de conexão, duplicação, atraso, reordenação, reconexão, reinício, destino pausado e mudanças de autorização durante uma transferência.

Inclua fuzzing de parsers, formatos, negociação e mensagens. Teste payloads malformados, tamanhos falsos, campos excessivos, limites de descompressão, eventos repetidos e abuso por um par autenticado. Faça os testes adversariais em ambiente isolado com dados sintéticos.

Cubra falhas de disco, banco bloqueado, migração interrompida, cofre indisponível, permissão revogada, clipboard ocupado e encerramento inesperado. Erros devem produzir resultados previsíveis, não simplesmente ser ignorados.

Não declare cobertura de 100% como sinônimo de qualidade. Defina cobertura prioritária para as regras críticas e mantenha testes de regressão para cada defeito relevante corrigido.

## 33. Validação real por plataforma e por aplicativo

Entregue uma matriz com versão do sistema, ambiente gráfico, arquitetura de CPU, formato, observação automática, leitura, escrita, atalhos, inicialização e situação de validação. Para Wayland, acrescente compositor e versões dos componentes relevantes.

Teste Windows ↔ Windows e, quando houver ambiente disponível, Windows ↔ Linux, Linux ↔ Linux, Windows ↔ macOS e Linux ↔ macOS. Dentro do Linux, não trate X11 ↔ X11, X11 ↔ Wayland e Wayland ↔ Wayland como resultados equivalentes.

Verifique acentos, emojis, caracteres combinantes, texto da direita para a esquerda, linhas longas, tabulações, texto vazio, cópias repetidas, imagens com transparência e formatos múltiplos. Para arquivos, use nomes longos, espaços, Unicode, colisões e alterações durante a transmissão.

Teste cópia em navegadores, terminais, editores, planilhas e aplicações com dados fornecidos sob demanda. Registre quando o aplicativo de destino escolhe uma representação diferente da esperada. Não force equivalência visual quando o formato original não for suportado.

Execute testes com Input Leap ativo e clipboard dele desativado, e um teste separado de conflito com ambos os sincronizadores ativos. O segundo deve validar diagnóstico e comportamento conservador, não estabelecer uma promessa de convivência perfeita.

Use perfis e áreas de trabalho de teste, sem sobrescrever o clipboard pessoal de quem executa a suíte. A restauração automática de dados anteriores só pode ocorrer quando for segura e não apagar uma cópia feita pelo usuário durante o teste.

## 34. Instalação, distribuição e atualização

Entregue instruções reproduzíveis para desenvolvimento e um pacote instalável para a plataforma efetivamente validada. Planeje instaladores e pacotes dos demais sistemas sem fingir que foram gerados ou assinados quando faltarem ferramentas, certificados ou acesso ao ambiente.

A instalação por usuário deve ser preferida quando viável. Inicialização automática precisa de consentimento, pode ser desativada e deve iniciar o agente na sessão correta. Não instale um serviço privilegiado apenas para manter um ícone na bandeja.

A desinstalação deve remover componentes ativos e oferecer escolha clara sobre configurações, identidade, histórico e conteúdo recebido. Não apague arquivos que o usuário tenha movido para fora da área gerenciada. Mostre como revogar esse dispositivo nos demais computadores.

Atualizações devem ter verificação de autenticidade, integridade e versão, tratamento de interrupções e mecanismo seguro de recuperação. Um hash obtido do mesmo servidor comprometido que fornece o binário não substitui uma assinatura verificada contra uma raiz confiável.

Separe chave de assinatura de release das identidades dos dispositivos. Não coloque segredos de assinatura no repositório. Planeje rotação e resposta a comprometimento. Atualizações críticas podem ser destacadas, mas não devem interromper silenciosamente uma transferência ou alterar permissões de sincronização.

Considere compatibilidade entre pares de versões diferentes e migrações de dados. Evite rollback que reative vulnerabilidades, reutilize estado criptográfico indevidamente ou corrompa o banco. Modo portátil, quando oferecido, deve informar suas limitações de proteção de chaves e identidade.

## 35. CI, dependências e qualidade de engenharia

Configure formatação, lint, verificação de tipos, testes, builds e análise de dependências. Use versões fixadas ou arquivos de lock e documente atualização controlada. Prefira dependências mantidas e necessárias; não adicione serviços ou bibliotecas apenas por conveniência superficial.

Produza inventário de componentes e licenças, e configure verificação de vulnerabilidades e vazamento de segredos. Resultados de ferramentas precisam ser revisados; ausência de alerta não equivale a ausência de vulnerabilidade.

A automação deve diferenciar compilar para uma plataforma de executar testes nela. Runners sem sessão gráfica não comprovam funcionamento de clipboard, bandeja, atalhos ou permissões. Identifique os testes manuais ou instrumentados que ainda faltam.

Proteja a cadeia de publicação, permissões do pipeline, artefatos, assinatura e proveniência de builds. Busque builds reproduzíveis onde for viável e documente diferenças conhecidas. Não faça download e execução de scripts remotos não verificados como parte opaca da instalação.

Organize código, comentários e erros para manutenção. Evite capturas genéricas de exceção que escondam corrupção ou falha de segurança. Concorrência, cancelamento e propriedade de dados devem ter desenho explícito e testes.

## 36. Backup, recuperação e manutenção

Ofereça exportação e importação de configurações sem incluir segredos por padrão. Histórico e identidade, quando exportáveis, precisam de fluxos separados, proteção adequada, confirmação e explicação das consequências.

Defina como recuperar acesso após perda de uma máquina, cofre inacessível ou corrupção do perfil. Um novo dispositivo não deve ganhar confiança apenas porque importou o nome de um antigo. Repareamento seguro deve ser um caminho simples.

Planeje retenção e limpeza de arquivos recebidos, temporários, thumbnails e backups. Não remova um arquivo que ainda esteja sendo usado para uma oferta ativa ou que o usuário tenha escolhido preservar. Falhas de limpeza precisam ser registradas sem expor os conteúdos.

Mantenha changelog, guia de solução de problemas, política de suporte, matriz de compatibilidade e processo de relato responsável de vulnerabilidades. Documente limitações conhecidas e alternativas seguras, em vez de esconder limitações sob mensagens genéricas.

Crie procedimentos para comprometimento de chave, publicação defeituosa e descoberta de falha no protocolo. A capacidade de desativar um recurso vulnerável não deve se transformar em um mecanismo remoto irrestrito de controle sobre os computadores.

## 37. Recursos futuros e experimentais

Avalie clientes Android e iOS somente em uma fase própria. Pesquise as restrições reais de clipboard, execução em segundo plano, permissões e compartilhamento nas versões suportadas. Ofereça ações manuais e extensões de compartilhamento quando adequadas; não prometa monitoramento contínuo universal.

Considere extensão de navegador ou integração com editor como componentes opcionais, com permissões mínimas e comunicação autenticada com o agente local. O aplicativo desktop não deve depender de manter o navegador aberto.

Snippets e modelos de texto podem ser recursos úteis, mas são dados gerenciados separadamente do histórico capturado. OCR de imagens deve ser opt-in, preferencialmente local e com controle da retenção do texto extraído. Tradução e assistência por IA exigem consentimento separado e indicação explícita de qualquer envio externo.

Nenhum recurso de IA pode transformar o texto recebido em instrução privilegiada nem executar comandos sugeridos. O aplicativo básico precisa funcionar sem modelo de linguagem, chave de API, internet ou cobrança por uso.

Sincronização completa de histórico, notas compartilhadas e grupos com outras pessoas exigem revisão do modelo de autorização, privacidade, exclusão e chaves. Não generalize automaticamente a confiança entre minhas máquinas para usuários diferentes.

Documente também ideias como integração com dispositivos headless, envio para uma caixa de recebidos sem clipboard gráfico, transferência sob demanda e automações locais limitadas. Mantenha todas essas ideias em um backlog separado, com custo, risco e valor, sem atrasar o fluxo principal.

## 38. Critérios de aceite verificáveis

A versão inicial só é utilizável quando três instâncias autorizadas conseguem trocar texto nas direções permitidas, com conteúdo íntegro e sem depender de troca de tela no Input Leap. Uma máquina não autorizada não pode ler conteúdo, publicar eventos nem aplicar alterações ao clipboard.

Pareamento precisa exigir verificação, reconexão precisa autenticar identidades e o conteúdo não pode aparecer em claro na captura de tráfego do transporte. Esse teste não deve ser apresentado como prova completa de segurança; é apenas uma verificação necessária.

Pausar deve interromper as operações anunciadas pela interface. Revogar um par deve fechar ou invalidar sessões e cancelar filas afetadas. Conteúdo privado ou expirado não pode reaparecer depois de reiniciar ou reconectar.

O ciclo A → B → C → A deve terminar sem tempestade de eventos. Copiar o mesmo texto novamente de forma explícita deve continuar possível. Uma transferência atrasada não pode sobrescrever uma cópia local posterior contra a política definida.

Falha de rede, clipboard ocupado, versão incompatível, falta de espaço e permissão negada devem gerar diagnóstico e recuperação controlada. O aplicativo não pode pedir para desligar a segurança do sistema ou o firewall inteiro como solução padrão.

Quando histórico estiver desativado, não deve existir persistência oculta de payloads. Quando habilitado, a proteção prometida deve abranger conteúdo e derivados. Logs e relatórios não devem expor os dados copiados.

Recursos não disponíveis precisam ser indicados honestamente. “Multiplataforma” exige uma matriz de evidências; compilar, simular e testar em sistema real são estados diferentes. Entregue evidências para cada critério ou marque-o como pendente.

## 39. Entregáveis do projeto

Entregue uma especificação de produto com premissas, casos de uso, requisitos priorizados e critérios de aceite. Inclua arquitetura, diagramas de componentes e sequência, decisões técnicas registradas, modelo de ameaças e matriz de compatibilidade.

Forneça repositório organizado, código completo da etapa implementada, arquivos de dependências, configuração de exemplo sem segredos, scripts de desenvolvimento e instruções de build e execução. Não use reticências para substituir trechos indispensáveis nem chamadas para funções que não existem.

Documente protocolo, modelos de dados, política de conflitos, pareamento, armazenamento e limites. Inclua guia específico para as três máquinas, teste de conexão, convivência com Input Leap, pausas, revogação e resolução de problemas.

Entregue testes e resultados reais, indicando ambiente e comandos executados. Diferencie teste planejado de teste realizado e resultado observado. Inclua benchmarks quando tiverem sido medidos e não preencha números fictícios.

Acrescente instalador ou artefato de execução quando o ambiente permitir, documentação de distribuição, lista de dependências e limitações conhecidas. Se assinatura, notarização ou build de outro sistema dependerem de credenciais e infraestrutura ausentes, identifique isso objetivamente.

Mantenha um mapa ligando cada requisito ao módulo responsável, aos testes e ao estado da implementação. O resumo final deve permitir que eu saiba exatamente o que consigo usar hoje e o que continua no backlog.

## 40. Ordem de execução e comportamento esperado

Comece apresentando a compreensão do cenário, as premissas assumidas e a menor fatia funcional que será construída. Faça uma análise curta dos riscos mais importantes: acesso nativo ao clipboard, pareamento, ciclos, concorrência e proteção de dados.

Valide as capacidades do ambiente e escolha a stack. Prove leitura e escrita na plataforma inicial, depois implemente uma comunicação autenticada mínima entre dois pares. Acrescente prevenção de ciclos, políticas e teste com três instâncias antes de investir em telas avançadas.

Trabalhe em incrementos executáveis, incluindo testes e documentação em cada etapa. Segurança essencial faz parte de cada incremento; não deixe autenticação, criptografia ou validação para uma hipotética revisão final.

Se o ambiente permitir editar arquivos e executar comandos, realize o trabalho e relate evidências. Se só permitir resposta textual, entregue arquivos e instruções completos da primeira fatia funcional, deixando claro que não foram executados. Não apresente uma demonstração de interface como implementação do sincronizador.

Se o escopo ultrapassar uma única entrega, conclua uma etapa coerente dentro da entrega atual, registre pendências e descreva a próxima fatia. Não prometa trabalho em segundo plano, não diga que testes foram executados sem evidência e não peça confirmação a cada pequeno passo quando houver uma decisão conservadora e reversível disponível.

Antes de concluir, revise se alguma exigência deste documento entrou em conflito com outra. Resolva explicitamente conflitos entre privacidade e conveniência, fidelidade e sanitização, convergência e cópia local recente, histórico e modo privado, simplicidade e extensibilidade.

A prioridade final é esta: um aplicativo confiável para copiar em uma das minhas máquinas e colar nas outras autorizadas, que continue convivendo com o Input Leap, explique seus limites e não transforme minha área de transferência em um canal de exposição involuntária de dados.

---

## Referências técnicas para a implementação

Estas referências orientam a validação das APIs e dos limites. Confira a documentação e a versão realmente utilizada antes de implementar; elas não substituem testes no ambiente-alvo.

- Input Leap, repositório e releases: https://github.com/input-leap/input-leap e https://github.com/input-leap/input-leap/releases
- Microsoft, uso do clipboard: https://learn.microsoft.com/en-us/windows/win32/dataxchg/using-the-clipboard
- Microsoft, formatos e exclusão de histórico/sincronização: https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats
- Apple, NSPasteboard: https://developer.apple.com/documentation/appkit/nspasteboard
- Apple, changeCount: https://developer.apple.com/documentation/appkit/nspasteboard/changecount
- Apple, comportamento de acesso: https://developer.apple.com/documentation/AppKit/NSPasteboard/accessBehavior-86972
- XDG Desktop Portal, clipboard: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Clipboard.html
- Tauri, segurança e permissões: https://v2.tauri.app/security/ e https://v2.tauri.app/security/capabilities/
- Tauri, plugin de clipboard: https://v2.tauri.app/plugin/clipboard/
