#!/usr/bin/env bash
# Runs eh3d-vision with its sample. Extra arguments replace the sample: ./run.sh [ARGUMENTS]
set -u
cd "$(dirname "$0")"
pause() { [ -n "${NOPAUSE:-}" ] || { read -r -p "Press Enter to close..." _ || true; }; }
trap pause EXIT
if [ "$#" -eq 0 ]; then set -- align examples/design.csv examples/measured.csv; fi
cargo run --release --quiet -- "$@"
