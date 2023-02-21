/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::dom::bindings::codegen::Bindings::ReadableStreamDefaultControllerBinding::ReadableStreamDefaultControllerMethods;
use crate::dom::bindings::reflector::{reflect_dom_object, DomObject, Reflector};
use dom_struct::dom_struct;
#[dom_struct]
pub struct ReadableStreamDefaultController {
    reflector_: Reflector,
}

impl ReadableStreamDefaultControllerMethods for ReadableStreamDefaultController {
    fn GetDesiredSize(&self) -> Option<f64> {
        todo!()
    }

    fn Close(&self) -> super::bindings::error::Fallible<()> {
        todo!()
    }

    fn Enqueue(
        &self,
        cx: crate::script_runtime::JSContext,
        chunk: js::rust::HandleValue,
    ) -> super::bindings::error::Fallible<()> {
        todo!()
    }

    fn Error(
        &self,
        cx: crate::script_runtime::JSContext,
        e: js::rust::HandleValue,
    ) -> super::bindings::error::Fallible<()> {
        todo!()
    }
}
