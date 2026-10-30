// Motivos de apresentação usados só quando o gate de input já está bloqueado.
export function motivoInputIndisponivel(app) {
  if (app?.input_supported === false) return "Teclado e mouse: indisponível nesta versão neste sistema.";
  if (app?.modules?.input === false) return "O módulo de teclado e mouse está desligado neste computador. Confira Módulos em Preferências.";
  return "Não foi possível confirmar os recursos de teclado e mouse deste computador. Consulte o Diagnóstico.";
}
