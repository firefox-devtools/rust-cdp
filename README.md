# rust-cdp

> Chrome DevTools Protocol toolkit for Rust

[![Linux/OSX Build Status](https://img.shields.io/travis/devtools-html/rust-cdp/master.svg)](https://travis-ci.org/devtools-html/rust-cdp)
[![Windows Build Status](https://img.shields.io/appveyor/ci/spinda/rust-cdp/master.svg)](https://ci.appveyor.com/project/spinda/rust-cdp)

## Contents

This repository houses the source for the following Rust libraries, each of which has its own
README:

- [`cdp`](/cdp):
  Chrome DevTools Protocol types and (de)serialization for Rust
- [`cdp-definition`](/cdp-definition):
  Types and (de)serialization for the Chrome DevTools Protocol definition JSON
- [`cdp-derive`](/cdp-derive):
  Derive traits from the `cdp` library
- [`tokio-cdp`](/tokio-cdp):
  Chrome DevTools Protocol client/server library for Rust/Tokio

## Maintainers

The project should ideally track the Chrome stable release cycle with regard to the version of the
protocol it supports.

The `update-json.sh` script can be used to automatically identify the latest Chrome stable release
and download the correspondong Chrome DevTools Protocol specification JSON files.

During development, the `cdp` crate is configured to automatically generate code from these JSON
files as part of the build process. For a release, however, this generated code should be "baked"
into the published artifact, saving time for end users and freeing them from having to install the
build dependencies. The `bake.sh` script will perform this task, placing a ready-for-release
version of the crate in the `cdp-baked` directory.
