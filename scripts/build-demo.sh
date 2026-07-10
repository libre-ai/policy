#!/usr/bin/env bash
# Build the public demo bundle (static files + WASM, local mode only).
# The demo ships an illustrative catalogue with zero Artificial Analysis
# data — see docs/deploy-demo.md before publishing anywhere.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
out_dir="${repo_root}/dist/demo"

(cd "${repo_root}/apps/web" && dx build --platform web --release)

rm -rf "${out_dir}"
mkdir -p "${out_dir}"
cp -R "${repo_root}/target/dx/rumble-ai-clearance-web-app/release/web/public/." "${out_dir}/"

echo "demo bundle ready: ${out_dir}"
echo "sanity: $(ls "${out_dir}" | tr '\n' ' ')"
