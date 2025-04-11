use serde::{Deserialize, Serialize};
use pixels::Multiply;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PixelFormat {
    #[default]
    RGBA,
    BGRA,
}

pub trait PixelFormatTrait: Default {
    fn format(&self) -> PixelFormat;
}

impl PixelFormatTrait for PixelFormat {
    fn format(&self) -> PixelFormat {
        *self
    }
}

impl<T: ConstPixelFormat> PixelFormatTrait for T {
    fn format(&self) -> PixelFormat {
        Self::PIXEL_FORMAT
    }
}

/*
We use this to simulate enums in GATs
*/

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RGBA;
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct BGRA;

pub trait ConstPixelFormat: Default {
    const PIXEL_FORMAT: PixelFormat;
}

impl ConstPixelFormat for RGBA {
    const PIXEL_FORMAT: PixelFormat = PixelFormat::RGBA;
}

impl ConstPixelFormat for BGRA {
    const PIXEL_FORMAT: PixelFormat = PixelFormat::BGRA;
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AlphaMode {
    /// Internal data is opaque (alpha is cleared to 1)
    Opaque,
    /// Internal data should be threated as opaque (does not mean it actually is)
    AsOpaque { premultiplied: bool },
    /// Data is not opaque
    Transparent { premultiplied: bool },
}

impl Default for AlphaMode {
    fn default() -> Self {
        Self::Transparent {
            premultiplied: true,
        }
    }
}

impl AlphaMode {
    pub const fn is_premultiplied(&self) -> bool {
        match self {
            AlphaMode::Opaque => true,
            AlphaMode::AsOpaque { premultiplied } => *premultiplied,
            AlphaMode::Transparent { premultiplied } => *premultiplied,
        }
    }

    pub const fn is_opaque(&self) -> bool {
        matches!(self, AlphaMode::Opaque | AlphaMode::AsOpaque { .. })
    }
}

pub trait AlphaModeTrait: Default {
    fn alpha_mode(&self) -> AlphaMode;
}

impl AlphaModeTrait for AlphaMode {
    fn alpha_mode(&self) -> AlphaMode {
        *self
    }
}

impl<T: ConstAlphaMode + Default> AlphaModeTrait for T {
    fn alpha_mode(&self) -> AlphaMode {
        Self::ALPHA_MODE
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Opaque;
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AsOpaque<const PREMULTIPLIED: bool>;
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Transparent<const PREMULTIPLIED: bool>;

pub trait ConstAlphaMode {
    const ALPHA_MODE: AlphaMode;
}

impl ConstAlphaMode for Opaque {
    const ALPHA_MODE: AlphaMode = AlphaMode::Opaque;
}

impl<const PREMULTIPLIED: bool> ConstAlphaMode for AsOpaque<PREMULTIPLIED> {
    const ALPHA_MODE: AlphaMode = AlphaMode::AsOpaque { premultiplied: PREMULTIPLIED };
}

impl<const PREMULTIPLIED: bool> ConstAlphaMode for Transparent<PREMULTIPLIED> {
    const ALPHA_MODE: AlphaMode = AlphaMode::Transparent { premultiplied: PREMULTIPLIED };
}

pub(crate) const fn to_target_parameters(
    format: PixelFormat,
    target_format: PixelFormat,
    alpha_mode: AlphaMode,
    target_alpha_mode: AlphaMode,
) -> (bool, Multiply, bool) {
    let swap_rb = !matches!((format, target_format), (PixelFormat::RGBA, PixelFormat::RGBA) | (PixelFormat::BGRA, PixelFormat::BGRA));
    let multiply = match (alpha_mode, target_alpha_mode) {
        (AlphaMode::Opaque, _) => Multiply::None,
        (alpha_mode, AlphaMode::Opaque) => {
            if alpha_mode.is_premultiplied() {
                Multiply::UnMultiply
            } else {
                Multiply::None
            }
        },
        (
            AlphaMode::Transparent { premultiplied } | AlphaMode::AsOpaque { premultiplied },
            AlphaMode::Transparent {
                premultiplied: target_premultiplied,
            } |
            AlphaMode::AsOpaque {
                premultiplied: target_premultiplied,
            },
        ) => {
            if premultiplied == target_premultiplied {
                Multiply::None
            } else if target_premultiplied {
                Multiply::PreMultiply
            } else {
                Multiply::UnMultiply
            }
        },
    };
    let clear_alpha =
        !matches!(alpha_mode, AlphaMode::Opaque) && matches!(target_alpha_mode, AlphaMode::Opaque);
    (swap_rb, multiply, clear_alpha)
}
