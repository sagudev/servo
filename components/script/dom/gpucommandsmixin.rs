/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! https://gpuweb.github.io/gpuweb/#gpucommandsmixin

use std::cell::Cell;
use webgpu::identity::WebGPUOpResult;

use super::bindings::codegen::UnionTypes::DoubleSequenceOrGPUColorDict;
use crate::dom::bindings::cell::DomRefCell;
use crate::dom::bindings::codegen::Bindings::WebGPUBinding::{
    GPUCommandEncoderMethods, GPUComputePassDescriptor, GPUExtent3D, GPUOrigin3D,
    GPURenderPassDescriptor, GPUSize64, GPUStoreOp,
};
use crate::dom::bindings::root::{Dom};
use crate::dom::gpudevice::GPUDevice;

// https://gpuweb.github.io/gpuweb/#encoder-state
#[derive(JSTraceable, MallocSizeOf, PartialEq)]
pub enum GPUEncoderState {
    Open,
    Locked,
    Ended,
}

pub trait GPUCommandsMixin {
    fn state(&self) -> DomRefCell<GPUEncoderState>;
    fn valid(&self) -> Cell<bool>;
    fn device(&self) -> Dom<GPUDevice>;

    /// https://gpuweb.github.io/gpuweb/#abstract-opdef-validate-the-encoder-state
    fn validate(&self) -> bool {
        let state = self.state().borrow();
        match *state {
            GPUEncoderState::Open => true,
            GPUEncoderState::Locked => {
                self.valid().set(false);
                false
            }
            GPUEncoderState::Ended => {
                let device = self.device();
                let scope_id = device.use_current_scope();
                device.handle_server_msg(
                    scope_id,
                    WebGPUOpResult::ValidationError(String::from("Command encoder is already closed")),
                );
                false
            },
        }
    }
}