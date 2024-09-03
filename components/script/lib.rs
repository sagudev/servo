/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

#![feature(register_tool)]
//#![deny(unsafe_code)]
#![doc = "The script crate contains all matters DOM."]
// Register the linter `crown`, which is the Servo-specific linter for the script
// crate. Issue a warning if `crown` is not being used to compile, but not when
// building rustdoc or running clippy.
#![register_tool(crown)]
#![cfg_attr(any(doc, clippy), allow(unknown_lints))]
#![deny(crown_is_not_used)]
#![allow(unused_imports)]
#![allow(unused_variables)]

// These are used a lot so let's keep them for now
#[macro_use]
extern crate js;
#[macro_use]
extern crate jstraceable_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate malloc_size_of_derive;
#[macro_use]
extern crate servo_atoms;

mod animation_timeline;
mod animations;
#[warn(deprecated)]
#[macro_use]
mod task;
#[warn(deprecated)]
mod body;
#[warn(deprecated)]
pub mod clipboard_provider;
#[warn(deprecated)]
mod devtools;
#[warn(deprecated)]
pub mod document_loader;
#[warn(deprecated)]
#[macro_use]
mod dom;
#[warn(deprecated)]
mod canvas_state;
#[warn(deprecated)]
pub mod fetch;
#[warn(deprecated)]
mod image_listener;
#[warn(deprecated)]
mod init;
#[warn(deprecated)]
mod layout_image;

pub mod layout_dom;
#[warn(deprecated)]
mod mem;
#[warn(deprecated)]
mod microtask;
#[warn(deprecated)]
mod network_listener;
#[warn(deprecated)]
mod realms;
#[warn(deprecated)]
mod script_module;
#[warn(deprecated)]
pub mod script_runtime;
#[warn(deprecated)]
#[allow(unsafe_code)]
pub mod script_thread;
#[warn(deprecated)]
pub mod security_manager;
#[warn(deprecated)]
pub mod serviceworker_manager;
#[warn(deprecated)]
mod stylesheet_loader;
#[warn(deprecated)]
mod stylesheet_set;
#[warn(deprecated)]
mod task_manager;
#[warn(deprecated)]
mod task_queue;
#[warn(deprecated)]
mod task_source;
#[warn(deprecated)]
pub mod test;
#[warn(deprecated)]
pub mod textinput;
#[warn(deprecated)]
mod timers;
#[warn(deprecated)]
mod unpremultiplytable;
#[warn(deprecated)]
mod webdriver_handlers;
#[warn(deprecated)]
mod window_named_properties;

pub use init::init;
pub use script_runtime::JSEngineSetup;

// export traits to be available for derive macros
pub use script_bindings::inheritance::HasParent;
pub use script_bindings::reflector::{DomObject/*, DomGlobal*/, MutDomObject, Reflector};
pub use script_bindings::trace::{CustomTraceable, JSTraceable};

pub use crate::dom::bindings::codegen::DomTypeHolder::DomTypeHolder;

impl script_bindings::DomHelpers<crate::DomTypeHolder> for crate::DomTypeHolder {
    fn throw_dom_exception(
        cx: crate::script_runtime::JSContext,
        global: &<crate::DomTypeHolder as script_bindings::DomTypes>::GlobalScope,
        result: script_bindings::error::Error,
    ) {
        todo!()
    }

    unsafe fn global_scope_from_object(obj: *mut js::jsapi::JSObject) -> crate::dom::bindings::root::DomRoot<<crate::DomTypeHolder as script_bindings::DomTypes>::GlobalScope> {
        todo!()
    }

    fn global_scope_origin(global: &<crate::DomTypeHolder as script_bindings::DomTypes>::GlobalScope) -> &servo_url::MutableOrigin {
        todo!()
    }

    fn report_cross_origin_denial(cx: crate::script_runtime::JSContext, id: js::jsapi::HandleId, access: &str) -> bool {
        todo!()
    }

    fn Window_create_named_properties_object(
        cx: crate::script_runtime::JSContext,
        proto: js::rust::HandleObject,
        object: js::rust::MutableHandleObject,
    ) {
        todo!()
    }

    fn Promise_new_resolved(
        global: &<crate::DomTypeHolder as script_bindings::DomTypes>::GlobalScope,
        cx: crate::script_runtime::JSContext,
        value: js::rust::HandleValue,
    ) -> crate::dom::bindings::error::Fallible<std::rc::Rc<<crate::DomTypeHolder as script_bindings::DomTypes>::Promise>> {
        todo!()
    }

    unsafe fn GlobalScope_from_object_maybe_wrapped(
        obj: *mut js::jsapi::JSObject,
        cx: *mut js::jsapi::JSContext,
    ) -> crate::dom::bindings::root::DomRoot<<crate::DomTypeHolder as script_bindings::DomTypes>::GlobalScope> {
        todo!()
    }

    fn GlobalScope_incumbent() -> Option<crate::dom::bindings::root::DomRoot<<crate::DomTypeHolder as script_bindings::DomTypes>::GlobalScope>> {
        todo!()
    }

    fn GlobalScope_get_cx() -> crate::script_runtime::JSContext {
        todo!()
    }

    fn GlobalScope_from_context(cx: *mut js::jsapi::JSContext, in_realm: crate::realms::InRealm) -> crate::dom::bindings::root::DomRoot<<crate::DomTypeHolder as script_bindings::DomTypes>::GlobalScope> {
        todo!()
    }

    fn GlobalScope_report_an_error(info: crate::dom::bindings::error::ErrorInfo, value: js::rust::HandleValue) {
        todo!()
    }

    fn TestBinding_condition_satisfied(cx: crate::script_runtime::JSContext, obj: js::rust::HandleObject) -> bool {
        todo!()
    }
    fn TestBinding_condition_unsatisfied(cx: crate::script_runtime::JSContext, obj: js::rust::HandleObject) -> bool {
        todo!()
    }
    fn WebGL2RenderingContext_is_webgl2_enabled(cx: crate::script_runtime::JSContext, obj: js::rust::HandleObject) -> bool {
        todo!()
    }

    fn perform_a_microtask_checkpoint(global: &<crate::DomTypeHolder as script_bindings::DomTypes>::GlobalScope) {
        todo!()
    }

    fn ReadableStream_from_js(cx: crate::script_runtime::JSContext, obj: *mut js::jsapi::JSObject, in_realm: crate::realms::InRealm) -> Result<crate::dom::bindings::root::DomRoot<<crate::DomTypeHolder as script_bindings::DomTypes>::ReadableStream>, ()> {
        todo!()
    }

    fn DOMException_stringifier(exception: &<crate::DomTypeHolder as script_bindings::DomTypes>::DOMException) -> crate::dom::bindings::str::DOMString {
        todo!()
    }

    fn get_map() -> &'static phf::Map<&'static [u8], fn(crate::script_runtime::JSContext, js::rust::HandleObject)> {
        todo!()
    }
}
