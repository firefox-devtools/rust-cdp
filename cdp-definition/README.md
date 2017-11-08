# cdp-definition *(unstable; unreleased)*

> Types and (de)serialization for the Chrome DevTools Protocol definition JSON

<!-- [![Crates.io](https://img.shields.io/crates/v/cdp-definition.svg)](https://crates.io/crates/cdp-definition) -->
[![Linux/OSX Build Status](https://img.shields.io/travis/devtools-html/rust-cdp/master.svg)](https://travis-ci.org/devtools-html/rust-cdp)
[![Windows Build Status](https://img.shields.io/appveyor/ci/spinda/rust-cdp/master.svg)](https://ci.appveyor.com/project/spinda/rust-cdp)

[Documentation](https://www.spinda.net/files/mozilla/rust-cdp/doc/cdp_definition/index.html)

## Usage

First, add this to your `Cargo.toml`:

```toml
[dependencies.cdp-definition]
git = "https://github.com/devtools-html/rust-cdp"
```

Next, add this to your crate:

```rust
extern crate cdp_definition;

use cdp_definition::Definition;
```

## License

[MPL-2.0](/LICENSE)

Helpful resources:

- [Mozilla's MPL-2.0 FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/)
- [MPL-2.0 on TLDRLegal](https://tldrlegal.com/license/mozilla-public-license-2.0-\(mpl-2\))

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be licensed as above, without any
additional terms or conditions.
