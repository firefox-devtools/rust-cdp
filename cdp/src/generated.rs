// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

// We carry over deprecation flags from the protocol definition JSON. Unless we
// disable this warning for this file, rustc complains that we're using our own
// deprecated types.
#![allow(deprecated)]

include!(concat!(env!("OUT_DIR"), "/generated.rs"));
