// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate cdp;
extern crate serde_json;

#[macro_use]
extern crate cdp_derive;

use cdp::{page, DeserializeCdpCommand};

#[derive(DeserializeCdpCommand, Debug, PartialEq)]
#[cfg_attr(feature = "clippy", allow(enum_variant_names))]
enum Foo<'a> {
    #[cdp = "Page.navigate"]
    PageNavigate { params: page::NavigateCommand<'a> },
    #[cdp = "Page.captureScreenshot"]
    PageCaptureScreenshot(page::CaptureScreenshotCommand),
    #[cdp = "Page.enable"]
    PageEnable,
    #[cdp = "Page.disable"]
    PageDisable(page::EnableCommand),
}

#[test]
fn test() {
    let json_navigate = r#"{"url":"https://www.mozilla.org"}"#;
    let rust_navigate = Foo::PageNavigate {
        params: page::NavigateCommand {
            url: "https://www.mozilla.org".into(),
            referrer: None,
            transition_type: None,
        },
    };

    let json_capture_screenshot = r#"{"format":"png"}"#;
    let rust_capture_screenshot = Foo::PageCaptureScreenshot(page::CaptureScreenshotCommand {
        format: Some(page::CaptureScreenshotCommandFormat::Png),
        quality: None,
        from_surface: None,
        clip: None,
    });

    {
        let mut deserializer = serde_json::Deserializer::from_str(json_navigate);
        let result = Foo::deserialize_command("Page.navigate", &mut deserializer);
        assert_eq!(result.ok().map(Result::ok), Some(Some(rust_navigate)));
    }

    {
        let mut deserializer = serde_json::Deserializer::from_str(json_capture_screenshot);
        let result = Foo::deserialize_command("Page.captureScreenshot", &mut deserializer);
        assert_eq!(result.ok().map(Result::ok), Some(Some(rust_capture_screenshot)));
    }

    {
        let mut deserializer = serde_json::Deserializer::from_str("{}");
        let result = Foo::deserialize_command("Page.enable", &mut deserializer);
        assert_eq!(result.ok().map(Result::ok), Some(Some(Foo::PageEnable)));
    }

    {
        let mut deserializer = serde_json::Deserializer::from_str("{}");
        let result = Foo::deserialize_command("Page.disable", &mut deserializer);
        assert_eq!(result.ok().map(Result::ok), Some(Some(Foo::PageDisable(page::EnableCommand))));
    }

    {
        let mut deserializer = serde_json::Deserializer::from_str(json_navigate);
        let result = Foo::deserialize_command("Bar.baz", &mut deserializer);
        assert!(result.is_err());
    }
}
