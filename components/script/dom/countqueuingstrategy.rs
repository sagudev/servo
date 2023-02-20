use crate::dom::bindings::reflector::{reflect_dom_object, DomObject, Reflector};
use dom_struct::dom_struct;
//use crate::dom::bindings::codegen::Bindings::CountQueuingStrategyBindings::CountQueuingStrategyMethods;
use crate::dom::bindings::codegen::Bindings::CountQueuingStrategyBinding::CountQueuingStrategyBinding::CountQueuingStrategyMethods;
#[dom_struct]
pub struct CountQueuingStrategy {
    reflector_: Reflector,
}

impl CountQueuingStrategyMethods for CountQueuingStrategy {
    fn HighWaterMark(&self) -> f64 {
        todo!()
    }
}
