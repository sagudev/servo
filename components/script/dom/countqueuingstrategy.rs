/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

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
