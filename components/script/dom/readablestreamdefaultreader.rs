/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::rc::Rc;

use crate::dom::bindings::codegen::Bindings::ReadableStreamDefaultReaderBinding::{
    ReadableStreamDefaultReaderMethods, ReadableStreamReadResult,
};
use crate::dom::bindings::error::Error;
use crate::dom::bindings::reflector::{reflect_dom_object, DomObject, Reflector};
use crate::script_runtime::JSContext;
use dom_struct::dom_struct;
use js::conversions::ToJSValConvertible;
use js::jsapi::{JS_ClearPendingException, JS_WrapValue};
use js::jsval::UndefinedValue;
use js::rust::HandleValue;

use super::bindings::error::Fallible;
use super::promise::Promise;
use super::readablestream::ReadableStream;
use super::types::GlobalScope;
use crate::dom::bindings::cell::DomRefCell;
use crate::dom::bindings::root::{Dom, DomRoot};

// is this dyn dispatched?
trait ReadRequest {
    /// An algorithm taking a chunk, called when a chunk is available for reading close steps
    fn chunk_steps(&self, cx: JSContext, chunk: HandleValue) -> Fallible<()>;
    /// An algorithm taking no arguments, called when no chunks are available because the stream is closed error steps
    fn close_steps(&self, cx: JSContext) -> Fallible<()>;
    /// An algorithm taking a JavaScript value, called when no chunks are available because the stream is errored
    fn error_steps(&self, cx: JSContext, chunk: HandleValue) -> Fallible<()>;
}

#[derive(JSTraceable, MallocSizeOf)]
// https://streams.spec.whatwg.org/#default-reader-read
struct Read_ReadRequest {
    promise: Promise,
}

impl ReadRequest for Read_ReadRequest {
    fn chunk_steps(&self, cx: JSContext, chunk: HandleValue) -> Fallible<()> {
        // https://streams.spec.whatwg.org/#default-reader-read Step 3.
        // chunk steps, given chunk:
        //  Step 1. Resolve promise with «[ "value" → chunk, "done" → false ]».

        // Value may need to be wrapped if stream and reader are in different
        // compartments.
        rooted!(in(*cx) let chunked = chunk.get());
        #[allow(unsafe_code)]
        unsafe {
            if !JS_WrapValue(*cx, chunked.handle_mut().into()) {
                //JS_ClearPendingException(*cx);
                return Err(Error::JSFailed);
            }
        }

        let result = ReadableStreamReadResult::empty();
        result.value.set(chunked.get());
        result.done = Some(false);

        // Ensure that the object is created with the current global.
        rooted!(in(*cx) let mut value = UndefinedValue());
        #[allow(unsafe_code)]
        unsafe {
            result.to_jsval(*cx, value.handle_mut())
        };

        self.promise.resolve(cx, value.handle());
        Ok(())
    }

    fn close_steps(&self, cx: JSContext) -> Fallible<()> {
        // https://streams.spec.whatwg.org/#default-reader-read Step 3.
        // close steps:
        //  Step 1. Resolve promise with «[ "value" → undefined, "done" → true ]».
        let result = ReadableStreamReadResult::empty();
        result.done = Some(true);

        rooted!(in(*cx) let mut value = UndefinedValue());
        #[allow(unsafe_code)]
        unsafe {
            result.to_jsval(*cx, value.handle_mut())
        };

        self.promise.resolve(cx, value.handle());
        Ok(())
    }

    fn error_steps(&self, cx: JSContext, e: HandleValue) -> Fallible<()> {
        // https://streams.spec.whatwg.org/#default-reader-read Step 3.
        // error steps:
        //  Step 1. Reject promise with e.
        self.promise.reject(cx, e);
        Ok(())
    }
}

#[dom_struct]
pub struct ReadableStreamDefaultReader {
    reflector_: Reflector,
    /// A list of read requests, used when a consumer requests chunks sooner than they are available
    read_requests: Read_ReadRequest,
    // from ReadableStreamGenericReader mixin
    //[[closedPromise]] 	A promise returned by the reader’s closed getter
    //[[stream]]
}

impl ReadableStreamDefaultReaderMethods for ReadableStreamDefaultReader {
    fn Read(&self) -> Fallible<Rc<Promise>> {
        todo!()
    }

    fn ReleaseLock(&self) -> Fallible<()> {
        todo!()
    }

    fn Closed(&self) -> Rc<Promise> {
        todo!()
    }

    fn Cancel(&self, cx: JSContext, reason: HandleValue) -> Fallible<Rc<Promise>> {
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
