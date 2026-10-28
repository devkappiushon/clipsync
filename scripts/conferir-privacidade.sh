#!/usr/bin/env bash
# Condição R5 do Sentinel (7cd67a3) e achado S7-04 (c8cf755): o código que toca os
# hooks de baixo nível e a tradução de teclas não registra nada sobre teclas.
# Arquivos de código (src) do clipsync-win, keymap.rs e adapter/win*.rs e adapter/win/**:
#   - proibidos println!, eprintln!, print!, eprint! e dbg!;
#   - macro de log (trace/debug/info/warn/error, com ou sem tracing:: ou log::)
#     só com um texto fixo numa linha, sem chaves: warn!("hook caiu").
#     Qualquer argumento (campo, formato, ?valor, %valor) falha.
# Roda da raiz do repositório, no CI dos dois sistemas (bash, grep e awk).
set -euo pipefail
arquivos=$(git ls-files -- 'crates/clipsync-win/src/*.rs' 'crates/clipsync-win/src/**/*.rs' \
  'crates/clipsync-input/src/keymap.rs' 'crates/clipsync-input/src/adapter/win*.rs' \
  'crates/clipsync-input/src/adapter/win/*.rs' 'crates/clipsync-input/src/adapter/win/**/*.rs' | sort -u)
[ -n "$arquivos" ] || { echo "nenhum arquivo de hook ainda: ok"; exit 0; }
if printf '%s\n' "$arquivos" | tr '\n' '\0' | xargs -0 awk '
  {
    codigo = $0; sub(/\/\/.*/, "", codigo)
    if (codigo ~ /(^|[^A-Za-z0-9_])(e?println|e?print|dbg)!/) { print FILENAME ":" FNR ": impressão proibida: " $0; ruim = 1; next }
    if (codigo ~ /(^|[^A-Za-z0-9_:])((tracing|log)::)?(trace|debug|info|warn|error)!/) {
      if (codigo !~ /(trace|debug|info|warn|error)!\("[^"{}]*"\)/) { print FILENAME ":" FNR ": log com argumento: " $0; ruim = 1 }
    }
  }
  END { exit ruim }' >&2; then
  echo "privacidade dos hooks: ok ($(printf '%s\n' "$arquivos" | wc -l | tr -d ' ') arquivos)"
else
  exit 1
fi
