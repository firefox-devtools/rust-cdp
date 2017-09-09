#!/bin/bash

# This Source Code Form is subject to the terms of the Mozilla Public License,
# v. 2.0. If a copy of the MPL was not distributed with this file, You can
# obtain one at http://mozilla.org/MPL/2.0/.

set -eufo pipefail

# Do a fresh build to ensure that all the source files we'll need are generated.
if [[ "${1:-}" != "--skip-build" ]]; then
  cargo clean -p cdp
  cargo build
fi

# Initialize the output directory.
rm -rf target/bake/
mkdir -p target/bake/

function find_most_recent {
  find . -type f -name "${1}" -printf "%T+\t%p\n" | sort -r | head -n1 | cut -f2
}

# Find the generated source files.
constants_rs="$(find_most_recent 'constants.rs')"
proto_generated_rs="$(find_most_recent 'proto_generated.rs')"

# Set up the baked Cargo.toml.
sed '/build/d' Cargo.toml > target/bake/Cargo.toml

# Copy over the relevant files.
cp LICENSE.md target/bake/
cp LICENSE-MPL target/bake/
cp LICENSE-CHROMIUM target/bake/
cp README.md target/bake/
cp -r src/ target/bake/
cp -r tests/ target/bake/
cp .gitignore target/bake/
cp rustfmt.toml target/bake/
cp appveyor.yml target/bake/
sed '/sh/d' .travis.yml > target/bake/.travis.yml

# Bake in the generated source.
sed -i '/include/d' target/bake/src/lib.rs
cat "${constants_rs}" >> target/bake/src/lib.rs

sed -i '/include/d' target/bake/src/proto_generated.rs
echo "
// The following code is automatically generated using a JSON specification of
// the Chrome DevTools Protocol published by the Chromium project. The JSON
// files themselves are covered by their own BSD 3-Clause license. Please see
// the LICENSE-CHROMIUM file for more information.
" >> target/bake/src/proto_generated.rs
cat "${proto_generated_rs}" >> target/bake/src/proto_generated.rs

echo
echo
echo
find target/bake/ -type f
