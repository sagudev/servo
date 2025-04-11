/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::ops::{Deref, DerefMut};

use euclid::default::Size2D;
use ipc_channel::ipc::IpcSharedMemory;
use serde::{Deserialize, Serialize};

mod types;

pub use types::*;

#[derive(Debug)]
pub enum Data {
    //IPC(IpcSharedMemory),
    Owned(Vec<u8>),
}

impl Deref for Data {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match &self {
            //Data::IPC(ipc_shared_memory) => ipc_shared_memory,
            Data::Owned(items) => items,
        }
    }
}

impl DerefMut for Data {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            //Data::IPC(ipc_shared_memory) => unsafe { ipc_shared_memory.deref_mut() },
            Data::Owned(items) => items,
        }
    }
}

pub type AnySnapshot<T> = Snapshot<T, PixelFormat, AlphaMode>;

#[derive(Debug, Deserialize, Serialize)]
pub struct Snapshot<T, P: PixelFormatTrait, A: AlphaModeTrait> {
    size: Size2D<u64>,
    /// internal data (can be any format it will be converted on use if needed)
    data: T,
    /// RGBA/BGRA (reflect internal data)
    format: P,
    /// How to threat alpha channel
    alpha_mode: A,
}

impl<T, P: PixelFormatTrait, A: AlphaModeTrait> Snapshot<T, P, A> {
    pub const fn size(&self) -> Size2D<u64> {
        self.size
    }
}

impl<T, P: PixelFormatTrait, A: AlphaModeTrait> Snapshot<T, P, A> {
    pub fn format(&self) -> PixelFormat {
        self.format.format()
    }

    pub fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode.alpha_mode()
    }

    pub fn is_premultiplied(&self) -> bool {
        self.alpha_mode().is_premultiplied()
    }

    pub fn is_opaque(&self) -> bool {
        self.alpha_mode().is_opaque()
    }
}

impl<P: PixelFormatTrait, A: AlphaModeTrait> Snapshot<Data, P, A> {
    pub fn empty() -> Self {
        Self {
            size: Size2D::zero(),
            data: Data::Owned(vec![]),
            format: P::default(),
            alpha_mode: A::default(),
        }
    }

    pub fn cleared(size: Size2D<u64>) -> Self {
        Self {
            size,
            data: Data::Owned(vec![0; size.area() as usize * 4]),
            format: P::default(),
            alpha_mode: A::default(),
        }
    }

    pub fn as_ipc(self) -> Snapshot<IpcSharedMemory, P, A> {
        let Snapshot {
            size,
            data,
            format,
            alpha_mode,
        } = self;
        let data = match data {
            //Data::IPC(ipc_shared_memory) => ipc_shared_memory,
            Data::Owned(items) => IpcSharedMemory::from_bytes(&items),
        };
        Snapshot {
            size,
            data,
            format,
            alpha_mode,
        }
    }

    pub fn transform<TP: ConstPixelFormat + Default, TA: ConstAlphaMode + Default>(mut self) -> Snapshot<Data, TP, TA> {
        let target_alpha_mode = TA::ALPHA_MODE;
        let target_format = TP::PIXEL_FORMAT;
        let (swap_rb, multiply, clear_alpha) = to_target_parameters(
            self.format(),
            target_format,
            self.alpha_mode(),
            target_alpha_mode,
        );
        pixels::transform_inplace(self.data.deref_mut(), multiply, swap_rb, clear_alpha);

        Snapshot::<Data, TP, TA> {
            size: self.size,
            data: self.data,
            format: TP::default(),
            alpha_mode: TA::default(),
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn to_vec(self) -> Vec<u8> {
        match self.data {
            Data::Owned(data) => data,
        }
    }
}

impl<T, P: PixelFormatTrait, A: AlphaModeTrait> Snapshot<T, P, A> {
    pub fn erase_types(self) -> Snapshot<T, PixelFormat, AlphaMode> {
        Snapshot {
            size: self.size,
            format: self.format(),
            alpha_mode: self.alpha_mode(),
            data: self.data,
        }
    }
}

impl Snapshot<Data, PixelFormat, AlphaMode> {
    pub fn new(size: Size2D<u64>, format: PixelFormat, alpha_mode: AlphaMode, data: Vec<u8>) -> Self {
        Self {
            size,
            data: Data::Owned(data),
            format,
            alpha_mode,
        }
    }

    pub fn from_vec(size: Size2D<u64>, format: PixelFormat, alpha_mode: AlphaMode, data: Vec<u8>) -> Self {
        Self {
            size,
            data: Data::Owned(data),
            format,
            alpha_mode,
        }
    }

    pub fn from_ism(size: Size2D<u64>, format: PixelFormat, alpha_mode: AlphaMode, ism: IpcSharedMemory) -> Self {
        Self {
            size,
            data: Data::Owned(ism.to_vec()),
            format,
            alpha_mode,
        }
    }
}

impl<P: ConstPixelFormat + Default, A: ConstAlphaMode + Default> Snapshot<Data, P, A> {
    pub fn new(size: Size2D<u64>, data: Vec<u8>) -> Self {
        Self {
            size,
            data: Data::Owned(data),
            format: P::default(),
            alpha_mode: A::default(),
        }
    }

    pub fn from_vec(size: Size2D<u64>, data: Vec<u8>) -> Self {
        Self {
            size,
            data: Data::Owned(data),
            format: P::default(),
            alpha_mode: A::default(),
        }
    }

    pub fn from_ism(size: Size2D<u64>, ism: IpcSharedMemory) -> Self {
        Self {
            size,
            data: Data::Owned(ism.to_vec()),
            format: P::default(),
            alpha_mode: A::default(),
        }
    }

    /*
    /// # Safety
    ///
    /// This is safe is data is owned by this proces only
    /// (ownership is transferred on send)
    pub unsafe fn from_ism(
        size: Size2D<u64>,
        format: PixelFormat,
        alpha_mode: AlphaMode,
        ism: IpcSharedMemory,
    ) -> Self {
        Self {
            size,
            data: Data::IPC(ism),
            format,
            alpha_mode,
        }
    }
    */

    /*pub fn transform<TP: ConstPixelFormat, TA: ConstAlphaMode>(mut self) -> Snapshot<Data, TP, TA> {
        let target_alpha_mode = TA::alpha_mode;
        let target_format = TP::pixel_format;
        const smc = to_target_parameters(
            P::pixel_format,
            TP::pixel_format,
            A::alpha_mode,
            TA::alpha_mode,
        );
        pixels::generic_transform_inplace::<{smc.0}, {smc.1}, {smc.2}>(self.data.deref_mut());

        Snapshot::<Data, P, A> {
            size: self.size,
            data: self.data,
            format: P::new(),
            alpha_mode: A::new(),
        }
    }*/
}

impl<P: PixelFormatTrait, A: AlphaModeTrait> Snapshot<IpcSharedMemory, P, A> {
    /*
    /// # Safety
    ///
    /// This is safe is data is owned by this proces only
    /// (ownership is transferred on send)
    pub unsafe fn to_data(self) -> Snapshot<Data> {
        let Snapshot {
            size,
            data,
            format,
            alpha_mode,
        } = self;
        Snapshot {
            size,
            data: Data::IPC(data),
            format,
            alpha_mode,
        }
    }
    */
    pub fn to_owned(self) -> Snapshot<Data, P, A> {
        let Snapshot {
            size,
            data,
            format,
            alpha_mode,
        } = self;
        Snapshot {
            size,
            data: Data::Owned(data.to_vec()),
            format,
            alpha_mode,
        }
    }

    pub fn to_ipc_shared_memory(self) -> IpcSharedMemory {
        self.data
    }
}
