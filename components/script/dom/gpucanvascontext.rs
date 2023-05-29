/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::dom::bindings::cell::DomRefCell;
use crate::dom::bindings::codegen::Bindings::GPUCanvasContextBinding::{
    GPUCanvasConfiguration, GPUCanvasContextMethods,
};
use crate::dom::bindings::codegen::Bindings::GPUDeviceBinding::GPUDeviceBinding::GPUDeviceMethods;
use crate::dom::bindings::codegen::Bindings::GPUObjectBaseBinding::GPUObjectDescriptorBase;
use crate::dom::bindings::codegen::Bindings::GPUTextureBinding::{
    GPUExtent3D, GPUExtent3DDict, GPUTextureDescriptor, GPUTextureDimension, GPUTextureFormat,
};
use crate::dom::bindings::inheritance::Castable;
use crate::dom::bindings::reflector::{reflect_dom_object, DomObject, Reflector};
use crate::dom::bindings::root::{Dom, DomRoot, LayoutDom};
use crate::dom::globalscope::GlobalScope;
use crate::dom::htmlcanvaselement::{HTMLCanvasElement, LayoutCanvasRenderingContextHelpers};
use crate::dom::node::{document_from_node, Node, NodeDamage};
use arrayvec::ArrayVec;
use dom_struct::dom_struct;
use euclid::default::Size2D;
use ipc_channel::ipc;
use script_layout_interface::HTMLCanvasDataSource;
use std::cell::Cell;
use webgpu::{wgpu::id, wgt, WebGPU, WebGPURequest, PRESENTATION_BUFFER_COUNT};

use super::bindings::codegen::UnionTypes::HTMLCanvasElementOrOffscreenCanvas;
use super::bindings::error::{Fallible, Error};
use super::bindings::root::MutNullableDom;
use super::gpudevice::convert_texture_format;
use super::gputexture::GPUTexture;
use super::offscreencanvas::OffscreenCanvas;

#[derive(Clone, Copy, Debug, Eq, Hash, MallocSizeOf, Ord, PartialEq, PartialOrd)]
pub struct WebGPUContextId(pub u64);

#[derive(Clone, JSTraceable, MallocSizeOf)]
pub enum Canvas {
    Elemental(DomRoot<HTMLCanvasElement>),
    Offscreen(DomRoot<OffscreenCanvas>),
}

impl From<HTMLCanvasElementOrOffscreenCanvas> for Canvas {
    fn from(canvas: HTMLCanvasElementOrOffscreenCanvas) -> Self {
        match canvas {
            HTMLCanvasElementOrOffscreenCanvas::HTMLCanvasElement(hce) => Self::Elemental(hce),
            HTMLCanvasElementOrOffscreenCanvas::OffscreenCanvas(oc) => Self::Offscreen(oc),
        }
    }
}

pub type SurfaceConfiguration = wgt::SurfaceConfiguration<Vec<wgt::TextureFormat>>;

#[dom_struct]
pub struct GPUCanvasContext {
    reflector_: Reflector,
    #[ignore_malloc_size_of = "channels are hard"]
    channel: WebGPU,
    canvas: Canvas,
    size: Cell<Size2D<u32>>,
    #[ignore_malloc_size_of = "Defined in webrender"]
    webrender_image: Cell<Option<webrender_api::ImageKey>>,
    context_id: WebGPUContextId,
    config: MutNullableDom<GPUCanvasConfiguration>,
    texture: Dom<GPUTexture>,
}

impl GPUCanvasContext {
    fn new_inherited(canvas: Canvas, size: Size2D<u32>, channel: WebGPU) -> Self {
        let (sender, receiver) = ipc::channel().unwrap();
        if let Err(e) = channel.0.send((None, WebGPURequest::CreateContext(sender))) {
            warn!("Failed to send CreateContext ({:?})", e);
        }
        let external_id = receiver.recv().unwrap();
        Self {
            reflector_: Reflector::new(),
            channel,
            canvas,
            size: Cell::new(size),
            webrender_image: Cell::new(None),
            context_id: WebGPUContextId(external_id.0),
            config: None,
            context: todo!(),
            texture: todo!(),
        }
    }

    pub fn new(
        global: &GlobalScope,
        canvas: &HTMLCanvasElement,
        size: Size2D<u32>,
        channel: WebGPU,
    ) -> DomRoot<Self> {
        reflect_dom_object(
            Box::new(GPUCanvasContext::new_inherited(
                Canvas::Elemental(DomRoot::from_ref(canvas)),
                size,
                channel,
            )),
            global,
        )
    }
}

impl GPUCanvasContext {
    fn layout_handle(&self) -> HTMLCanvasDataSource {
        let image_key = if self.webrender_image.get().is_some() {
            self.webrender_image.get().unwrap()
        } else {
            webrender_api::ImageKey::DUMMY
        };
        HTMLCanvasDataSource::WebGPU(image_key)
    }

    pub fn context_id(&self) -> WebGPUContextId {
        self.context_id
    }

    pub fn mark_as_dirty(&self) {
        if let Canvas::Elemental(canvas) = &self.canvas {
            canvas.upcast::<Node>().dirty(NodeDamage::OtherNodeDamage);
            let document = document_from_node(&**canvas);
            document.add_dirty_webgpu_canvas(self);
        }
    }
}

impl LayoutCanvasRenderingContextHelpers for LayoutDom<'_, GPUCanvasContext> {
    #[allow(unsafe_code)]
    unsafe fn canvas_data_source(self) -> HTMLCanvasDataSource {
        (*self.unsafe_get()).layout_handle()
    }
}

impl GPUCanvasContextMethods for GPUCanvasContext {
    /// https://gpuweb.github.io/gpuweb/#dom-gpucanvascontext-canvas
    fn Canvas(&self) -> HTMLCanvasElementOrOffscreenCanvas {
        match &self.canvas {
            Canvas::Elemental(hce) => {
                HTMLCanvasElementOrOffscreenCanvas::HTMLCanvasElement(hce.clone())
            },
            Canvas::Offscreen(oc) => {
                HTMLCanvasElementOrOffscreenCanvas::OffscreenCanvas(oc.clone())
            },
        }
    }

    /// https://gpuweb.github.io/gpuweb/#dom-gpucanvascontext-configure
    fn Configure(&self, descriptor: &GPUCanvasConfiguration) {
        self.Unconfigure();

        // idk
        let mut buffer_ids = ArrayVec::<id::BufferId, PRESENTATION_BUFFER_COUNT>::new();
        for _ in 0..PRESENTATION_BUFFER_COUNT {
            buffer_ids.push(
                self.global()
                    .wgpu_id_hub()
                    .lock()
                    .create_buffer_id(descriptor.device.id().0.backend()),
            );
        }

        let image_desc = webrender_api::ImageDescriptor {
            format: match descriptor.format {
                GPUTextureFormat::Rgba8unorm | GPUTextureFormat::Rgba8unorm_srgb => webrender_api::ImageFormat::RGBA8,
                GPUTextureFormat::Bgra8unorm | GPUTextureFormat::Bgra8unorm_srgb => webrender_api::ImageFormat::BGRA8,
                _ => panic!("SwapChain format({:?}) not supported", descriptor.format),
            },
            size: webrender_api::units::DeviceIntSize::new(
                self.size.get().width as i32,
                self.size.get().height as i32,
            ),
            stride: Some(
                (((self.size.get().width * 4) | (wgt::COPY_BYTES_PER_ROW_ALIGNMENT - 1)) + 1)
                    as i32,
            ),
            offset: 0,
            flags: webrender_api::ImageDescriptorFlags::from_bits(1).unwrap(),
        };

        let image_data = webrender_api::ImageData::External(webrender_api::ExternalImageData {
            id: webrender_api::ExternalImageId(self.context_id.0),
            channel_index: 0,
            image_type: webrender_api::ExternalImageType::Buffer,
        });

        let (sender, receiver) = ipc::channel().unwrap();

        self.channel
            .0
            .send((
                None,
                WebGPURequest::CreateSwapChain {
                    device_id: descriptor.device.id().0,
                    buffer_ids,
                    external_id: self.context_id.0,
                    sender,
                    image_desc,
                    image_data,
                },
            ))
            .expect("Failed to create WebGPU SwapChain");

        let usage = if descriptor.usage % 2 == 0 {
            descriptor.usage + 1
        } else {
            descriptor.usage
        };
        let text_desc = GPUTextureDescriptor {
            parent: GPUObjectDescriptorBase { label: None },
            dimension: GPUTextureDimension::_2d,
            format: descriptor.format,
            mipLevelCount: 1,
            sampleCount: 1,
            usage,
            size: GPUExtent3D::GPUExtent3DDict(GPUExtent3DDict {
                width: self.size.get().width,
                height: self.size.get().height,
                depthOrArrayLayers: 1,
            }),
            viewFormats: descriptor.viewFormats.iter()
            .map(|tf| convert_texture_format(*tf))
            .collect(),
        };

        let texture = descriptor.device.CreateTexture(&text_desc);

        self.webrender_image.set(Some(receiver.recv().unwrap()));

        let swap_chain = GPUSwapChain::new(
            &self.global(),
            self.channel.clone(),
            &self,
            &*texture,
            descriptor.parent.label.as_ref().cloned(),
        );
        *self.swap_chain.borrow_mut() = Some(Dom::from_ref(&*swap_chain));
        swap_chain
    }

    /// https://gpuweb.github.io/gpuweb/#dom-gpucanvascontext-unconfigure
    fn Unconfigure(&self) {
        // send swapchain destroy
        if let Some(chain) = &*self.swap_chain.borrow() {
            chain.destroy(self.context_id.0, self.webrender_image.get().unwrap());
            self.webrender_image.set(None);
        }
        *self.swap_chain.borrow_mut() = None;
        todo!()
    }

    /// https://gpuweb.github.io/gpuweb/#dom-gpucanvascontext-getcurrenttexture
    fn GetCurrentTexture(&self) -> Fallible<DomRoot<GPUTexture>> {
        // Step 1.
        if self.config.is_none() {
            return Err(Error::InvalidState)
        }
        // Step 2,3,4 are implicit
        // Step 5.
        self.context.mark_as_dirty();
        // Step 6.
        Ok(DomRoot::from_ref(&*self.texture))
    }
}

impl Drop for GPUCanvasContext {
    fn drop(&mut self) {
        self.Unconfigure()
    }
}