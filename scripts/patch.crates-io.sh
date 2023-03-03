#!/usr/bin/env bash
#
# Patches the SPL crates for developing against a local safecoin monorepo
#

here="$(dirname "$0")"

solana_dir=$1
if [[ -z $solana_dir ]]; then
  echo "Usage: $0 <path-to-solana-monorepo>"
  exit 1
fi

workspace_crates=(
  "$here"/../Cargo.toml
)

if [[ ! -r "$solana_dir"/scripts/read-cargo-variable.sh ]]; then
  echo "$solana_dir is not a path to the safecoin monorepo"
  exit 1
fi

set -e

solana_dir=$(cd "$solana_dir" && pwd)

source "$solana_dir"/scripts/read-cargo-variable.sh
solana_ver=$(readCargoVariable version "$solana_dir"/sdk/Cargo.toml)

echo "Patching in $solana_ver from $solana_dir"

if ! git diff --quiet && [[ -z $DIRTY_OK ]]; then
  echo "Error: dirty tree"
  exit 1
fi
export DIRTY_OK=1

for crate in "${workspace_crates[@]}"; do
  if grep -q '\[patch.crates-io\]' "$crate"; then
    echo "* $crate is already patched"
  else
    echo "* patched $crate"
    cat >> "$crate" <<PATCH
[patch.crates-io]
safecoin-clap-utils = {path = "$solana_dir/clap-utils" }
safecoin-cli-config = {path = "$solana_dir/cli-config" }
safecoin-client = { path = "$solana_dir/client"}
safecoin-logger = { path = "$solana_dir/logger"}
safecoin-program = { path = "$solana_dir/sdk/program" }
safecoin-program-test = { path = "$solana_dir/program-test" }
safecoin-remote-wallet = { path = "$solana_dir/remote-wallet"}
safecoin-sdk = { path = "$solana_dir/sdk" }
safecoin-validator = { path = "$solana_dir/validator"}
PATCH
  fi
done

"$here"/update-solana-dependencies.sh "$solana_ver"

