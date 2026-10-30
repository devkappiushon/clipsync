// Continuidade de interação. Chaves são comparadas como texto, nunca interpoladas em seletores.
const documentos = new WeakMap();
function observarFoco(doc) {
  if (!documentos.has(doc)) {
    const estado = { versao: 0, ultimo: null };
    doc.addEventListener('focusin', ({ target }) => {
      if (target !== doc.body && target !== doc.documentElement) {
        estado.versao++;
        estado.ultimo = target;
      }
    });
    documentos.set(doc, estado);
  }
  return documentos.get(doc);
}
const rotaAtual = (doc) => doc.defaultView?.location.href;
const disponivel = (no) => no?.isConnected && !no.disabled && !no.closest?.('[hidden], [inert]');

/** Guardar antes da espera/repintura; restaurar uma vez, na mesma raiz ainda montada. */
export function guardarFoco(raiz) {
  const doc = raiz.ownerDocument;
  const original = doc.activeElement;
  if (!original || !raiz.contains(original) || original === doc.body) return null;
  const estado = observarFoco(doc);
  return {
    original, chave: original.dataset?.focusKey, id: original.id,
    doc, rota: rotaAtual(doc), modal: doc.querySelector('dialog[open]'), versao: estado.versao,
    selecao: typeof original.selectionStart === 'number'
      ? [original.selectionStart, original.selectionEnd, original.selectionDirection] : null,
  };
}

/** Antes de abrir uma nova etapa após await: não sobrepor a escolha posterior do usuário. */
export function interacaoAtual(registro, raiz) {
  return !!registro && raiz.isConnected && rotaAtual(registro.doc) === registro.rota &&
    observarFoco(registro.doc).versao === registro.versao &&
    registro.doc.querySelector('dialog[open]') === registro.modal;
}

/** fallback: nó ou () => nó. aposDialogo é reservado ao retorno do modal que acabou de fechar. */
export function restaurarFoco(registro, raiz, { fallback, aposDialogo } = {}) {
  if (!registro || !raiz.isConnected) return false;
  const { doc, original } = registro;
  const estado = observarFoco(doc);
  const modal = doc.querySelector('dialog[open]');
  if (rotaAtual(doc) !== registro.rota || (modal && modal !== registro.modal)) return false;
  if (estado.versao !== registro.versao && !(aposDialogo &&
      (aposDialogo.contains(estado.ultimo) || estado.ultimo === original))) return false;
  const atual = doc.activeElement;
  if (atual === original && disponivel(original)) return false;
  if (disponivel(atual) && atual !== doc.body && atual !== doc.documentElement && atual !== original && !aposDialogo?.contains(atual)) return false;
  const equivalente = registro.chave
    ? [...raiz.querySelectorAll('[data-focus-key]')].find((no) => no.dataset.focusKey === registro.chave)
    : registro.id ? [...raiz.querySelectorAll('[id]')].find((no) => no.id === registro.id) : null;
  const alvo = [original, equivalente, typeof fallback === 'function' ? fallback() : fallback]
    .find((no) => disponivel(no) && raiz.contains(no));
  if (!alvo) return false;
  if (alvo.tabIndex < 0 && alvo.matches?.("h1, h2, h3, [data-focus-fallback]")) alvo.tabIndex = -1;
  alvo.focus({ preventScroll: true });
  if (alvo === equivalente && registro.selecao) alvo.setSelectionRange?.(...registro.selecao);
  return doc.activeElement === alvo;
}

/** Ocupação transitória: nunca escreve disabled, que pertence à disponibilidade real. */
export function ocupar(no) {
  if (no.disabled || no.getAttribute('aria-disabled') === 'true' || no.getAttribute('aria-busy') === 'true') return null;
  const anterior = no.getAttribute('aria-disabled');
  no.setAttribute('aria-busy', 'true');
  no.setAttribute('aria-disabled', 'true');
  return () => {
    no.removeAttribute('aria-busy');
    if (anterior === null) no.removeAttribute('aria-disabled');
    else no.setAttribute('aria-disabled', anterior);
  };
}

/** Valor confirmado só muda após sucesso; segunda tentativa durante espera é ignorada. */
export function criarEscolhaAssincrona({ valor, aplicar, aoMudar, aoOcupar = () => {}, disponivel: podeMudar = () => true }) {
  let confirmado = valor;
  let ocupado = false;
  return async (proximo) => {
    aplicar(confirmado);
    if (ocupado || !podeMudar(proximo) || proximo === confirmado) return;
    ocupado = true;
    aoOcupar(true);
    try {
      await aoMudar?.(proximo);
      confirmado = proximo;
    } finally {
      aplicar(confirmado);
      ocupado = false;
      aoOcupar(false);
    }
  };
}

/** Monte no antes de anunciar. Regiões permanecem vazias/montadas; texto igual não é reanunciado. */
export function criarAnunciador() {
  const no = document.createElement('div');
  no.className = 'anunciador';
  const status = document.createElement('p');
  const erro = document.createElement('p');
  for (const [regiao, papel] of [[status, 'status'], [erro, 'alert']]) {
    regiao.className = 'so-leitor';
    regiao.setAttribute('role', papel);
    regiao.setAttribute('aria-atomic', 'true');
    no.append(regiao);
  }
  const escrever = (alvo, outro, texto) => {
    const mensagem = texto == null ? '' : String(texto);
    if (outro.textContent) outro.textContent = '';
    if (alvo.textContent !== mensagem) alvo.textContent = mensagem;
  };
  return { no, status: (texto) => escrever(status, erro, texto), erro: (texto) => escrever(erro, status, texto),
    limpar: () => { escrever(status, erro, ''); } };
}
