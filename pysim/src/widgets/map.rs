use std::{
    convert::identity,
    iter::repeat,
    sync::{Arc, RwLock},
};

use eframe::{
    egui::{self, Color32, Id, Painter, Pos2, Shape, Stroke, Widget},
    epaint::{CircleShape, PathShape, PathStroke},
};
use geo::{Coord, LineString, Point, Translate};
use proj::Proj;
use rstar::{AABB, primitives::GeomWithData};
use rusty_roads::RoadIndex;

use crate::sim::{BBox, Cars, Projection, Time};

struct Draw<'a, F: Fn(Pos2) -> Pos2> {
    painter: &'a Painter,
    height: f32,
    transform_fn: F,
}

impl<'a, F: Fn(Pos2) -> Pos2> Draw<'a, F> {
    #[inline]
    pub fn circle(&self, center: Pos2, radius: f32, fill: Color32, stroke: Stroke) -> &Self {
        self.painter.add(Shape::Circle(CircleShape {
            center,
            radius,
            fill,
            stroke,
        }));
        self
    }

    #[inline]
    fn raw_path<I: Iterator<Item = Pos2>>(
        &self,
        points: I,
        stroke: PathStroke,
        fill: Color32,
        closed: bool,
    ) -> &Self {
        self.painter.add(Shape::Path(PathShape {
            points: points.map(&self.transform_fn).collect(),
            closed,
            fill,
            stroke,
        }));
        self
    }

    #[inline]
    pub fn path<I: Iterator<Item = Pos2>>(&self, points: I, stroke: PathStroke) -> &Self {
        self.raw_path(points, stroke, Color32::TRANSPARENT, false)
    }

    #[inline]
    pub fn polygon<I: Iterator<Item = Pos2>>(
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
        let points = ls
            .points()
            .map(transform)
            .map(|p| Pos2::new(p.0.x as f32, p.0.y as f32));
        self.path(points, stroke)
    }

    #[inline]
    fn invert_y(&self, mut vec: Pos2) -> Pos2 {
        vec.y = self.height - vec.y;
        vec
    }
}

trait DrawTransformed<'a, F: Fn(Pos2) -> Pos2 + 'a> {
    fn draw_transformed(&'a self, transform_fn: F, height: f32) -> Draw<'a, F>;
}

impl<'a, F: Fn(Pos2) -> Pos2 + 'a> DrawTransformed<'a, F> for Painter {
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
        let BBox(min, max) = ui.data(|r| r.get_temp(Id::NULL).unwrap());
        let roads_tmp = ui
            .data(|r| r.get_temp::<Option<Arc<RoadIndex>>>(Id::NULL))
            .flatten();
        let roads = roads_tmp
            .as_ref()
            .map(|index| index.box_query(&AABB::from_corners(min, max)));

        let cars = ui.data(|r| r.get_temp::<Cars>(Id::NULL).unwrap());
        let Time(time) = ui.data(|r| r.get_temp::<Time>(Id::NULL).unwrap());

        let projection: Projection = ui.data(|r| r.get_temp(Id::NULL).unwrap());

        let vw = ui.available_width() as f64;
        let vh = ui.available_height() as f64;
        let va = (vw / vh) as f64;

        let c = max - min;
        let w = c.x();
        let h = c.y();
        let a = (w / h) as f64;

        let transform: Box<dyn Fn(Point) -> Point> = if a > va {
            Box::new(|v: Point| (max - v) / w * vw)
        } else {
            Box::new(|v: Point| (max - v) / h * vh)
        };

        let painter = ui.painter();
        let draw = painter.draw_transformed(
            |Pos2 { x, y }| Pos2 {
                x: ui.available_width() - x,
                y,
            },
            vh as f32,
        );

        let transform = &transform;

        roads
            .into_iter()
            .flatten()
            .map(GeomWithData::geom)
            .for_each(|geom| {
                draw.linestring(geom, PathStroke::new(1.0, Color32::LIGHT_GRAY), transform);
            });

        cars.get()
            .iter()
            .filter(|car| car.should_draw(time))
            .flat_map(|car| {
                [
                    /*
                    (
                        LineString::from_iter(
                            car.drive
                                .points
                                .iter()
                                .map(|p| projection.project(p.0, true).unwrap()),
                        ),
                        Color32::DARK_GREEN,
                    ),
                    */
                    (
                        LineString::from_iter(
                            car.record
                                .points
                                .iter()
                                .map(|p| projection.project(p.0, true).unwrap()),
                        ),
                        car.color,
                    ),
                    (
                        LineString::from_iter(
                            car.predicted
                                .points
                                .iter()
                                .map(|p| projection.project(p.0, true).unwrap()),
                        ),
                        Color32::DARK_RED,
                    ),
                ]
            })
            .for_each(|(ls, color)| {
                // Draw Trajectories
                draw.linestring(&ls, PathStroke::new(0.5, color), transform);
            });

        ui.response()
    }
}
