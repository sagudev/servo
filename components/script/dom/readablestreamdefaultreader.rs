use crate::dom::bindings::codegen::Bindings::ReadableStreamDefaultReaderBinding::ReadableStreamDefaultReaderMethods;
use crate::dom::bindings::reflector::{reflect_dom_object, DomObject, Reflector};
use dom_struct::dom_struct;
#[dom_struct]
pub struct ReadableStreamDefaultReader {
    reflector_: Reflector,
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
