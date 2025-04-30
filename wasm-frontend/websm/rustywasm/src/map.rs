use std::thread::current;

use eframe::{
    egui::{Color32, Painter, Pos2, Response, Sense, Shape, Stroke, Ui, Vec2},
    epaint::{CircleShape, PathShape, PathStroke},
};
use geo_types::{Coord, LineString, Point, coord};
use itertools::Itertools;
use nalgebra::{ComplexField, Rotation2, Similarity2, Vector2};
use rstar::AABB;
use wasm_bindgen::JsValue;

use crate::{PrintDebug, state::StateHandle};

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
    pub fn linestring(&self, ls: &LineString<f64>, stroke: PathStroke) -> &Self {
        let points = ls.coords().map(vec2_from_coord);
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

pub trait Cartograph {
    fn cartograph(&mut self, state: StateHandle) -> Response;
}

impl Cartograph for Ui {
    fn cartograph(&mut self, state: StateHandle) -> Response {
        /*
        let road = vec![
            coord! {x:57.011908, y:9.990876},
            coord! {x:57.011908, y:9.991820},
            coord! {x:57.011749, y:9.991776},
        ];
        */
        let w = self.available_width();
        let h = self.available_height();

        let (response, painter) = self.allocate_painter(Vec2::new(w, h), Sense::empty());
        let rect = response.rect;
        let mut transform = state.transform.lock().unwrap();

        if let Some(multi_touch) = response.ctx.multi_touch() {
            transform.scale(multi_touch.zoom_delta);
            transform.rotate(multi_touch.rotation_delta);
        }

        let user_location = state.user_location.lock().unwrap();
        //let user_location = road;
        let Some(current_coord) = user_location.last() else {
            return response;
        };

        let t = &*transform;

        let current = vec2_from_coord(current_coord).print_debug("Current");
        let center = vec2_from_pos2(&rect.center());
        let i = t.inverse();

        let transform_fn = move |v: &Vector2<f32>| **t * (v - current) + center;
        let inverse_fn = move |v: &Vector2<f32>| i * (v - center) + current;

        let draw_navigation_layer = painter.draw_transformed(transform_fn, h);

        // Center Circle
        draw_navigation_layer.circle(&current, 10.0, Color32::GOLD, Stroke::NONE);

        // Draw path if multiple points
        if user_location.len() > 1 {
            let points = user_location.iter().map(vec2_from_coord);
            draw_navigation_layer.path(points, PathStroke::new(3.0, Color32::BLUE));
        }

        // Start query road network
        let tl = inverse_fn(&vec2_from_pos2(&rect.left_top()));
        let tr = inverse_fn(&vec2_from_pos2(&rect.right_top()));
        let bl = inverse_fn(&vec2_from_pos2(&rect.left_bottom()));
        let br = inverse_fn(&vec2_from_pos2(&rect.right_bottom()));

        let top = Vector2::new(
            tl.x.max(tr.x).max(bl.x).max(br.x),
            tl.y.max(tr.y).max(bl.y).max(br.y),
        );
        let bottom = Vector2::new(
            tl.x.min(tr.x).min(bl.x).min(br.x),
            tl.y.min(tr.y).min(bl.y).min(br.y),
        );

        let roads = state.road_index.lock().unwrap();
        // Query and draw road network
        for road in roads.box_query(&AABB::from_corners(
            point_from_vec2(&top),
            point_from_vec2(&bottom),
        )) {
            draw_navigation_layer.linestring(road.geom(), PathStroke::new(1.0, Color32::RED));
        }

        response
    }
}

pub trait CompasRose {
    fn compas_rose(&mut self, state: StateHandle) -> Response;
}

impl CompasRose for Ui {
    fn compas_rose(&mut self, state: StateHandle) -> Response {
        let h = self.available_height();
        let (response, painter) = self.allocate_painter(Vec2::new(h, h), Sense::click());
        let center = vec2_from_pos2(&response.rect.center());
        let mut t = state.transform.lock().unwrap();

        if response.clicked() {
            t.reset_rotation();
        }

        let scale = h / 2.0;
        let mut transform = Similarity2::from_scaling(scale);
        transform.append_rotation_mut(&Rotation2::new(t.get_rotation()).into());

        let draw_compas_layer =
            painter.draw_transformed(|x: &Vector2<f32>| transform * x + center, h);
        draw_compas_layer.circle(
            &Vector2::new(0.0, 0.0),
            scale,
            Color32::DARK_BLUE,
            Stroke::default(),
        );

        const ARROW_ANGLE: f32 = 60f32.to_radians();
        draw_compas_layer.polygon(
            [
                Vector2::new(0.0, -1.0),
                Vector2::new(-ARROW_ANGLE.cos(), ARROW_ANGLE.sin()),
                Vector2::new(0.0, 0.5),
            ]
            .into_iter(),
            PathStroke::default(),
            Color32::RED,
        );
        draw_compas_layer.polygon(
            [
                Vector2::new(0.0, 0.5),
                Vector2::new(ARROW_ANGLE.cos(), ARROW_ANGLE.sin()),
                Vector2::new(0.0, -1.0),
            ]
            .into_iter(),
            PathStroke::default(),
            Color32::RED,
        );

        response
    }
}
