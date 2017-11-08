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

use cdp::{page, DeserializeCdpEvent};
use serde_json::{Map, Number, Value};

#[derive(DeserializeCdpEvent, Debug, PartialEq)]
enum Foo {
    #[cdp = "Page.domContentEventFired"]
    PageDomContentEventFired {
        params: page::DomContentEventFiredEvent,
    },
    #[cdp = "Page.loadEventFired"]
    PageLoadEventFired(page::LoadEventFiredEvent),
    #[cdp = "Page.interstitialHidden"]
    PageInterstitialHidden,
    #[cdp = "Quux.zot"]
    QuuxZot(page::InterstitialHiddenEvent),
    Other(String, Value),
}

#[test]
fn test() {
    let json_dom_event = r#"{"timestamp":12.7}"#;
    let rust_dom_event = Foo::PageDomContentEventFired {
        params: page::DomContentEventFiredEvent { timestamp: 12.7 },
    };

    let json_load_event = r#"{"timestamp":12.7}"#;
    let rust_load_event = Foo::PageLoadEventFired(page::LoadEventFiredEvent { timestamp: 12.7 });

    let rust_other = Foo::Other(
        "Bar.baz".into(),
        Value::Object({
            let mut params = Map::with_capacity(1);
            params.insert("timestamp".into(), Value::Number(Number::from_f64(12.7).unwrap()));
            params
        }),
    );

    {
        let mut deserializer = serde_json::Deserializer::from_str(json_dom_event);
        let result = Foo::deserialize_event("Page.domContentEventFired", &mut deserializer);
        assert_eq!(result.ok().map(Result::ok), Some(Some(rust_dom_event)));
    }

    {
        let mut deserializer = serde_json::Deserializer::from_str(json_load_event);
        let result = Foo::deserialize_event("Page.loadEventFired", &mut deserializer);
        assert_eq!(result.ok().map(Result::ok), Some(Some(rust_load_event)));
    }

    {
        let mut deserializer = serde_json::Deserializer::from_str("{}");
        let result = Foo::deserialize_event("Page.interstitialHidden", &mut deserializer);
        assert_eq!(result.ok().map(Result::ok), Some(Some(Foo::PageInterstitialHidden)));
    }

    {
        let mut deserializer = serde_json::Deserializer::from_str("{}");
        let result = Foo::deserialize_event("Quux.zot", &mut deserializer);
        assert_eq!(
            result.ok().map(Result::ok),
            Some(Some(Foo::QuuxZot(page::InterstitialHiddenEvent)))
        );
    }

    {
        let mut deserializer = serde_json::Deserializer::from_str(json_dom_event);
        let result = Foo::deserialize_event("Bar.baz", &mut deserializer);
        assert_eq!(result.ok().map(Result::ok), Some(Some(rust_other)));
    }
}
