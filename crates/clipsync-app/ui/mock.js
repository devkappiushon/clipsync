// Mock do agente para abrir a interface no navegador (nunca entra no app Tauri).
// Cenários: ?cenario=padrao | vazio | sem-perfil | erro | pareamento | estados
// Plataforma: ?plataforma=windows desliga os recursos de teclado e mouse.
// Cada domínio mora em mock/*.js e registra seus comandos no núcleo.

import "./mock/dados.js";
import "./mock/comuns.js";
import "./mock/onboarding.js";
import "./mock/dispositivos.js";
import "./mock/pareamento.js";
import "./mock/layout.js";
import "./mock/clipboard.js";
import "./mock/diagnostico.js";
import "./mock/prefs.js";

export { invoke, listen } from "./mock/nucleo.js";
