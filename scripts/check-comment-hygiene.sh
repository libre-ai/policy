#!/usr/bin/env bash
# Comment hygiene control.
#
# Scope, stated honestly: only three properties of a comment are syntactic
# enough to be decided by a machine. Whether a comment is true, and whether it
# explains a "why" rather than restating the "what", is judgement — this
# control does not pretend to cover it and must not be read as if it did.
#
#   1. commented-out code    — a statement that was disabled instead of deleted
#   2. unreferenced markers  — a to-do with no owner, no context, no issue
#   3. copy-pasted comments  — the same text repeated in three or more places
#
# Like the design-system control, it reports how many files it scanned and
# fails when that number is zero: a renamed directory or a tightened filter
# would otherwise leave the whole control inert and permanently green.
#
# The marker words are assembled from fragments so this file does not trip its
# own check — same idiom as the "No machine-local absolute paths" step.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# Files with comment syntax. Lockfiles, licence texts, fixtures and binaries
# carry no first-party commentary and are not scanned.
FILES="$(git ls-files -- '*.rs' '*.ts' '*.css' '*.sh' '*.yml' '*.yaml')"
COUNT=0
if [ -n "$FILES" ]; then
  COUNT="$(printf '%s\n' "$FILES" | wc -l | tr -d ' ')"
fi

printf '[comment-hygiene] scanned %s file(s):' "$COUNT"
for ext in rs ts css sh yml yaml; do
  n="$(printf '%s\n' "$FILES" | grep -c "\\.${ext}\$" || true)"
  printf ' %s .%s' "$n" "$ext"
done
printf '\n'

if [ "$COUNT" -eq 0 ]; then
  echo "::error::scanned 0 files: the filter or the tree moved and this control is inert"
  exit 1
fi

SCANNED=()
while IFS= read -r file; do
  [ -n "$file" ] && SCANNED+=("$file")
done <<EOF
$FILES
EOF

FAILED=0

# --- comment text extraction ------------------------------------------------
# Rust/TS: `//` comments, excluding `///` and `//!` (public documentation, not
# internal commentary) and excluding the `//` inside a URL. CSS/shell/YAML:
# the leading `/*` or `#` form; a shebang is not a comment.
extract() {
  awk '
    {
      ext = FILENAME; sub(/^.*\./, "", ext)
      line = $0; sub(/^[ \t]+/, "", line)
      text = ""; leading = 0
      if (ext == "rs" || ext == "ts") {
        if (line ~ /^\/\/[^\/!]/ || line == "//") { text = substr(line, 3); leading = 1 }
        else if (match(line, /[^:\/]\/\/[^\/!]/)) { text = substr(line, RSTART + 3) }
      } else if (ext == "css") {
        if (line ~ /^\/\*/) { text = substr(line, 3) }
        else if (line ~ /^\*[^\/]/) { text = substr(line, 2) }
      } else if (line ~ /^#[^!]/) {
        text = substr(line, 2)
      }
      if (text == "") next
      gsub(/\*\//, "", text)
      gsub(/^[ \t]+|[ \t]+$/, "", text)
      if (text == "") next
      print FILENAME ":" FNR ":" leading ":" text
    }
  ' "$@"
}

COMMENTS="$(extract "${SCANNED[@]}")"
COMMENT_COUNT=0
if [ -n "$COMMENTS" ]; then
  COMMENT_COUNT="$(printf '%s\n' "$COMMENTS" | wc -l | tr -d ' ')"
fi
echo "[comment-hygiene] extracted $COMMENT_COUNT comment line(s)"

# The file count alone does not prove the control ran: a broken extractor would
# scan every file, read nothing, and pass all three checks in silence.
if [ "$COMMENT_COUNT" -eq 0 ]; then
  echo "::error::extracted 0 comments from $COUNT file(s): the extractor is broken"
  exit 1
fi

# --- 1. commented-out code --------------------------------------------------
# Rust and TypeScript only. A disabled statement both opens with a declaration
# keyword and closes like code; prose does neither. CSS is excluded on purpose:
# the design-system control already strips `/* ... */` so that commented-out
# style examples stay legitimate there.
KEYWORDS='(let|const|var|fn|function|use|impl|struct|enum|mod|pub|static|return|import|export|println!|eprintln!|dbg!|panic!|assert!|assert_eq!|assert_ne!)'
DEAD="$(printf '%s\n' "$COMMENTS" \
  | grep -E "^[^:]+\.(rs|ts):[0-9]+:1:${KEYWORDS}([^A-Za-z]|\$)" \
  | grep -E '[;{}][ \t]*$' || true)"
if [ -n "$DEAD" ]; then
  printf '%s\n' "$DEAD" | sed 's/^/  /'
  echo "::error::commented-out code: delete it — git keeps the history, this file does not"
  FAILED=1
fi

# --- 2. unreferenced markers ------------------------------------------------
# A marker earns its place when it names a context, an issue or a date; without
# one it is an orphan nobody can act on. Searched over the whole file, not only
# its comments, so a marker hidden in a string is caught too.
M1="TO"; M1="${M1}DO"
M2="FIX"; M2="${M2}ME"
M3="HA"; M3="${M3}CK"
M4="XX"; M4="${M4}X"
MARKER="(${M1}|${M2}|${M3}|${M4})"
MARKED="$(grep -nE "$MARKER" "${SCANNED[@]}" || true)"
ORPHANS="$(printf '%s\n' "$MARKED" \
  | grep -vE "${MARKER}\(" \
  | grep -vE '#[0-9]+' \
  | grep -vE '20[0-9][0-9]-[0-9][0-9]-[0-9][0-9]' \
  | grep -vE '^[ \t]*$' || true)"
if [ -n "$ORPHANS" ]; then
  printf '%s\n' "$ORPHANS" | sed 's/^/  /'
  echo "::error::marker without a context, an issue reference or a date"
  FAILED=1
fi

# --- 3. copy-pasted comments ------------------------------------------------
# The threshold is three, not two, because two identical justifications on two
# sibling declarations are a legitimate shape; three or more say the
# explanation belongs in one place and now sits in three that can drift apart.
# Short texts are ignored: separators and one-word labels repeat harmlessly.
DUPES="$(printf '%s\n' "$COMMENTS" \
  | sed -E 's/^[^:]+:[0-9]+:[01]://' \
  | awk 'length($0) >= 24 && NF >= 4' \
  | sort \
  | uniq -c \
  | awk '$1 >= 3' || true)"
if [ -n "$DUPES" ]; then
  printf '%s\n' "$DUPES" | sed 's/^/  /'
  echo "::error::the same comment in 3+ places: state it once, reference it from the others"
  FAILED=1
fi

if [ "$FAILED" -ne 0 ]; then
  exit 1
fi

echo "[comment-hygiene] OK: no commented-out code, no orphan marker, no triplicated comment."
