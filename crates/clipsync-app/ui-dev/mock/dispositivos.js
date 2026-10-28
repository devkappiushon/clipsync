// Mock do domínio dispositivos: devices_list, device_set_perms, device_revoke e device_pause (unavailable).

import { agenteDePe, emitir, erro, estado, registrar } from "./nucleo.js";

function achar(peerId) {
  const dispositivo = estado.dispositivos.find((d) => d.peer_id === peerId);
  if (!dispositivo) throw erro("unknown_peer", "Este dispositivo não está mais na lista de pares.");
  return dispositivo;
}

registrar({
  devices_list: () => {
    const rodando = agenteDePe();
    return {
      agent_running: rodando,
      devices: estado.dispositivos.map((d) => (rodando ? d : { ...d, state: "offline" })),
    };
  },

  device_set_perms: ({ peer_id, perms, acknowledge_full_trust = false }) => {
    const dispositivo = achar(peer_id);
    if (!perms || typeof perms !== "object") throw erro("bad_request", "Pedido malformado: faltam as permissões.");
    const clipboardIn = perms.clipboard_in === true;
    const clipboardOut = perms.clipboard_out === true;
    const inputIn = perms.input_in === true;
    if ((clipboardIn || clipboardOut) && !estado.modulos.clipboard) {
      throw erro("module_off", "O módulo de clipboard está desligado nesta máquina.", { field: clipboardIn ? "clipboard_in" : "clipboard_out" });
    }
    if (inputIn && !estado.modulos.input) {
      throw erro("module_off", "O módulo de teclado e mouse está desligado nesta máquina.", { field: "input_in" });
    }
    if (inputIn && !dispositivo.perms.input_in && acknowledge_full_trust !== true) {
      throw erro("needs_confirmation", "Conceder controle é confiança total. Confirme para continuar.", { field: "input_in" });
    }
    dispositivo.perms = { clipboard_in: clipboardIn, clipboard_out: clipboardOut, input_in: inputIn };
    return { device: dispositivo };
  },

  device_revoke: ({ peer_id, typed_name }) => {
    const dispositivo = achar(peer_id);
    if (typed_name !== dispositivo.name) {
      throw erro("name_mismatch", "O nome digitado não é o nome deste dispositivo.", { field: "typed_name" });
    }
    estado.dispositivos = estado.dispositivos.filter((d) => d !== dispositivo);
    if (dispositivo.state === "connected") {
      emitir("peer_disconnected", { peer_id: dispositivo.peer_id, short: dispositivo.short, name: dispositivo.name });
    } else {
      emitir("resync", {});
    }
    return { removed: true };
  },

  device_pause: () => {
    throw erro("unavailable", "Pausar um par só: indisponível nesta versão. Use Pausar tudo.");
  },
});
