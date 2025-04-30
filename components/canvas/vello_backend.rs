use canvas_traits::canvas::{CompositionOrBlending, FillOrStrokeStyle};
use euclid::default::Size2D;
use style::color::AbsoluteColor;

use crate::canvas_data::{
    self, Backend, CanvasPaintState, Color, CompositionOp, DrawOptions, Filter, GenericDrawTarget,
    GenericPathBuilder, GradientStop, GradientStops, Path, SourceSurface, StrokeOptions, TextRun,
};

use vello::kurbo::{self, Shape as _};

#[derive(Default)]
pub struct VelloBackend;

impl Backend for VelloBackend {
    fn get_composition_op(&self, opts: &DrawOptions) -> CompositionOp {
        CompositionOp::Raqote(opts.as_raqote().blend_mode)
    }

    fn need_to_draw_shadow(&self, color: &Color) -> bool {
        color.as_raqote().a != 0
    }

    fn set_shadow_color(&mut self, color: AbsoluteColor, state: &mut CanvasPaintState<'_>) {
        state.shadow_color = Color::Raqote(color.to_raqote_style());
    }

    fn set_fill_style(
        &mut self,
        style: FillOrStrokeStyle,
        state: &mut CanvasPaintState<'_>,
        _drawtarget: &dyn GenericDrawTarget,
    ) {
        if let Some(pattern) = style.to_raqote_pattern() {
            state.fill_style = canvas_data::Pattern::Raqote(pattern);
        }
    }

    fn set_stroke_style(
        &mut self,
        style: FillOrStrokeStyle,
        state: &mut CanvasPaintState<'_>,
        _drawtarget: &dyn GenericDrawTarget,
    ) {
        if let Some(pattern) = style.to_raqote_pattern() {
            state.stroke_style = canvas_data::Pattern::Raqote(pattern);
        }
    }

    fn set_global_composition(
        &mut self,
        op: CompositionOrBlending,
        state: &mut CanvasPaintState<'_>,
    ) {
        state.draw_options.as_raqote_mut().blend_mode = op.to_raqote_style();
    }

    fn create_drawtarget(&self, size: Size2D<u64>) -> Box<dyn GenericDrawTarget> {
        Box::new(raqote::DrawTarget::new(
            size.width as i32,
            size.height as i32,
        ))
    }

    fn recreate_paint_state<'a>(&self, _state: &CanvasPaintState<'a>) -> CanvasPaintState<'a> {
        CanvasPaintState::default()
    }
}

struct PathBuilder(kurbo::BezPath);

impl PathBuilder {
    fn new() -> PathBuilder {
        PathBuilder(kurbo::BezPath::new())
    }
}

impl GenericPathBuilder for PathBuilder {
    fn arc(
        &mut self,
        origin: lyon_geom::Point<f32>,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        anticlockwise: bool,
    ) {
        kurbo::Arc::new(center, radius, start_angle, start_angle - end_angle, 0).to_cubic_beziers(tolerance, p);
    }

    fn bezier_curve_to(
        &mut self,
        p1: &lyon_geom::Point<f32>,
        p2: &lyon_geom::Point<f32>,
        p3: &lyon_geom::Point<f32>,
    ) {
        self.0.curve_to(p1.convert(), p2.convert(), p3.convert());
    }

    fn close(&mut self) {
        self.0.close_path();
    }

    fn ellipse(
        &mut self,
        origin: lyon_geom::Point<f32>,
        radius_x: f32,
        radius_y: f32,
        rotation_angle: f32,
        start_angle: f32,
        end_angle: f32,
        anticlockwise: bool,
    ) {

    }

    fn get_current_point(&mut self) -> Option<lyon_geom::Point<f32>> {
        self.0.elements().last().and_then(|last| last.end_point().map(Convert::convert))
    }

    fn line_to(&mut self, point: lyon_geom::Point<f32>) {
        self.0.line_to(point.convert());
    }

    fn move_to(&mut self, point: lyon_geom::Point<f32>) {
        self.0.move_to(point.convert());
    }

    fn quadratic_curve_to(&mut self, p1: &lyon_geom::Point<f32>, p2: &lyon_geom::Point<f32>) {
        self.0.quad_to(p1.convert(), p2.convert());
    }

    fn svg_arc(
        &mut self,
        radius_x: f32,
        radius_y: f32,
        rotation_angle: f32,
        large_arc: bool,
        sweep: bool,
        end_point: lyon_geom::Point<f32>,
    ) {
        let sarc = kurbo::SvgArc { from: self.get_current_point().unwrap().convert(), to: end_point.convert(), radii: kurbo::Vec2::new(radius_x as f64, radius_y as f64), x_rotation: rotation_angle as f64, large_arc, sweep };
        let arc = kurbo::Arc::from_svg_arc(&sarc).unwrap();
        self.0.extend(arc.path_elements(0.1));
    }

    fn finish(&mut self) -> Path {
        todo!()
    }
}

pub(crate) trait Convert<T> {
    fn convert(self) -> T;
}

impl Convert<kurbo::Point> for lyon_geom::Point<f32> {
    fn convert(self) -> kurbo::Point {
        self.cast::<f64>().convert()
    }
}

impl Convert<kurbo::Point> for lyon_geom::Point<f64> {
    fn convert(self) -> kurbo::Point {
        kurbo::Point { x: self.x, y: self.y }
    }
}

impl Convert<lyon_geom::Point<f64>> for kurbo::Point {
    fn convert(self) -> lyon_geom::Point<f64> {
        lyon_geom::Point::new(self.x, self.y)
    }
}

impl Convert<lyon_geom::Point<f32>> for kurbo::Point {
    fn convert(self) -> lyon_geom::Point<f32> {
        let t: lyon_geom::Point<f64> = self.convert();
        t.cast()
    }
}