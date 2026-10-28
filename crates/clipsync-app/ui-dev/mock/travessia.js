// Apenas simulador. Espelha a forma de app_state.traversal, nunca afirma saúde dos hooks.
import { agenteDePe, estado } from "./nucleo.js";

const mensagens = {
  not_initialized: "Configure esta máquina antes de passar o mouse.",
  agent_stopped: "O agente está parado. Inicie-o para conferir a passagem do mouse.",
  input_off: "O módulo de teclado e mouse está desligado nesta máquina.",
  paused: "Tudo está pausado. Retome para passar o mouse.",
  screen_locked: "O ClipSync está travado nesta tela. Destrave para sair com o mouse.",
  no_local_link: "O mouse desta máquina não sai pelas bordas. Configure a passagem no computador que tem o mouse físico. Uma máquina que só recebe não precisa de ligação local.",
  peer_offline: "O outro computador está offline. Conecte-o antes de testar.",
  remote_input_off: "O módulo de teclado e mouse está desligado no outro computador.",
  peer_too_old: "O protocolo do outro computador não suporta a passagem por monitor. Atualize o app.",
  checking: "Conferindo a configuração da passagem do mouse.",
  unverified: "Passagem configurada. Falta testar o mouse e conferir a permissão de controle no destino.",
  active: "O agente informa uma sessão de saída. Confira o movimento e a volta com o mouse.",
};

export function travessiaAtual() {
  const pares = estado.dispositivos ?? [];
  const conhecidos = new Set(pares.map((p) => p.peer_id));
  const local = estado.eu?.peer_id;
  const links = (estado.layout?.links ?? []).filter((l) => (l.a.peer_id === local && conhecidos.has(l.b.peer_id)) || (l.b.peer_id === local && conhecidos.has(l.a.peer_id)));
  const bloqueio = !estado.inicializado ? "not_initialized" : !agenteDePe() ? "agent_stopped" : !estado.modulos.input ? "input_off" : estado.pausado ? "paused" : estado.telaTravada ? "screen_locked" : links.length === 0 ? "no_local_link" : null;
  const peers = pares.map((p) => {
    const code = bloqueio ?? (p.state !== "connected" ? "peer_offline" : !p.remote_modules.input ? "remote_input_off" : p.remote_version == null || p.remote_version === 0 ? "checking" : p.remote_version === 1 ? "peer_too_old" : !links.some((l) => l.a.peer_id === p.peer_id || l.b.peer_id === p.peer_id) ? "no_local_link" : estado.layout.active_peer_id === p.peer_id ? "active" : "unverified");
    return { peer_id: p.peer_id, code, detail: mensagens[code], protocol_warning: p.remote_version > 0 && p.remote_version < 3 ? `O par usa o protocolo ${p.remote_version}. Atualize os apps para usar o retorno pelo trecho da tela.` : null };
  });
  const code = bloqueio ?? (peers.some((p) => p.code === "active") ? "active" : peers.some((p) => p.code === "unverified") ? "unverified" : peers[0]?.code ?? "checking");
  return { code, detail: mensagens[code], local_links: links.length, last_block: estado.bloqueioRecusa ? "O Windows não liberou Win+Alt+P para retomar: use a bandeja ou o botão Pausar tudo." : null, peers };
}
