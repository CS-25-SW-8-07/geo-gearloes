const ESPG_FROM: &str = "EPSG:4326";
const ESPG_TO: &str = "EPSG:2196";

use eframe::{
    egui::{Color32, Id, Painter, Pos2, Shape, Stroke, Widget},
    epaint::{CircleShape, PathShape, PathStroke},
};
use geo::{Coord, LineString, Point};
use nalgebra::Vector2;
use proj::Proj;
use rstar::{AABB, primitives::GeomWithData};

use crate::{NORTH_EAST, SOUTH_WEST};

#[inline]
fn pos2_from_vec2(v: &Vector2<f32>) -> Pos2 {
    Pos2 { x: v.x, y: v.y }
}

#[inline]
fn vec2_from_pos2(v: &Pos2) -> Vector2<f32> {
    Vector2::new(v.x, v.y)
}

#[inline]
fn point_from_vec2(v: &Vector2<f32>) -> Point {
    Point::new(v.x as f64, v.y as f64)
}

#[inline]
fn vec2_from_coord(v: &Coord<f64>) -> Vector2<f32> {
    Vector2::new(v.x as f32, v.y as f32)
}

struct Draw<'a, F: Fn(&Vector2<f32>) -> Vector2<f32>> {
    painter: &'a Painter,
    height: f32,
    transform_fn: F,
}

impl<'a, F: Fn(&Vector2<f32>) -> Vector2<f32>> Draw<'a, F> {
    #[inline]
    pub fn circle(
        &self,
        center: &Vector2<f32>,
        radius: f32,
        fill: Color32,
        stroke: Stroke,
    ) -> &Self {
        self.painter.add(Shape::Circle(CircleShape {
            center: pos2_from_vec2(&self.transform(center)),
            radius,
            fill,
            stroke,
        }));
        self
    }

    #[inline]
    fn raw_path<I: Iterator<Item = Vector2<f32>>>(
        &self,
        points: I,
        stroke: PathStroke,
        fill: Color32,
        closed: bool,
    ) -> &Self {
        self.painter.add(Shape::Path(PathShape {
            points: points
                .map(|x| self.transform(&x))
                .map(|x| pos2_from_vec2(&x))
                .collect(),
            closed,
            fill,
            stroke,
        }));
        self
    }

    #[inline]
    pub fn path<I: Iterator<Item = Vector2<f32>>>(&self, points: I, stroke: PathStroke) -> &Self {
        self.raw_path(points, stroke, Color32::TRANSPARENT, false)
    }

    #[inline]
    pub fn polygon<I: Iterator<Item = Vector2<f32>>>(
        &self,
        points: I,
        stroke: PathStroke,
        fill: Color32,
    ) -> &Self {
        self.raw_path(points, stroke, fill, true)
    }

    #[inline]
    pub fn linestring(
        &self,
        ls: &LineString,
        stroke: PathStroke,
        transform: impl Fn(Point) -> Point,
    ) -> &Self {
        let points = ls.points().map(transform).map(|p| vec2_from_coord(&p.0));
        self.path(points, stroke)
    }

    #[inline]
    fn transform(&self, vec: &Vector2<f32>) -> Vector2<f32> {
        self.invert_y((self.transform_fn)(&vec))
    }

    #[inline]
    fn invert_y(&self, mut vec: Vector2<f32>) -> Vector2<f32> {
        vec.y = self.height - vec.y;
        vec
    }
}

trait DrawTransformed<'a, F: Fn(&Vector2<f32>) -> Vector2<f32> + 'a> {
    fn draw_transformed(&'a self, transform_fn: F, height: f32) -> Draw<'a, F>;
}

impl<'a, F: Fn(&Vector2<f32>) -> Vector2<f32> + 'a> DrawTransformed<'a, F> for Painter {
    fn draw_transformed(&'a self, transform_fn: F, height: f32) -> Draw<'a, F> {
        Draw {
            painter: self,
            height,
            transform_fn,
        }
    }
}

pub struct Map;

impl Widget for Map {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let Some(index) = ui.data(|r| r.get_temp::<rusty_roads::RoadIndex>(Id::NULL)) else {
            return ui.response();
        };

        let roads = index.box_query(&AABB::from_corners(NORTH_EAST.into(), SOUTH_WEST.into()));

        let projection = Proj::new_known_crs(ESPG_FROM, ESPG_TO, None).unwrap();

        let north_east = Point::from(projection.project(NORTH_EAST, true).unwrap());
        let south_west = Point::from(projection.project(SOUTH_WEST, true).unwrap());

        let vw = ui.available_width() as f64;
        let vh = ui.available_height() as f64;
        let va = (vw / vh) as f64;

        let c = south_west - north_east;
        let w = c.x();
        let h = c.y();
        let a = (w / h) as f64;

        dbg!(w, h, a, vw, vh, va);

        let transform = if a > va {
            Box::new(|v: Point| {
                let p = south_west - projection.project(v, true).unwrap();
                p / w * vw
            }) as Box<dyn Fn(Point) -> Point>
        } else {
            Box::new(|v: Point| {
                let p = south_west - projection.project(v, true).unwrap();
                p / h * vh
            }) as Box<dyn Fn(Point) -> Point>
        };

        let painter = ui.painter();
        let draw = painter.draw_transformed(Clone::clone, vh as f32);

        let transform = &transform;

        roads.map(GeomWithData::geom).for_each(|geom| {
            draw.linestring(geom, PathStroke::new(1.0, Color32::DARK_GRAY), transform);
        });

        ui.response()
    }
}
