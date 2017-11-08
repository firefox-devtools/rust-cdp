#!/bin/bash

# This Source Code Form is subject to the terms of the Mozilla Public License,
# v. 2.0. If a copy of the MPL was not distributed with this file, You can
# obtain one at http://mozilla.org/MPL/2.0/.

set -eufo pipefail

# The easiest way to determine the exact stable Chromium version, at least as
# far as I've found, is to grep through the "Stable updates" tag on the Chrome
# Releases blog for the link to the changelog that's included in every release
# post. Filter out the Chrome for Android results which contain
# "play.google.com" links.
echo "Determining stable Chromium version..."
echo
CHROMIUM_STABLE="$(curl 'https://chromereleases.googleblog.com/search/label/Stable%20updates' \
  | grep 'https://chromium.googlesource.com/chromium/src/+log/' \
  | grep -v 'play.google.com' \
  | tac \
  | tail -1 \
  | sed 's/.\++log\/[0-9]\+\.[0-9]\+\.[0-9]\+\.[0-9]\+\.\.\([0-9]\+\.[0-9]\+\.[0-9]\+\.[0-9]\+\).\+/\1/')"
CHROMIUM_BRANCH="https://chromium.googlesource.com/chromium/src/+/${CHROMIUM_STABLE}"
echo
echo "Found: ${CHROMIUM_STABLE}"
echo
echo

# Then we need to determine which v8 revision that version of Chrome uses. Note
# the use of "base64 --decode": Google's web-based git interface (Gitiles) only
# provides raw blob downloads in base64 format, for some reason.
echo "Determining v8 revision from Chromium ${CHROMIUM_STABLE}..."
echo
V8_REVISION="$(curl "${CHROMIUM_BRANCH}/DEPS?format=TEXT" \
  | base64 --decode \
  | grep -A1 v8_revision \
  | tail -1 \
  | sed "s/[ ']//g")"
V8_BRANCH="https://chromium.googlesource.com/v8/v8/+/${V8_REVISION}"
echo
echo "Found: ${V8_REVISION}"
echo
echo

echo "Downloading browser_protocol.json from Chromium ${CHROMIUM_STABLE}..."
echo
curl "${CHROMIUM_BRANCH}/third_party/WebKit/Source/core/inspector/browser_protocol.json?format=TEXT" \
  | base64 --decode >json/browser_protocol.json
echo
echo

echo "Downloading js_protocol.json from v8 revision ${V8_REVISION}..."
echo
curl "${V8_BRANCH}/src/inspector/js_protocol.json?format=TEXT" \
  | base64 --decode >json/js_protocol.json
echo
echo

# Grab the LICENSE file that goes with the Chrome DevTools Protocol JSON, for
# good measure.
echo "Downloading LICENSE-CHROMIUM file from GitHub..."
echo
curl 'https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol/master/LICENSE' >json/LICENSE-CHROMIUM.new
echo
( git diff --no-index -- json/LICENSE-CHROMIUM json/LICENSE-CHROMIUM.new \
  && echo 'LICENSE-CHROMIUM file matches our previous copy.' ) || true
mv json/LICENSE-CHROMIUM.new json/LICENSE-CHROMIUM
cp json/LICENSE-CHROMIUM cdp/
