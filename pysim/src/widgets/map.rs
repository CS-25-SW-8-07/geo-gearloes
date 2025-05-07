use std::{convert::identity, sync::Arc};

use eframe::{
    egui::{self, Color32, Id, Painter, Pos2, Shape, Stroke, Widget},
    epaint::{CircleShape, PathShape, PathStroke},
};
use geo::{Coord, LineString, Point, Translate};
use proj::Proj;
use rstar::{primitives::GeomWithData, AABB};
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
        let index = ui.data(|r| r.get_temp::<Arc<RoadIndex>>(Id::NULL).unwrap());
        let cars = ui.data(|r| r.get_temp::<Cars>(Id::NULL).unwrap());
        let Time(time) = ui.data(|r| r.get_temp::<Time>(Id::NULL).unwrap());

        let projection: Projection = ui.data(|r| r.get_temp(Id::NULL).unwrap());

        let BBox(min, max) = ui.data(|r| r.get_temp(Id::NULL).unwrap());

        let roads = index.box_query(&AABB::from_corners(min, max));

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

        dbg!(c, w, h, a, vw, vh, max, transform(max), min, transform(min));

        let painter = ui.painter();
        let draw = painter.draw_transformed(
            |Pos2 { x, y }| Pos2 {
                x: ui.available_width() - x,
                y,
            },
            vh as f32,
        );

        let transform = &transform;

        roads.map(GeomWithData::geom).for_each(|geom| {
            draw.linestring(geom, PathStroke::new(1.0, Color32::LIGHT_GRAY), transform);
        });

        cars.iter()
            .map(|car| {
                let pts = car
                    .trajectory
                    .timestamps
                    .windows(2)
                    .zip(car.trajectory.points.windows(2))
                    .filter_map(|(t, p)| {
                        if t[0] < time {
                            Some(p[0])
                        } else if t[0] >= time {
                            None
                        } else {
                            let rate = (time.as_secs_f64() - t[0].as_secs_f64())
                                / (t[1].as_secs_f64() - t[0].as_secs_f64());
                            let translation = (p[1] - p[0]) * rate;
                            Some(p[0].translate(translation.x(), translation.y()))
                        }
                    })
                    .map(|p| projection.project(p, true).unwrap());
                LineString::from_iter(pts)
            })
            .for_each(|ls| {
                draw.linestring(&ls, PathStroke::new(0.5, Color32::DARK_BLUE), transform);
            });

        ui.response()
    }
}
