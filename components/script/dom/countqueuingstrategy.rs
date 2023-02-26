/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::dom::bindings::reflector::{reflect_dom_object, DomObject, Reflector};
use dom_struct::dom_struct;
//use crate::dom::bindings::codegen::Bindings::CountQueuingStrategyBindings::CountQueuingStrategyMethods;
use crate::dom::bindings::codegen::Bindings::CountQueuingStrategyBinding::CountQueuingStrategyBinding::CountQueuingStrategyMethods;

use super::bindings::codegen::Bindings::QueuingStrategyBinding::QueuingStrategyInit;
use super::bindings::error::Fallible;
use super::bindings::root::DomRoot;
use super::types::GlobalScope;
#[dom_struct]
pub struct CountQueuingStrategy {
    reflector_: Reflector,
    high_water_mark: f64,
}

impl CountQueuingStrategyMethods for CountQueuingStrategy {
    fn HighWaterMark(&self) -> f64 {
        self.high_water_mark
    }
}

#[allow(non_snake_case)]
impl CountQueuingStrategy {
    //https://streams.spec.whatwg.org/#cqs-constructor
    pub fn Constructor(global: &GlobalScope, init: &QueuingStrategyInit) -> DomRoot<Self> {
        Self::new(global, init.highWaterMark)
    }
}

impl CountQueuingStrategy {
    pub fn new_inherited(init: f64) -> Self {
        Self {
            reflector_: Reflector::new(),
            high_water_mark: init,
        }
    }

    pub fn new(global: &GlobalScope,init: f64) -> DomRoot<Self> {
        let count_queuing_strategy = Self::new_inherited(init);
        reflect_dom_object(Box::new(count_queuing_strategy), global)
    }
}