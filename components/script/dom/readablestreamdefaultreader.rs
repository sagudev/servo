/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::dom::bindings::codegen::Bindings::ReadableStreamDefaultReaderBinding::ReadableStreamDefaultReaderMethods;
use crate::dom::bindings::reflector::{reflect_dom_object, DomObject, Reflector};
use dom_struct::dom_struct;

use super::bindings::error::Fallible;
use super::bindings::root::DomRoot;
use super::readablestream::ReadableStream;
use super::types::GlobalScope;
#[dom_struct]
pub struct ReadableStreamDefaultReader {
    reflector_: Reflector,
    /// A list of read requests, used when a consumer requests chunks sooner than they are available
    read_requests: todo!(),
}

impl ReadableStreamDefaultReaderMethods for ReadableStreamDefaultReader {
    fn Read(&self) -> super::bindings::error::Fallible<std::rc::Rc<super::promise::Promise>> {
        todo!()
    }

    fn ReleaseLock(&self) -> super::bindings::error::Fallible<()> {
        todo!()
    }

    fn Closed(&self) -> std::rc::Rc<super::promise::Promise> {
        todo!()
    }

    fn Cancel(
        &self,
        cx: crate::script_runtime::JSContext,
        reason: js::rust::HandleValue,
    ) -> super::bindings::error::Fallible<std::rc::Rc<super::promise::Promise>> {
        todo!()
    }
}

#[allow(non_snake_case)]
impl ReadableStreamDefaultReader {
    //https://streams.spec.whatwg.org/#default-reader-constructor
    pub fn Constructor(global: &GlobalScope, stream: &ReadableStream) -> Fallible<DomRoot<Self>> {
        todo!()
    }
}
