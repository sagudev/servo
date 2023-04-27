/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::dom::bindings::conversions::{ConversionBehavior, ConversionResult};
use crate::dom::bindings::error::Error;
use crate::dom::bindings::reflector::{reflect_dom_object, DomObject, Reflector};
use crate::dom::bindings::root::DomRoot;
use crate::dom::bindings::settings_stack::{AutoEntryScript, AutoIncumbentScript};
use crate::dom::bindings::utils::get_dictionary_property;
use crate::dom::globalscope::GlobalScope;
use crate::dom::promise::Promise;
use crate::js::conversions::FromJSValConvertible;
use crate::realms::{enter_realm, InRealm};
use crate::script_runtime::JSContext as SafeJSContext;
use dom_struct::dom_struct;
use js::glue::{
    CreateReadableStreamUnderlyingSource, DeleteReadableStreamUnderlyingSource,
    ReadableStreamUnderlyingSourceTraps,
};
use js::jsapi::{
    AutoRequireNoGC, IsReadableStream, JS_GetArrayBufferViewData,
    NewReadableExternalSourceStreamObject, ReadableStreamClose, ReadableStreamDefaultReaderRead,
    ReadableStreamError, ReadableStreamGetReader, ReadableStreamIsDisturbed,
    ReadableStreamIsLocked, ReadableStreamIsReadable, ReadableStreamReaderMode,
    ReadableStreamReaderReleaseLock, ReadableStreamUnderlyingSource,
    ReadableStreamUpdateDataAvailableFromSource, UnwrapReadableStream,
};
use js::jsapi::{HandleObject, HandleValue, Heap, JSContext, JSObject};
use js::jsval::JSVal;
use js::jsval::UndefinedValue;
use js::rust::HandleValue as SafeHandleValue;
use js::rust::IntoHandle;
use std::cell::{Cell, RefCell};
use std::os::raw::c_void;
use std::ptr::{self, NonNull};
use std::rc::Rc;
use std::slice;

static UNDERLYING_SOURCE_TRAPS: ReadableStreamUnderlyingSourceTraps =
    ReadableStreamUnderlyingSourceTraps {
        requestData: Some(request_data),
        writeIntoReadRequestBuffer: Some(write_into_read_request_buffer),
        cancel: Some(cancel),
        onClosed: Some(close),
        onErrored: Some(error),
        finalize: Some(finalize),
    };

#[dom_struct]
pub struct UnderlyingSource {
    reflector_: Reflector,
}

#[allow(unsafe_code)]
unsafe extern "C" fn request_data(
    source: *const c_void,
    cx: *mut JSContext,
    stream: HandleObject,
    desired_size: usize,
) {
    let source = &*(source as *const ExternalUnderlyingSourceController);
    source.pull(SafeJSContext::from_ptr(cx), stream, desired_size);
}

#[allow(unsafe_code)]
unsafe extern "C" fn write_into_read_request_buffer(
    source: *const c_void,
    _cx: *mut JSContext,
    _stream: HandleObject,
    chunk: HandleObject,
    length: usize,
    bytes_written: *mut usize,
) {
    let source = &*(source as *const ExternalUnderlyingSourceController);
    let mut is_shared_memory = false;
    let buffer = JS_GetArrayBufferViewData(
        *chunk,
        &mut is_shared_memory,
        &AutoRequireNoGC { _address: 0 },
    );
    assert!(!is_shared_memory);
    let slice = slice::from_raw_parts_mut(buffer as *mut u8, length);
    source.write_into_buffer(slice);

    // Currently we're always able to completely fulfill the write request.
    *bytes_written = length;
}

#[allow(unsafe_code)]
unsafe extern "C" fn cancel(
    _source: *const c_void,
    _cx: *mut JSContext,
    _stream: HandleObject,
    _reason: HandleValue,
    _resolve_to: *mut JSVal,
) {
}

#[allow(unsafe_code)]
unsafe extern "C" fn close(_source: *const c_void, _cx: *mut JSContext, _stream: HandleObject) {}

#[allow(unsafe_code)]
unsafe extern "C" fn error(
    _source: *const c_void,
    _cx: *mut JSContext,
    _stream: HandleObject,
    _reason: HandleValue,
) {
}

#[allow(unsafe_code)]
unsafe extern "C" fn finalize(source: *mut ReadableStreamUnderlyingSource) {
    DeleteReadableStreamUnderlyingSource(source);
}

pub enum ExternalUnderlyingSource {
    /// Facilitate partial integration with sources
    /// that are currently read into memory.
    Memory(usize),
    /// A blob as underlying source, with a known total size.
    Blob(usize),
    /// A fetch response as underlying source.
    FetchResponse,
}

#[derive(JSTraceable, MallocSizeOf)]
struct ExternalUnderlyingSourceController {
    /// Loosely matches the underlying queue,
    /// <https://streams.spec.whatwg.org/#internal-queues>
    buffer: RefCell<Vec<u8>>,
    /// Has the stream been closed by native code?
    closed: Cell<bool>,
    /// Does this stream contains all it's data in memory?
    in_memory: Cell<bool>,
}

impl ExternalUnderlyingSourceController {
    fn new(source: ExternalUnderlyingSource) -> ExternalUnderlyingSourceController {
        let (buffer, in_mem) = match source {
            ExternalUnderlyingSource::Blob(size) => (Vec::with_capacity(size), false),
            ExternalUnderlyingSource::Memory(size) => (Vec::with_capacity(size), true),
            ExternalUnderlyingSource::FetchResponse => (vec![], false),
        };
        ExternalUnderlyingSourceController {
            buffer: RefCell::new(buffer),
            closed: Cell::new(false),
            in_memory: Cell::new(in_mem),
        }
    }

    /// Does the stream have all data in memory?
    pub fn in_memory(&self) -> bool {
        self.in_memory.get()
    }

    /// Return bytes synchronously if the stream has all data in memory.
    pub fn get_in_memory_bytes(&self) -> Option<Vec<u8>> {
        if self.in_memory.get() {
            return Some(self.buffer.borrow().clone());
        }
        None
    }

    /// Signal available bytes if the stream is currently readable.
    #[allow(unsafe_code)]
    fn maybe_signal_available_bytes(
        &self,
        cx: SafeJSContext,
        stream: HandleObject,
        available: usize,
    ) {
        if available == 0 {
            return;
        }
        unsafe {
            let mut readable = false;
            if !ReadableStreamIsReadable(*cx, stream, &mut readable) {
                return;
            }
            if readable {
                ReadableStreamUpdateDataAvailableFromSource(*cx, stream, available as u32);
            }
        }
    }

    /// Close a currently readable js stream.
    #[allow(unsafe_code)]
    fn maybe_close_js_stream(&self, cx: SafeJSContext, stream: HandleObject) {
        unsafe {
            let mut readable = false;
            if !ReadableStreamIsReadable(*cx, stream, &mut readable) {
                return;
            }
            if readable {
                ReadableStreamClose(*cx, stream);
            }
        }
    }

    fn close(&self, cx: SafeJSContext, stream: HandleObject) {
        self.closed.set(true);
        self.maybe_close_js_stream(cx, stream);
    }

    fn enqueue_chunk(&self, cx: SafeJSContext, stream: HandleObject, mut chunk: Vec<u8>) {
        let available = {
            let mut buffer = self.buffer.borrow_mut();
            chunk.append(&mut buffer);
            *buffer = chunk;
            buffer.len()
        };
        self.maybe_signal_available_bytes(cx, stream, available);
    }

    #[allow(unsafe_code)]
    fn pull(&self, cx: SafeJSContext, stream: HandleObject, _desired_size: usize) {
        // Note: for pull sources,
        // this would be the time to ask for a chunk.

        if self.closed.get() {
            return self.maybe_close_js_stream(cx, stream);
        }

        let available = {
            let buffer = self.buffer.borrow();
            buffer.len()
        };

        self.maybe_signal_available_bytes(cx, stream, available);
    }

    fn get_chunk_with_length(&self, length: usize) -> Vec<u8> {
        let mut buffer = self.buffer.borrow_mut();
        let buffer_len = buffer.len();
        assert!(buffer_len >= length as usize);
        buffer.split_off(buffer_len - length)
    }

    fn write_into_buffer(&self, dest: &mut [u8]) {
        let length = dest.len();
        let chunk = self.get_chunk_with_length(length);
        dest.copy_from_slice(chunk.as_slice());
    }
}

#[allow(unsafe_code)]
/// Get the `done` property of an object that a read promise resolved to.
pub fn get_read_promise_done(cx: SafeJSContext, v: &SafeHandleValue) -> Result<bool, Error> {
    unsafe {
        rooted!(in(*cx) let object = v.to_object());
        rooted!(in(*cx) let mut done = UndefinedValue());
        match get_dictionary_property(*cx, object.handle(), "done", done.handle_mut()) {
            Ok(true) => match bool::from_jsval(*cx, done.handle(), ()) {
                Ok(ConversionResult::Success(val)) => Ok(val),
                Ok(ConversionResult::Failure(error)) => Err(Error::Type(error.to_string())),
                _ => Err(Error::Type("Unknown format for done property.".to_string())),
            },
            Ok(false) => Err(Error::Type("Promise has no done property.".to_string())),
            Err(()) => Err(Error::JSFailed),
        }
    }
}

#[allow(unsafe_code)]
/// Get the `value` property of an object that a read promise resolved to.
pub fn get_read_promise_bytes(cx: SafeJSContext, v: &SafeHandleValue) -> Result<Vec<u8>, Error> {
    unsafe {
        rooted!(in(*cx) let object = v.to_object());
        rooted!(in(*cx) let mut bytes = UndefinedValue());
        match get_dictionary_property(*cx, object.handle(), "value", bytes.handle_mut()) {
            Ok(true) => {
                match Vec::<u8>::from_jsval(*cx, bytes.handle(), ConversionBehavior::EnforceRange) {
                    Ok(ConversionResult::Success(val)) => Ok(val),
                    Ok(ConversionResult::Failure(error)) => Err(Error::Type(error.to_string())),
                    _ => Err(Error::Type("Unknown format for bytes read.".to_string())),
                }
            },
            Ok(false) => Err(Error::Type("Promise has no value property.".to_string())),
            Err(()) => Err(Error::JSFailed),
        }
    }
}
