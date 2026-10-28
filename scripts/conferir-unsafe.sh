#!/usr/bin/env bash
# Prova do contrato §13.3 item 5: o único lugar com unsafe no repositório é
# crates/clipsync-win/src/ffi.rs, que tem o único #![allow(unsafe_code)].
# Roda da raiz do repositório, no CI dos dois sistemas (bash).
set -euo pipefail
ffi="crates/clipsync-win/src/ffi.rs"
falhou=0

# git grep: só arquivos rastreados, igual no Linux e no Windows.
allows=$(git grep -n 'allow(unsafe_code)' -- 'crates/*.rs' 'crates/**/*.rs' || true)
fora=$(printf '%s\n' "$allows" | grep -v "^${ffi}:" | grep -v '^$' || true)
if [ -n "$fora" ]; then
  echo "allow(unsafe_code) fora de ${ffi}:" >&2; echo "$fora" >&2; falhou=1
fi
if [ -f "$ffi" ]; then
  n=$(printf '%s\n' "$allows" | grep -c "^${ffi}:" || true)
  if [ "$n" -ne 1 ]; then
    echo "${ffi} deve ter exatamente um allow(unsafe_code); tem ${n}" >&2; falhou=1
  fi
fi

usos=$(git grep -nE '\bunsafe\s*(\{|fn|impl|extern|trait)' -- 'crates/*.rs' 'crates/**/*.rs' || true)
fora=$(printf '%s\n' "$usos" | grep -v "^${ffi}:" | grep -v '^$' || true)
if [ -n "$fora" ]; then
  echo "unsafe fora de ${ffi}:" >&2; echo "$fora" >&2; falhou=1
fi

if [ "$falhou" -ne 0 ]; then exit 1; fi
echo "unsafe só em ${ffi}: ok ($(printf '%s\n' "$usos" | grep -c . || true) ocorrências)"
