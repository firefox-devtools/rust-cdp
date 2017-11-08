#!/bin/bash

# This Source Code Form is subject to the terms of the Mozilla Public License,
# v. 2.0. If a copy of the MPL was not distributed with this file, You can
# obtain one at http://mozilla.org/MPL/2.0/.

set -eufo pipefail

input="cdp"
output="cdp-baked"

# Do a fresh build to ensure that all the source files we'll need are generated.
if [[ "${1:-}" != "--skip-build" ]]; then
  cargo clean -p "${input}"
  cargo build -p "${input}"
fi

# Find the generated sources.
generated_rs="$(find target/ -type f -name generated.rs -printf "%T+\t%p\n" | sort -r | head -n1 | cut -f2)"

# Initialize the output directory.
rm -rf "${output}/"
mkdir -p "${output}/"

# Set up the baked Cargo.toml.
sed '/build/d' "${input}/Cargo.toml" > "${output}/Cargo.toml"

# Copy over the relevant files.
cp "${input}/LICENSE.md" "${output}/"
cp "${input}/LICENSE-MPL" "${output}/"
cp "${input}/LICENSE-CHROMIUM" "${output}/"
cp "${input}/README.md" "${output}/"
mkdir "${output}/src/"
cp "${input}/src/lib.rs" "${output}/src/"
cp "${input}/src/generated.rs" "${output}/src/"
cp -r "${input}/tests/" "${output}/"

# Bake in the generated source.
sed -i '/include/d' "${output}/src/generated.rs"
echo "
// The following code is automatically generated using a JSON specification of
// the Chrome DevTools Protocol published by the Chromium project. The JSON
// files themselves are covered by their own BSD 3-Clause license. Please see
// the LICENSE-CHROMIUM file for more information.
" >> "${output}/src/generated.rs"
cat "${generated_rs}" >> "${output}/src/generated.rs"

echo
echo
echo
find "${output}" -type f
