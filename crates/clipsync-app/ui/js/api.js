// Único ponto de acesso a dados (contrato §12.4). Dentro do Tauri, um comando só:
// invoke("clipsync", { req: { cmd, args } }). Fora dele, ui/mock.js responde igual.
// Erro: a promessa rejeita com { code, message, field, details }; a message já vem em português.

let mock = null;

async function chamar(cmd, args) {
  const req = args === undefined ? { cmd } : { cmd, args };
  const tauri = window.__TAURI__;
  if (tauri) return tauri.core.invoke("clipsync", { req });
  mock ??= await import("../mock.js");
  return mock.invoke("clipsync", { req });
}

const comando = (nome) => (args) => chamar(nome, args);

export const api = {
  // comuns
  appState: comando("app_state"),
  agentStart: comando("agent_start"),
  agentStop: comando("agent_stop"),
  pauseSet: comando("pause_set"),
  // 1 onboarding
  onboardingStart: comando("onboarding_start"),
  onboardingCheckName: comando("onboarding_check_name"),
  onboardingCreate: comando("onboarding_create"),
  onboardingSelfTest: comando("onboarding_self_test"),
  // 2 dispositivos
  devicesList: comando("devices_list"),
  deviceSetPerms: comando("device_set_perms"),
  deviceRevoke: comando("device_revoke"),
  devicePause: comando("device_pause"),
  // 3 pareamento
  pairHostStart: comando("pair_host_start"),
  pairJoinStart: comando("pair_join_start"),
  pairCodeMatches: comando("pair_code_matches"),
  pairFinish: comando("pair_finish"),
  pairCancel: comando("pair_cancel"),
  pairState: comando("pair_state"),
  // 4 layout
  layoutGet: comando("layout_get"),
  layoutSetLink: comando("layout_set_link"),
  layoutRemoveLink: comando("layout_remove_link"),
  layoutSetDevice: comando("layout_set_device"),
  layoutSetOptions: comando("layout_set_options"),
  layoutCopyToPeers: comando("layout_copy_to_peers"),
  layoutOfferAnswer: comando("layout_offer_answer"),
  lockToScreen: comando("lock_to_screen"),
  switchToDevice: comando("switch_to_device"),
  inputTest: comando("input_test"),
  // 5 clipboard
  clipboardGet: comando("clipboard_get"),
  clipboardPreview: comando("clipboard_preview"),
  clipboardSetMode: comando("clipboard_set_mode"),
  clipboardCopyItem: comando("clipboard_copy_item"),
  clipboardDiscardItem: comando("clipboard_discard_item"),
  clipboardSendNow: comando("clipboard_send_now"),
  // 6 diagnóstico
  diagnosticsGet: comando("diagnostics_get"),
  supportBundlePreview: comando("support_bundle_preview"),
  supportBundleExport: comando("support_bundle_export"),
  // 7 preferências
  prefsGet: comando("prefs_get"),
  prefsSet: comando("prefs_set"),
};

/** Escuta os eventos do agente. fn recebe { event, data }. Devolve (promessa de) a função que cancela. */
export async function ouvir(fn) {
  const tauri = window.__TAURI__;
  if (tauri) return tauri.event.listen("clipsync://event", (evento) => fn(evento.payload));
  mock ??= await import("../mock.js");
  return mock.listen("clipsync://event", (evento) => fn(evento.payload));
}
