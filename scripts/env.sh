#!/usr/bin/env sh
# Nexus canonical command environment (EP-003 M5, owner clarification).
# Source this file from repository entry scripts so every command runs with
# the same non-interactive environment COMMANDS.md documents AND can find the
# mise-managed locked toolchain from a fresh noninteractive shell without a
# manual PATH preamble.
#
# Idempotent: safe to source more than once. Does not modify any user global
# shell configuration; it only affects the sourcing shell's environment.

export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

# mise installs its launcher in ~/.local/bin and its tool shims in
# ~/.local/share/mise/shims. Prepend both when present and not already on
# PATH so rustc, cargo, node, pnpm, uv, flutter, tofu, age, sops,
# cargo-deny, and cargo-audit resolve from a clean noninteractive shell.
# The mise shims MUST precede ~/.local/bin so the mise-pinned version wins
# over any standalone binary (e.g. a self-installed uv) in ~/.local/bin.
_mise_shims="${MISE_SHIMS_DIR:-$HOME/.local/share/mise/shims}"
_mise_bin="$HOME/.local/bin"
_path_add=""
# Process the launcher bin first, then shims, so shims end up first in PATH.
for _d in "$_mise_bin" "$_mise_shims"; do
  if [ -d "$_d" ]; then
    case ":$PATH:" in
      *":$_d:"*) ;;
      *) _path_add="$_d:$_path_add" ;;
    esac
  fi
done
if [ -n "$_path_add" ]; then
  PATH="$_path_add$PATH"
fi
export PATH
unset _mise_shims _mise_bin _path_add _d
