// Dados falsos para abrir a interface no navegador, sem o agente.
// O mesmo formato de resposta vale para o invoke do Tauri (contrato §12).

const ATRASO_MS = 180;

const estado = {
  maquina: { nome: "notebook-a", impressao: "f3d7-ea9a" },
  malha: { conectados: 3, total: 3 },
  pausado: false,
};

const comandos = {
  estado_geral: () => structuredClone(estado),
  pausar_tudo: ({ pausado }) => {
    estado.pausado = pausado;
    return structuredClone(estado);
  },
};

export async function invocar(comando, args = {}) {
  const tratar = comandos[comando];
  if (!tratar) throw new Error(`comando desconhecido no mock: ${comando}`);
  await new Promise((resolver) => setTimeout(resolver, ATRASO_MS));
  return tratar(args);
}
