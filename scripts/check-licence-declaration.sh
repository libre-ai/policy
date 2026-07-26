#!/usr/bin/env bash
# Licence declaration control.
#
# Catches the one defect that every existing licence gate in this repository is
# blind to: a package manifest that announces a licence the code does not carry.
# `reuse lint` only asks whether every file *has* a licence, and `cargo deny`
# only asks whether the declared licence is on an allow-list. Both were green
# while `[workspace.package]` said `MIT` and every Rust source said `EUPL-1.2` —
# a manifest is what crates.io publishes and what a downstream reader relies on,
# so the gap between the two is a real grant of permissions that do not exist.
#
# Method: the effective licence is whatever `reuse spdx` says it is. This script
# never re-implements REUSE's path precedence — re-deriving it here would
# reproduce exactly the drift it is meant to detect. Workspace inheritance
# (`license.workspace = true`) is likewise resolved by `cargo metadata`, not by
# reading TOML, because the value lives in `[workspace.package]` and a naive
# per-crate read would see an absent field rather than the inherited one.
#
# Comparison rule, applied uniformly and to every package (no package is ever
# named or exempted here — a name-based carve-out would be an allow-list in
# disguise): a manifest's `license` field states the terms of the *code that
# package compiles*, so it is compared against the SPDX identifiers of the
# package's source files in its own language — `.rs` for Cargo, `.ts/.js` for
# npm. Non-code material under the same directory (test fixtures, vendored
# fonts, generated design-system assets, prose) is licensed at its own path
# scope by REUSE.toml, which is the mechanism designed to express exactly that;
# a single `license` field cannot restate a per-path map and should not try.
# Files are attributed to the longest matching package directory, so a nested
# package keeps its own files rather than donating them to its parent.
#
# A package whose source files do not agree on one identifier is not silently
# accepted: it is reported as HETEROGENEOUS and fails, for a human to adjudicate.
#
# Exit codes, kept distinct on purpose — "found nothing" and "could not look"
# must never be confused:
#   0  every examined package declares the licence its code carries
#   1  a real divergence (or a heterogeneous / undeclared publishable package)
#   2  could not search: a tool is missing, a step failed, or nothing was examined
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

readonly EXIT_DIVERGENCE=1
readonly EXIT_CANNOT_SEARCH=2

# Same pinned invocation as the "REUSE compliance" job in hygiene.yml: the
# machine doctrine bans `pip install`, and charset-normalizer gives the tool the
# encoding detection it needs on non-UTF-8 inputs.
readonly REUSE_PIN='reuse[charset-normalizer]==6.2.0'

cannot_search() {
  echo "::error::[licence-declaration] cannot search: $1"
  exit "$EXIT_CANNOT_SEARCH"
}

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

# --- preflight ---------------------------------------------------------------
# Checked before any work: a missing tool must report itself as "could not
# search", never as "nothing found". An absent command otherwise prints its help
# text and downstream `wc -l` cheerfully counts the usage lines as results.
for tool in cargo jq uvx git awk; do
  command -v "$tool" >/dev/null 2>&1 || cannot_search "\`$tool\` is not available"
done

# --- effective licences, straight from REUSE ---------------------------------
if ! uvx --from "$REUSE_PIN" reuse spdx >"$WORKDIR/spdx.txt" 2>"$WORKDIR/spdx.err"; then
  sed 's/^/  /' <"$WORKDIR/spdx.err" >&2
  cannot_search "\`reuse spdx\` failed"
fi

# SPDX tag-value: a FileName line, then the LicenseInfoInFile line(s) for it.
# LicenseConcluded is always NOASSERTION in reuse output and is deliberately
# ignored; NOASSERTION is dropped rather than compared.
awk '
  /^FileName:/          { f = substr($0, 11); sub(/^\.\//, "", f); next }
  /^LicenseInfoInFile:/ { if (f != "" && $2 != "NOASSERTION") print f "\t" $2 }
' "$WORKDIR/spdx.txt" | sort -u >"$WORKDIR/files.tsv"

if [ ! -s "$WORKDIR/files.tsv" ]; then
  cannot_search "\`reuse spdx\` returned no file/licence pair"
fi
echo "[licence-declaration] reuse spdx reported $(wc -l <"$WORKDIR/files.tsv" | tr -d ' ') file/licence pair(s)"

# --- declared licences, with workspace inheritance resolved by cargo ----------
if ! cargo metadata --no-deps --format-version 1 >"$WORKDIR/meta.json" 2>"$WORKDIR/meta.err"; then
  sed 's/^/  /' <"$WORKDIR/meta.err" >&2
  cannot_search "\`cargo metadata\` failed"
fi

# name, directory relative to the repo root, declared licence, kind, publishable.
# `.publish` is null when unrestricted and [] when `publish = false`.
jq -r --arg root "$PWD/" '
  .packages[]
  | ( .manifest_path | ltrimstr($root) | sub("Cargo\\.toml$"; "") | sub("/$"; "") ) as $d
  | [ .name, (if $d == "" then "." else $d end), (.license // ""), "cargo",
      (if .publish == null then "yes" else "no" end) ]
  | @tsv
' "$WORKDIR/meta.json" >"$WORKDIR/packages.tsv"

# npm packages get the same treatment; `private: true` is npm's `publish = false`.
git ls-files -- 'package.json' '*/package.json' >"$WORKDIR/npm.txt" || true
while IFS= read -r manifest; do
  [ -n "$manifest" ] || continue
  dir="$(dirname "$manifest")"
  jq -r --arg name "$(basename "$(dirname "$(pwd)/$manifest")")" --arg dir "$dir" '
    [ (.name // $name), $dir, (.license // ""), "npm",
      (if .private == true then "no" else "yes" end) ] | @tsv
  ' "$manifest" >>"$WORKDIR/packages.tsv" || cannot_search "could not parse $manifest"
done <"$WORKDIR/npm.txt"

PKG_COUNT="$(wc -l <"$WORKDIR/packages.tsv" | tr -d ' ')"
echo "[licence-declaration] examined $PKG_COUNT package manifest(s): \
$(awk -F'\t' '{print $4}' "$WORKDIR/packages.tsv" | sort | uniq -c | awk '{printf "%s %s ", $1, $2}')"

# The count is the control's own liveness proof. A renamed directory, a tightened
# filter or a workspace that stopped resolving would otherwise leave this script
# permanently and silently green.
if [ "$PKG_COUNT" -eq 0 ]; then
  cannot_search "examined 0 package manifests — the workspace or the filter moved and this control is inert"
fi

# --- compare -----------------------------------------------------------------
awk -F'\t' '
  function code_ext(kind, ext) {
    if (kind == "cargo") return (ext == "rs")
    return (ext == "ts" || ext == "tsx" || ext == "js" || ext == "mjs" || ext == "cjs")
  }
  NR == FNR {
    np++; pname[np] = $1; pdir[np] = $2; pdecl[np] = $3; pkind[np] = $4; ppub[np] = $5
    next
  }
  {
    path = $1; lic = $2
    ext = path; if (ext ~ /\./) sub(/^.*\./, "", ext); else ext = ""
    best = 0; bestlen = -1
    for (i = 1; i <= np; i++) {
      if (!code_ext(pkind[i], ext)) continue
      if (pdir[i] == ".") { pfx = ""; l = 0 } else { pfx = pdir[i] "/"; l = length(pfx) }
      if (substr(path, 1, l) == pfx && l > bestlen) { bestlen = l; best = i }
    }
    if (best == 0) next
    nfiles[best]++
    if (!((best SUBSEP lic) in seen)) { seen[best SUBSEP lic] = 1; ids[best] = ids[best] (ids[best] == "" ? "" : " AND ") lic; nids[best]++ }
  }
  END {
    status = 0
    for (i = 1; i <= np; i++) {
      decl = pdecl[i]; eff = ids[i]; n = nfiles[i] + 0
      label = sprintf("  %-32s %-12s", pname[i], pkind[i])
      if (n == 0) {
        printf "%s NO SOURCE FILE (dir=%s)\n", label, pdir[i]
        printf "::error::[licence-declaration] %s: no %s source file found — this package is not being checked\n", pname[i], pkind[i]
        status = 2
        continue
      }
      if (nids[i] > 1) {
        printf "%s HETEROGENEOUS  files=%d  effective={%s}  declared=%s\n", label, n, eff, (decl == "" ? "(none)" : decl)
        printf "::error::[licence-declaration] %s: its sources do not agree on one licence (%s); a single manifest field cannot state this — adjudicate it\n", pname[i], eff
        if (status != 2) status = 1
        continue
      }
      if (decl == "") {
        if (ppub[i] == "yes") {
          printf "%s UNDECLARED     files=%d  effective=%s  publishable=yes\n", label, n, eff
          printf "::error::[licence-declaration] %s: publishable but declares no licence, while its code is %s\n", pname[i], eff
          if (status != 2) status = 1
        } else {
          printf "%s no declaration files=%d  effective=%s  publishable=no (nothing announced to anyone)\n", label, n, eff
        }
        continue
      }
      if (decl != eff) {
        printf "%s DIVERGENCE     files=%d  declared=%s  effective=%s\n", label, n, decl, eff
        printf "::error::[licence-declaration] %s declares `%s` but its %d source file(s) are licensed %s\n", pname[i], decl, n, eff
        if (status != 2) status = 1
        continue
      }
      printf "%s ok             files=%d  declared=effective=%s\n", label, n, decl
    }
    exit status
  }
' "$WORKDIR/packages.tsv" "$WORKDIR/files.tsv" || RESULT=$?
RESULT="${RESULT:-0}"

if [ "$RESULT" -eq 2 ]; then
  exit "$EXIT_CANNOT_SEARCH"
fi
if [ "$RESULT" -ne 0 ]; then
  echo "::error::[licence-declaration] a manifest announces a licence its code does not carry"
  exit "$EXIT_DIVERGENCE"
fi

echo "[licence-declaration] OK: every examined manifest declares the licence its sources carry."
