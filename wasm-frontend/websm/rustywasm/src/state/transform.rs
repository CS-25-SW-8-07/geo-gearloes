use std::ops::Deref;

use nalgebra::{Rotation2, Translation2, Vector2, geometry::Similarity2};

pub const MAX_SCALE: f32 = DEF_SCALE * 10f32;
pub const DEF_SCALE: f32 = 100_000f32;
pub const MIN_SCALE: f32 = DEF_SCALE / 10f32;

#[derive(Debug)]
pub struct Transform {
    rotation: f32,
    scale: f32,
    pub transform: Similarity2<f32>,
}

impl Transform {
    pub fn translate(&mut self, v: Vector2<f32>) {
        self.transform
            .append_translation_mut(&Translation2::from(v).into());
    }

    pub fn scale(&mut self, factor: f32) {
        self.scale *= factor;
        self.transform.append_scaling_mut(factor);
    }

    pub fn rotate(&mut self, rad: f32) {
        self.rotation += rad;
        self.transform
            .append_rotation_mut(&Rotation2::new(rad).into());
    }

    pub fn reset_rotation(&mut self) {
        self.rotate(-self.rotation);
        self.rotation = 0.0;
    }

    pub fn reset_scale(&mut self) {
        self.scale(1f32 / self.scale);
        self.scale = 0.0;
    }

    pub fn get_rotation(&self) -> f32 {
        self.rotation
    }

    pub fn get_scale(&self) -> f32 {
        self.scale
    }
}

impl Default for Transform {
    fn default() -> Self {
        let mut slf = Self {
            rotation: 0.0,
            scale: 0.0,
            transform: Default::default(),
        };
        slf.scale(DEF_SCALE);
        slf
    }
}

impl Deref for Transform {
    type Target = Similarity2<f32>;
    fn deref(&self) -> &Self::Target {
        &self.transform
    }
}
