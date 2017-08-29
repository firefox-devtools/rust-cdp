// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![allow(deprecated)]
// Otherwise rustc warns that we're using our own deprecated types...

include!(concat!(env!("OUT_DIR"), "/tools_generated.rs"));
