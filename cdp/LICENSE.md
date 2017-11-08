# License

## The `cdp` crate itself

The `cdp` Rust crate is published under the
[MPL-2.0](https://github.com/devtools-html/rust-cdp/blob/master/cdp/LICENSE-MPL).

Helpful resources:

- [Mozilla's MPL-2.0 FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/)
- [MPL-2.0 on TLDRLegal](https://tldrlegal.com/license/mozilla-public-license-2.0-\(mpl-2\))

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be licensed as above, without any
additional terms or conditions.

## Chrome DevTools Protocol JSON

As part of its build process, the `cdp` crate automatically generates code using a 
[set of JSON files](https://github.com/devtools-html/rust-cdp/tree/master/json) published by the
Chromium project, which act as a machine-readable specification for the protocol. These JSON files
are covered by a separate
[BSD 3-Clause license](https://github.com/devtools-html/rust-cdp/blob/master/cdp/LICENSE-CHROMIUM).
