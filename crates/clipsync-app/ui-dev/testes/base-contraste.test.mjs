// Calcula contraste dos valores CSS realmente usados. Não mede pixels/foco no navegador.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
const tokens = readFileSync(new URL('../../ui/css/tokens.css', import.meta.url), 'utf8');
const componentes = readFileSync(new URL('../../ui/css/components.css', import.meta.url), 'utf8');
const campo = componentes.match(/\.entrada\s*\{([^}]+)\}/)[1];
const tokenBorda = campo.match(/border:\s*1px solid var\((--[\w-]+)\)/)[1];
const cores = (bloco) => Object.fromEntries([...bloco.matchAll(/(--[\w-]+):\s*(#[\da-f]{6})/g)].map((m) => [m[1], m[2]]));
function luminancia(hex) {
  const canais = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
    .map((v) => v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
  return canais[0] * 0.2126 + canais[1] * 0.7152 + canais[2] * 0.0722;
}
for (const [tema, bloco] of [['claro', tokens.match(/:root\s*\{([^}]+)\}/)[1]],
  ['escuro', tokens.match(/:root\[data-theme="dark"\]\s*\{([^}]+)\}/)[1]]]) {
  test(`contorno habilitado tem contraste 3:1 nos fundos do tema ${tema}`, () => {
    const c = cores(bloco);
    for (const fundo of ['--bg', '--panel', '--panel-hover']) {
      const a = luminancia(c[tokenBorda]); const b = luminancia(c[fundo]);
      const contraste = (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
      assert.ok(contraste >= 3, `${tema} / ${fundo}: ${contraste.toFixed(2)}:1`);
    }
  });
}
