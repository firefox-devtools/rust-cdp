# cdp-derive *(unstable; unreleased)*

> Derive traits from the [`cdp`](https://github.com/devtools-html/cdp) library
> for the Chrome DevTools Protocol

<!-- [![Crates.io](https://img.shields.io/crates/v/cdp-derive.svg)](https://crates.io/crates/cdp-derive) -->
[![Linux/OSX Build Status](https://img.shields.io/travis/devtools-html/rust-cdp/master.svg)](https://travis-ci.org/devtools-html/rust-cdp)
[![Windows Build Status](https://img.shields.io/appveyor/ci/spinda/rust-cdp/master.svg)](https://ci.appveyor.com/project/spinda/rust-cdp)

[Documentation](https://www.spinda.net/files/mozilla/rust-cdp/doc/cdp_derive/index.html)

## Usage

First, add this to your `Cargo.toml`:

```toml
[dependencies.cdp-derive]
git = "https://github.com/devtools-html/rust-cdp"
```

The generated code depends on the
[`cdp`](https://github.com/devtools-html/rust-cdp/tree/master/cdp) and
[`serde`](https://crates.io/crates/serde) crates, so add these to your `Cargo.toml` as well to
avoid compile errors:

```toml
[dependencies]
serde = "1.0"

[dependencies.cdp]
git = "https://github.com/devtools-html/rust-cdp"
```

Next, add this to your crate:

```rust
#[macro_use]
extern crate cdp_derive;
```

Now you can derive `DeserializeCdpCommand` and `DeserializeCdpEvent` *(on enum definitions
only)*.

```r
use cdp;
use serde_json::{Map, Value};

#[derive(DeserializeCdpCommand)]
enum MyCdpCommand {
    // Each (struct or tuple) variant should contain exactly one field which
    // implements `cdp::CdpCommand + serde::Deserialize`.
    PageNavigate(cdp::page::NavigateCommand),
    PageCaptureScreenshot { params: cdp::page::CaptureScreenshotCommand },

    // The #[cdp = "..."] attribute can be used to explicitly specify a command
    // or event name. Then the field type need only implement
    // `serde::Deserialize`.
    #[cdp = "DOM.getDocument"]
    DomGetDocument(Map<String, Value>),

    // Unit variants can also be used, if annotated with an explicit
    // command/event name via the #[cdp = "..."] attribute. The deserialization
    // will ignore any parameters passed in.
    #[cdp = "Page.enable"]
    PageEnable,

    // An optional "wildcard" variant can be placed as the *last* variant in the
    // enum, which matches any command. It should contain exactly two fields,
    // the first of which must implement `From<&str>` and the second
    // `serde::Deserialize`.
    Other {
        name: String,
        params: Map<String, Value>,
    }
}
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
