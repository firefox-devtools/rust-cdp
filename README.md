# cdp *(unstable; unreleased)*

> Chrome DevTools Protocol types and (de)serialization for Rust

~~[![Crates.io](https://img.shields.io/crates/v/cdp.svg)](https://crates.io/crates/cdp)~~
[![Linux/OSX Build Status](https://img.shields.io/travis/devtools-html/cdp/master.svg)](https://travis-ci.org/devtools-html/cdp)
[![Windows Build Status](https://img.shields.io/appveyor/ci/spinda/cdp/master.svg)](https://ci.appveyor.com/project/spinda/cdp)

[Documentation](https://www.spinda.net/files/mozilla/cdp/doc/cdp/index.html)

## Usage

First, add this to your `Cargo.toml`:

```toml
[dependencies.cdp]
git = "https://github.com/devtools-html/cdp"
```

Next, add this to your crate:

```rust
extern crate cdp;
```

## License

Please see [LICENSE.md](/LICENSE.md).

## Maintainers

The `update_json.sh` script can be used to automatically identify the latest Chrome stable release
and download the correspondong Chrome DevTools Protocol specification JSON files. Ideally this
crate should track the Chrome stable release cycle.

During development, this crate is configured to automatically generate code from these JSON files
as part of the build process. For a release, however, this generated code should be "baked" into
the published artifact, saving time for end users and freeing them from having to install the build
dependencies. The `bake.sh` script will perform this task, placing a ready-for-release version of
this crate in the `target/bake/` directory.
