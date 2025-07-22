/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! Vello implementation of 2D canvas backend.
//!
//! Vello only encodes commands for GPU, then runs rendering when
//! image is explicitly requested. This requires to copy image
//! from texture to buffer, then download buffer to CPU
//! (where we also need to un pad it).
//!
//! All Vello images are in no alpha premultiplied RGBA8 pixel format.

use std::cell::RefCell;
use std::collections::HashMap;

use canvas_traits::canvas::{
    BlendingStyle, CanvasGradientStop, CompositionOrBlending, CompositionStyle, FillOrStrokeStyle,
    LineCapStyle, LineJoinStyle, Path, TextAlign, TextBaseline,
};
use compositing_traits::SerializableImageData;
use euclid::default::{Point2D, Rect, Size2D, Transform2D, Vector2D};
use fonts::{ByteIndex, FontIdentifier, FontTemplateRefMethods as _};
use ipc_channel::ipc::IpcSharedMemory;
use pixels::{Snapshot, SnapshotAlphaMode, SnapshotPixelFormat};
use range::Range;
use style::color::AbsoluteColor;
use vello_cpu::{kurbo, peniko};
use webrender_api::{ImageDescriptor, ImageDescriptorFlags};

use crate::backend::{
    Backend, DrawOptionsHelpers, GenericDrawTarget, PatternHelpers, StrokeOptionsHelpers,
};
use crate::canvas_data::{CanvasPaintState, Filter, TextRun};

thread_local! {
    /// The shared font cache used by all canvases that render on a thread. It would be nicer
    /// to have a global cache, but it looks like font-kit uses a per-thread FreeType, so
    /// in order to ensure that fonts are particular to a thread we have to make our own
    /// cache thread local as well.
    static SHARED_FONT_CACHE: RefCell<HashMap<FontIdentifier, peniko::Font>> = RefCell::default();
}

#[derive(Clone, Default)]
pub(crate) struct VelloBackend;

impl Backend for VelloBackend {
    type Pattern<'a> = Pattern;
    type StrokeOptions = kurbo::Stroke;
    type Color = peniko::Color;
    type DrawOptions = DrawOptions;
    type CompositionOp = peniko::BlendMode;
    type DrawTarget = DrawTarget;
    type SourceSurface = Vec<u8>; // TODO: this should be texture
    type GradientStop = peniko::ColorStop;
    type GradientStops = peniko::ColorStops;

    fn get_composition_op(&self, opts: &Self::DrawOptions) -> Self::CompositionOp {
        opts.blend_mode
    }

    fn need_to_draw_shadow(&self, color: &Self::Color) -> bool {
        color.components[3] != 0.
    }

    fn set_shadow_color(&mut self, color: AbsoluteColor, state: &mut CanvasPaintState<'_, Self>) {
        state.shadow_color = color.convert();
    }

    fn set_fill_style(
        &mut self,
        style: FillOrStrokeStyle,
        state: &mut CanvasPaintState<'_, Self>,
        _drawtarget: &Self::DrawTarget,
    ) {
        state.fill_style = style.convert();
    }

    fn set_stroke_style(
        &mut self,
        style: FillOrStrokeStyle,
        state: &mut CanvasPaintState<'_, Self>,
        _drawtarget: &Self::DrawTarget,
    ) {
        state.stroke_style = style.convert();
    }

    fn set_global_composition(
        &mut self,
        op: CompositionOrBlending,
        state: &mut CanvasPaintState<'_, Self>,
    ) {
        state.draw_options.blend_mode = op.convert();
    }

    fn create_drawtarget(&self, size: Size2D<u64>) -> Self::DrawTarget {
        DrawTarget::new(size.cast())
    }

    fn new_paint_state<'a>(&self) -> CanvasPaintState<'a, Self> {
        let pattern = Pattern::new(peniko::color::AlphaColor::BLACK.into());
        CanvasPaintState {
            draw_options: DrawOptions::default(),
            fill_style: pattern.clone(),
            stroke_style: pattern,
            stroke_opts: kurbo::Stroke {
                width: 1.0,
                join: kurbo::Join::Miter,
                miter_limit: 10.0,
                start_cap: kurbo::Cap::Butt,
                end_cap: kurbo::Cap::Butt,
                ..Default::default()
            },
            transform: Transform2D::identity(),
            shadow_offset_x: 0.0,
            shadow_offset_y: 0.0,
            shadow_blur: 0.0,
            shadow_color: peniko::Color::TRANSPARENT,
            font_style: None,
            text_align: TextAlign::default(),
            text_baseline: TextBaseline::default(),
            _backend: std::marker::PhantomData,
        }
    }
}

pub struct DrawTarget {
    scene: vello_cpu::RenderContext,
    size: Size2D<u16>,
    pixmap: vello_cpu::Pixmap,
    clips: Vec<Path>,
}

impl DrawTarget {
    fn new(size: Size2D<u16>) -> Self {
        Self {
            scene: vello_cpu::RenderContext::new(size.width, size.height),
            pixmap: vello_cpu::Pixmap::new(size.width, size.height),
            size,
            clips: Vec::new(),
        }
    }

    fn pixmap(&mut self) -> &[u8] {
        //self.scene.flush();
        for _ in &self.clips {
            self.scene.pop_layer();
        }
        self.scene
            .render_to_pixmap(&mut self.pixmap, vello_cpu::RenderMode::OptimizeQuality);
        for path in &self.clips {
            self.scene.push_clip_layer(&path.0);
        }
        self.pixmap.data_as_u8_slice()
    }

    fn push_draw_options(&mut self, draw_options: &DrawOptions) {
        self.scene.push_layer(
            None,
            Some(draw_options.blend_mode),
            Some(draw_options.alpha),
            None,
        );
    }

    fn pop_draw_options(&mut self) {
        self.scene.pop_layer();
    }
}

impl GenericDrawTarget<VelloBackend> for DrawTarget {
    fn clear_rect(&mut self, rect: &Rect<f32>) {
        self.scene.push_blend_layer(peniko::Compose::Clear.into());
        self.scene.fill_rect(&rect.convert());
        self.scene.pop_layer();
    }

    fn copy_surface(&mut self, surface: Vec<u8>, source: Rect<i32>, destination: Point2D<i32>) {
        let destination = destination.cast::<f64>().convert();
        let rect = kurbo::Rect::from_origin_size(destination, source.size.cast::<f64>().convert());

        // TODO: ignore clip from prev layers
        // this will require creating a stacks of applicable clips
        // that will be popped and reinserted after
        // or we could impl this in vello directly

        // then there is also this nasty vello bug where clipping does not work correctly:
        // https://xi.zulipchat.com/#narrow/channel/197075-vello/topic/Servo.202D.20canvas.20backend/near/525153593

        self.scene.push_blend_layer(peniko::Compose::Copy.into());

        self.scene
            .set_paint(vello_cpu::Image::from_peniko_image(&peniko::Image {
                data: peniko::Blob::from(surface),
                format: peniko::ImageFormat::Rgba8,
                width: source.size.width as u32,
                height: source.size.height as u32,
                x_extend: peniko::Extend::Pad,
                y_extend: peniko::Extend::Pad,
                quality: peniko::ImageQuality::Low,
                alpha: 1.0,
            }));
        self.scene.fill_rect(&rect);
        self.scene.pop_layer();
    }

    fn create_similar_draw_target(&self, size: &Size2D<i32>) -> Self {
        Self::new(size.cast())
    }

    fn draw_surface(
        &mut self,
        surface: Vec<u8>,
        dest: Rect<f64>,
        source: Rect<f64>,
        filter: Filter,
        draw_options: &DrawOptions,
    ) {
        let scale_up = dest.size.width > source.size.width || dest.size.height > source.size.height;
        self.push_draw_options(draw_options);
        self.scene
            .set_paint(vello_cpu::Image::from_peniko_image(&peniko::Image {
                data: peniko::Blob::from(surface),
                format: peniko::ImageFormat::Rgba8,
                width: source.size.width as u32,
                height: source.size.height as u32,
                x_extend: peniko::Extend::Pad,
                y_extend: peniko::Extend::Pad,
                // we should only do bicubic when scaling up
                quality: if scale_up {
                    filter.convert()
                } else {
                    peniko::ImageQuality::Low
                },
                alpha: 1.0,
            }));
        self.scene.set_paint_transform(
            kurbo::Affine::translate((dest.origin.x, dest.origin.y)).pre_scale_non_uniform(
                dest.size.width / source.size.width,
                dest.size.height / source.size.height,
            ),
        );
        self.scene.fill_rect(&dest.convert());
        self.pop_draw_options();
    }

    fn draw_surface_with_shadow(
        &self,
        _surface: Vec<u8>,
        _dest: &Point2D<f32>,
        _color: &peniko::Color,
        _offset: &Vector2D<f32>,
        _sigma: f32,
        _operator: peniko::BlendMode,
    ) {
        log::warn!("no support for drawing shadows");
        /*
        We will need to do some changes to support drawing shadows with vello, as current abstraction is made for azure.
        In vello we do not need new draw target (we will use layers) and we need to pass whole rect.
        offsets will be applied to rect directly. shadow blur will be passed directly to let backend do transforms.
        */
        //self_.scene.draw_blurred_rounded_rect(self_.transform, rect, color, 0.0, sigma);
    }

    fn fill(&mut self, path: &Path, pattern: &Pattern, draw_options: &DrawOptions) {
        self.push_draw_options(draw_options);
        self.scene.set_paint(pattern.brush.clone());
        self.scene.fill_path(&path.0);
        self.pop_draw_options();
    }

    fn fill_text(
        &mut self,
        text_runs: Vec<TextRun>,
        start: Point2D<f32>,
        pattern: &Pattern,
        draw_options: &DrawOptions,
    ) {
        self.push_draw_options(draw_options);
        self.scene.set_paint(pattern.brush.clone());
        let mut advance = 0.;
        for run in text_runs.iter() {
            let glyphs = &run.glyphs;

            let template = &run.font.template;

            SHARED_FONT_CACHE.with(|font_cache| {
                let identifier = template.identifier();
                if !font_cache.borrow().contains_key(&identifier) {
                    font_cache.borrow_mut().insert(
                        identifier.clone(),
                        peniko::Font::new(
                            peniko::Blob::from(run.font.data().as_ref().to_vec()),
                            identifier.index(),
                        ),
                    );
                }

                let font_cache = font_cache.borrow();
                let Some(font) = font_cache.get(&identifier) else {
                    return;
                };

                self.scene
                    .glyph_run(font)
                    .font_size(run.font.descriptor.pt_size.to_f32_px())
                    .fill_glyphs(
                        glyphs
                            .iter_glyphs_for_byte_range(&Range::new(ByteIndex(0), glyphs.len()))
                            .map(|glyph| {
                                let glyph_offset = glyph.offset().unwrap_or(Point2D::zero());
                                let x = advance + start.x + glyph_offset.x.to_f32_px();
                                let y = start.y + glyph_offset.y.to_f32_px();
                                advance += glyph.advance().to_f32_px();
                                vello_cpu::Glyph {
                                    id: glyph.id(),
                                    x,
                                    y,
                                }
                            }),
                    );
            });
        }
        self.pop_draw_options();
    }

    fn fill_rect(&mut self, rect: &Rect<f32>, pattern: &Pattern, draw_options: &DrawOptions) {
        self.push_draw_options(draw_options);
        self.scene.set_paint(pattern.brush.clone());
        self.scene.fill_rect(&rect.convert());
        self.pop_draw_options();
    }

    fn get_size(&self) -> Size2D<i32> {
        self.size.cast()
    }

    fn get_transform(&self) -> Transform2D<f32> {
        self.scene.transform().convert()
    }

    fn pop_clip(&mut self) {
        if self.clips.pop().is_some() {
            self.scene.pop_layer();
        }
    }

    fn push_clip(&mut self, path: &Path) {
        self.clips.push(path.clone());
        self.scene.push_clip_layer(
            &path.0
        );
    }

    fn push_clip_rect(&mut self, rect: &Rect<i32>) {
        let mut path = Path::new();
        let rect = rect.cast();
        path.rect(
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        );
        self.push_clip(&path);
    }

    fn set_transform(&mut self, matrix: &Transform2D<f32>) {
        self.scene.set_transform(matrix.convert());
    }

    fn stroke(
        &mut self,
        path: &Path,
        pattern: &Pattern,
        stroke_options: &kurbo::Stroke,
        draw_options: &DrawOptions,
    ) {
        self.push_draw_options(draw_options);
        self.scene.set_paint(pattern.brush.clone());
        self.scene.set_stroke(stroke_options.clone());
        self.scene.stroke_path(&path.0);
        self.pop_draw_options();
    }

    fn stroke_rect(
        &mut self,
        rect: &Rect<f32>,
        pattern: &Pattern,
        stroke_options: &kurbo::Stroke,
        draw_options: &DrawOptions,
    ) {
        self.push_draw_options(draw_options);
        self.scene.set_paint(pattern.brush.clone());
        self.scene.set_stroke(stroke_options.clone());
        self.scene.stroke_rect(&rect.convert());
        self.pop_draw_options();
    }

    fn image_descriptor_and_serializable_data(&mut self) -> (ImageDescriptor, SerializableImageData) {
        let image_desc = ImageDescriptor {
            format: webrender_api::ImageFormat::RGBA8,
            size: self.size.cast().cast_unit(),
            stride: None,
            offset: 0,
            flags: ImageDescriptorFlags::empty(),
        };
        let data = SerializableImageData::Raw({
            let mut data = IpcSharedMemory::from_bytes(self.pixmap());
            #[allow(unsafe_code)]
            unsafe {
                pixels::generic_transform_inplace::<1, false, false>(data.deref_mut());
            };
            data
        });
        (image_desc, data)
    }

    fn snapshot(&mut self) -> pixels::Snapshot {
        Snapshot::from_vec(
            self.size.cast(),
            SnapshotPixelFormat::RGBA,
            SnapshotAlphaMode::Transparent {
                premultiplied: false,
            },
            self.pixmap().to_vec(),
        )
    }

    fn surface(&mut self) -> Vec<u8> {
        self.snapshot().to_vec(None, None).0
    }

    fn create_source_surface_from_data(&self, data: Snapshot) -> Option<Vec<u8>> {
        let (data, _, _) = data.to_vec(
            Some(SnapshotAlphaMode::Transparent {
                premultiplied: false,
            }),
            Some(SnapshotPixelFormat::RGBA),
        );
        Some(data)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DrawOptions {
    blend_mode: peniko::BlendMode,
    alpha: f32,
}

impl Default for DrawOptions {
    fn default() -> Self {
        Self {
            blend_mode: Default::default(),
            alpha: 1.,
        }
    }
}

impl DrawOptionsHelpers for DrawOptions {
    fn set_alpha(&mut self, val: f32) {
        self.alpha = val;
    }

    fn is_clear(&self) -> bool {
        self.blend_mode.compose == peniko::Compose::Clear
    }
}

impl StrokeOptionsHelpers for kurbo::Stroke {
    fn set_line_width(&mut self, val: f32) {
        self.width = val as f64;
    }

    fn set_miter_limit(&mut self, val: f32) {
        self.miter_limit = val as f64;
    }

    fn set_line_join(&mut self, val: LineJoinStyle) {
        self.join = val.convert()
    }

    fn set_line_cap(&mut self, val: LineCapStyle) {
        self.start_cap = val.convert();
        self.end_cap = val.convert();
    }

    fn set_line_dash(&mut self, items: Vec<f32>) {
        self.dash_pattern = items.iter().map(|x| *x as f64).collect();
    }

    fn set_line_dash_offset(&mut self, offset: f32) {
        self.dash_offset = offset as f64;
    }
}

#[derive(Clone)]
pub(crate) struct Pattern {
    brush: vello_cpu::PaintType,
    repeat_x: bool,
    repeat_y: bool,
}

impl Pattern {
    fn new(brush: vello_cpu::PaintType) -> Self {
        Self {
            brush,
            repeat_x: false,
            repeat_y: false,
        }
    }
}

impl PatternHelpers for Pattern {
    fn is_zero_size_gradient(&self) -> bool {
        match &self.brush {
            vello_cpu::PaintType::Gradient(gradient) => {
                if gradient.stops.is_empty() {
                    return true;
                }
                match gradient.kind {
                    peniko::GradientKind::Linear { start, end } => start == end,
                    peniko::GradientKind::Radial {
                        start_center,
                        start_radius,
                        end_center,
                        end_radius,
                    } => start_center == end_center && start_radius == end_radius,
                    peniko::GradientKind::Sweep {
                        center: _,
                        start_angle,
                        end_angle,
                    } => start_angle == end_angle,
                }
            },
            vello_cpu::PaintType::Image(_) | vello_cpu::PaintType::Solid(_) => false,
        }
    }

    fn x_bound(&self) -> Option<u32> {
        match &self.brush {
            vello_cpu::PaintType::Image(vello_cpu::Image {
                source: vello_cpu::ImageSource::Pixmap(image),
                ..
            }) => {
                if self.repeat_x {
                    None
                } else {
                    Some(image.width().into())
                }
            },
            _ => None,
        }
    }

    fn y_bound(&self) -> Option<u32> {
        match &self.brush {
            vello_cpu::PaintType::Image(vello_cpu::Image {
                source: vello_cpu::ImageSource::Pixmap(image),
                ..
            }) => {
                if self.repeat_y {
                    None
                } else {
                    Some(image.height().into())
                }
            },
            _ => None,
        }
    }
}

/// A version of the `Into<T>` trait from the standard library that can be used
/// to convert between two types that are not defined in the canvas crate.
pub(crate) trait Convert<T> {
    fn convert(self) -> T;
}

impl Convert<kurbo::Point> for Point2D<f32> {
    fn convert(self) -> kurbo::Point {
        kurbo::Point {
            x: self.x as f64,
            y: self.y as f64,
        }
    }
}

impl Convert<kurbo::Point> for Point2D<f64> {
    fn convert(self) -> kurbo::Point {
        kurbo::Point {
            x: self.x,
            y: self.y,
        }
    }
}

impl Convert<Point2D<f32>> for kurbo::Point {
    fn convert(self) -> Point2D<f32> {
        Point2D::new(self.x, self.y).cast()
    }
}

impl Convert<Transform2D<f32>> for kurbo::Affine {
    fn convert(self) -> Transform2D<f32> {
        Transform2D::from_array(self.as_coeffs()).cast()
    }
}

impl Convert<kurbo::Affine> for Transform2D<f32> {
    fn convert(self) -> kurbo::Affine {
        kurbo::Affine::new(self.cast().to_array())
    }
}

impl Convert<kurbo::Join> for LineJoinStyle {
    fn convert(self) -> kurbo::Join {
        match self {
            LineJoinStyle::Round => kurbo::Join::Round,
            LineJoinStyle::Bevel => kurbo::Join::Bevel,
            LineJoinStyle::Miter => kurbo::Join::Miter,
        }
    }
}

impl Convert<kurbo::Cap> for LineCapStyle {
    fn convert(self) -> kurbo::Cap {
        match self {
            LineCapStyle::Butt => kurbo::Cap::Butt,
            LineCapStyle::Round => kurbo::Cap::Round,
            LineCapStyle::Square => kurbo::Cap::Square,
        }
    }
}

impl Convert<peniko::Color> for AbsoluteColor {
    fn convert(self) -> peniko::Color {
        let srgb = self.into_srgb_legacy();
        peniko::Color::new([
            srgb.components.0,
            srgb.components.1,
            srgb.components.2,
            srgb.alpha,
        ])
    }
}

impl Convert<peniko::BlendMode> for CompositionOrBlending {
    fn convert(self) -> peniko::BlendMode {
        match self {
            CompositionOrBlending::Composition(composition_style) => {
                composition_style.convert().into()
            },
            CompositionOrBlending::Blending(blending_style) => blending_style.convert().into(),
        }
    }
}

impl Convert<peniko::Compose> for CompositionStyle {
    fn convert(self) -> peniko::Compose {
        match self {
            CompositionStyle::SourceIn => peniko::Compose::SrcIn,
            CompositionStyle::SourceOut => peniko::Compose::SrcOut,
            CompositionStyle::SourceOver => peniko::Compose::SrcOver,
            CompositionStyle::SourceAtop => peniko::Compose::SrcAtop,
            CompositionStyle::DestinationIn => peniko::Compose::DestIn,
            CompositionStyle::DestinationOut => peniko::Compose::DestOut,
            CompositionStyle::DestinationOver => peniko::Compose::DestOver,
            CompositionStyle::DestinationAtop => peniko::Compose::DestAtop,
            CompositionStyle::Copy => peniko::Compose::Copy,
            CompositionStyle::Lighter => peniko::Compose::Plus,
            CompositionStyle::Xor => peniko::Compose::Xor,
            CompositionStyle::Clear => peniko::Compose::Clear,
        }
    }
}

impl Convert<peniko::Mix> for BlendingStyle {
    fn convert(self) -> peniko::Mix {
        match self {
            BlendingStyle::Multiply => peniko::Mix::Multiply,
            BlendingStyle::Screen => peniko::Mix::Screen,
            BlendingStyle::Overlay => peniko::Mix::Overlay,
            BlendingStyle::Darken => peniko::Mix::Darken,
            BlendingStyle::Lighten => peniko::Mix::Lighten,
            BlendingStyle::ColorDodge => peniko::Mix::ColorDodge,
            BlendingStyle::ColorBurn => peniko::Mix::ColorBurn,
            BlendingStyle::HardLight => peniko::Mix::HardLight,
            BlendingStyle::SoftLight => peniko::Mix::SoftLight,
            BlendingStyle::Difference => peniko::Mix::Difference,
            BlendingStyle::Exclusion => peniko::Mix::Exclusion,
            BlendingStyle::Hue => peniko::Mix::Hue,
            BlendingStyle::Saturation => peniko::Mix::Saturation,
            BlendingStyle::Color => peniko::Mix::Color,
            BlendingStyle::Luminosity => peniko::Mix::Luminosity,
        }
    }
}

impl Convert<Pattern> for FillOrStrokeStyle {
    fn convert(self) -> Pattern {
        use canvas_traits::canvas::FillOrStrokeStyle::*;
        match self {
            Color(absolute_color) => {
                Pattern::new(vello_cpu::PaintType::Solid(absolute_color.convert()))
            },
            LinearGradient(style) => {
                let start = kurbo::Point::new(style.x0, style.y0);
                let end = kurbo::Point::new(style.x1, style.y1);
                let mut gradient = peniko::Gradient::new_linear(start, end);
                gradient.stops = style.stops.convert();
                Pattern::new(vello_cpu::PaintType::Gradient(gradient))
            },
            RadialGradient(style) => {
                let center1 = kurbo::Point::new(style.x0, style.y0);
                let center2 = kurbo::Point::new(style.x1, style.y1);
                let mut gradient = peniko::Gradient::new_two_point_radial(
                    center1,
                    style.r0 as f32,
                    center2,
                    style.r1 as f32,
                );
                gradient.stops = style.stops.convert();
                Pattern::new(vello_cpu::PaintType::Gradient(gradient))
            },
            Surface(surface_style) => {
                let data = surface_style
                    .surface_data
                    .to_owned()
                    .to_vec(
                        Some(SnapshotAlphaMode::Transparent {
                            premultiplied: false,
                        }),
                        Some(SnapshotPixelFormat::RGBA),
                    )
                    .0;
                Pattern {
                    brush: vello_cpu::PaintType::Image(vello_cpu::Image::from_peniko_image(
                        &peniko::Image {
                            data: peniko::Blob::from(data),
                            format: peniko::ImageFormat::Rgba8,
                            width: surface_style.surface_size.width,
                            height: surface_style.surface_size.height,
                            x_extend: if surface_style.repeat_x {
                                peniko::Extend::Repeat
                            } else {
                                peniko::Extend::Pad
                            },
                            y_extend: if surface_style.repeat_y {
                                peniko::Extend::Repeat
                            } else {
                                peniko::Extend::Pad
                            },
                            quality: peniko::ImageQuality::Low,
                            alpha: 1.0,
                        },
                    )),
                    repeat_x: surface_style.repeat_x,
                    repeat_y: surface_style.repeat_y,
                }
            },
        }
    }
}

impl Convert<peniko::color::DynamicColor> for AbsoluteColor {
    fn convert(self) -> peniko::color::DynamicColor {
        peniko::color::DynamicColor::from_alpha_color(self.convert())
    }
}

impl Convert<peniko::ColorStop> for CanvasGradientStop {
    fn convert(self) -> peniko::ColorStop {
        peniko::ColorStop {
            offset: self.offset as f32,
            color: self.color.convert(),
        }
    }
}

impl Convert<peniko::ColorStops> for Vec<CanvasGradientStop> {
    fn convert(self) -> peniko::ColorStops {
        let mut stops = peniko::ColorStops(self.into_iter().map(|item| item.convert()).collect());
        // https://www.w3.org/html/test/results/2dcontext/annotated-spec/canvas.html#testrefs.2d.gradient.interpolate.overlap
        stops
            .0
            .sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap());
        stops
    }
}

impl Convert<kurbo::Size> for Size2D<f32> {
    fn convert(self) -> kurbo::Size {
        kurbo::Size::new(self.width as f64, self.height as f64)
    }
}

impl Convert<kurbo::Size> for Size2D<f64> {
    fn convert(self) -> kurbo::Size {
        kurbo::Size::new(self.width, self.height)
    }
}

impl Convert<kurbo::Size> for Size2D<u32> {
    fn convert(self) -> kurbo::Size {
        kurbo::Size::new(self.width as f64, self.height as f64)
    }
}

impl Convert<kurbo::Rect> for Rect<f32> {
    fn convert(self) -> kurbo::Rect {
        kurbo::Rect::from_center_size(self.center().convert(), self.size.convert())
    }
}

impl Convert<kurbo::Rect> for Rect<f64> {
    fn convert(self) -> kurbo::Rect {
        kurbo::Rect::from_center_size(self.center().convert(), self.size.convert())
    }
}

impl Convert<peniko::ImageQuality> for Filter {
    fn convert(self) -> peniko::ImageQuality {
        match self {
            Filter::Bilinear => peniko::ImageQuality::Medium,
            Filter::Nearest => peniko::ImageQuality::Low,
        }
    }
}
