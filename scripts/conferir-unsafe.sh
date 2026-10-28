#!/usr/bin/env bash
# Prova do contrato §13.3 item 5, com as condições C1 e C2 do Sentinel (7cd67a3).
# Sob `deny`, um allow(unsafe_code) fora do ffi.rs compila: o isolamento do
# clipsync-win depende deste script. Regras, só em arquivos rastreados:
#   1. fora de crates/clipsync-win/src/ffi.rs, nenhuma linha .rs cita `unsafe_code`
#      (pega allow e expect em qualquer forma, inclusive em várias linhas);
#   2. fora dele, nenhuma palavra `unsafe` no código (comentários `//` não contam;
#      pega `unsafe {`, `unsafe` com a chave na linha de baixo, `unsafe fn`,
#      `unsafe impl` e `#[unsafe(...)]`);
#   3. todo crate de crates/ menos o clipsync-win tem `[lints]` com `workspace = true`;
#   4. o ffi.rs, quando existe, tem exatamente uma linha com `unsafe_code`, o
#      `#![allow(unsafe_code)]`, e não exporta `unsafe fn` (C2);
#   5. o Cargo.toml do clipsync-win mantém os lints da §13.3 item 2: em
#      [lints.rust] unsafe_code e unsafe_op_in_unsafe_fn em "deny"; em
#      [lints.clippy] undocumented_unsafe_blocks e multiple_unsafe_ops_per_block
#      em "deny", e nenhuma das seis do workspace afrouxada.
# Roda da raiz do repositório, no CI dos dois sistemas (bash, grep e awk).
set -euo pipefail
ffi="crates/clipsync-win/src/ffi.rs"
falhou=0

arquivos=$(git ls-files -- 'crates/*.rs' 'crates/**/*.rs' | grep -vx "$ffi" || true)
if [ -n "$arquivos" ]; then
  if ! printf '%s\n' "$arquivos" | tr '\n' '\0' | xargs -0 awk '
    {
      linha = $0; codigo = linha; sub(/\/\/.*/, "", codigo)
      if (linha ~ /unsafe_code/) { print FILENAME ":" FNR ": unsafe_code fora do ffi.rs: " linha; ruim = 1 }
      else if (codigo ~ /(^|[^A-Za-z0-9_])unsafe([^A-Za-z0-9_]|$)/) { print FILENAME ":" FNR ": unsafe fora do ffi.rs: " linha; ruim = 1 }
    }
    END { exit ruim }' >&2; then
    falhou=1
  fi
fi

for toml in crates/*/Cargo.toml; do
  [ "$toml" = "crates/clipsync-win/Cargo.toml" ] && continue
  if ! awk '/^\[lints\]/{f=1;next} /^\[/{f=0} f && /^[[:space:]]*workspace[[:space:]]*=[[:space:]]*true/{ok=1} END{exit ok?0:1}' "$toml"; then
    echo "$toml sem [lints] workspace = true" >&2; falhou=1
  fi
done

if [ -f "$ffi" ]; then
  n=$(grep -c 'unsafe_code' "$ffi" || true)
  if [ "$n" -ne 1 ] || ! grep -qE '^[[:space:]]*#!\[allow\(unsafe_code\)\][[:space:]]*$' "$ffi"; then
    echo "$ffi deve ter exatamente uma linha com unsafe_code, o #![allow(unsafe_code)]; tem $n" >&2; falhou=1
  fi
  if grep -nE '^[[:space:]]*pub(\([a-z]+\))?[[:space:]]+(const[[:space:]]+)?unsafe[[:space:]]+(extern[[:space:]]+"[^"]*"[[:space:]]+)?fn' "$ffi" >&2; then
    echo "$ffi não pode exportar unsafe fn (C2)" >&2; falhou=1
  fi
fi

win_toml="crates/clipsync-win/Cargo.toml"
if [ -f "$win_toml" ]; then
  exige() {  # seção, lint
    awk -v sec="[$1]" -v lint="$2" '
      $0 == sec { f = 1; next } /^\[/ { f = 0 }
      f && $0 ~ "^[[:space:]]*" lint "[[:space:]]*=[[:space:]]*\"(deny|forbid)\"" { ok = 1 }
      END { exit ok ? 0 : 1 }' "$win_toml"
  }
  for l in unsafe_code unsafe_op_in_unsafe_fn; do
    exige lints.rust "$l" || { echo "$win_toml: [lints.rust] $l precisa ser deny" >&2; falhou=1; }
  done
  for l in undocumented_unsafe_blocks multiple_unsafe_ops_per_block unwrap_used expect_used panic todo unimplemented dbg_macro; do
    exige lints.clippy "$l" || { echo "$win_toml: [lints.clippy] $l precisa ser deny" >&2; falhou=1; }
  done
fi

[ "$falhou" -eq 0 ] || exit 1
echo "unsafe isolado em ${ffi}: ok"
