use std::num::TryFromIntError;

use geo::GeodesicArea;
use geo::Scale;
use geo_types::LineString;
use geo_types::{Coord, Rect};
use rand::prelude::*;
use rstar::AABB;
use rusty_roads::AnonymityConf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnonymityError {
    #[error("Float could not be converted")]
    ConversionError,
}

/// The input [`current_k`] should only contain k's for rows that are being visited.
pub fn evaluate_route_anonymity<'a>(
    anon_conf: &AnonymityConf,
    current_k: impl IntoIterator<Item = impl Into<&'a f64> + Copy> + Clone,
) -> Result<bool, TryFromIntError> {
    let min_per = anon_conf.min_k_percentile;
    let min_k = anon_conf.min_k;

    let count: u64 = current_k.clone().into_iter().count().try_into()?;
    let below_k: u64 = current_k
        .into_iter()
        .filter(|x| *(*x).into() >= min_k as f64)
        .count()
        .try_into()?;

    let percentile = below_k as f64 / count as f64;

    Ok(percentile > min_per)
}

/// Function which calculates the aabb of a trajectory based on user configuration.
pub fn calculate_aabb(
    anon_conf: &AnonymityConf,
    trajectory: &LineString<f64>,
) -> Option<AABB<Coord>> {
    if trajectory.0.is_empty() {
        return None;
    }

    let mut aabb: AABB<Coord> = AABB::from_points(trajectory);

    let min_size = anon_conf.min_area_size;

    let rectangle = Rect::new(aabb.lower(), aabb.upper());

    let lower = aabb.lower();
    let upper = aabb.upper();

    let mut rng = rand::rng();

    if rectangle.geodesic_area_unsigned() < min_size {
        // Calculate the minimum aspect ratio scaling (scalar * aspect_ratio must not be less than 1)
        let min_aspect_ratio: f64 =
            rectangle.geodesic_area_unsigned() / (rectangle.geodesic_area_unsigned() + min_size);

        // Create random scaleable aspect ratio for hiding how the trajectory looks.
        let aspect_ratio: f64 = rng.random_range(min_aspect_ratio..=(1.0 - min_aspect_ratio));

        // Calculate how much to scale the aabb based on anonymity configuration
        let scalar = (min_size
            / (rectangle.geodesic_area_unsigned() * aspect_ratio * (1. - aspect_ratio)))
            .sqrt();

        let mut rectangle = Rect::new(lower, upper);

        // Choose random point to scale from
        let point: (f64, f64) = (
            rng.random_range(rectangle.min().x..rectangle.max().x),
            rng.random_range(rectangle.min().y..rectangle.max().y),
        );

        rectangle.scale_around_point_mut(
            scalar * aspect_ratio,
            scalar * (1. - aspect_ratio),
            point,
        );

        aabb = AABB::from_corners(rectangle.min(), rectangle.max());
    }

    Some(aabb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::Contains;

    #[test]
    fn anonymity_percentile_succes() {
        let ks: Vec<f64> = vec![5.7, 5.4, 1.0, 2.0, 3.0];
        let conf = AnonymityConf {
            min_k: 3,
            min_k_percentile: 0.5,
            min_area_size: 2.0,
        };

        let result = evaluate_route_anonymity(&conf, ks.iter()).unwrap();

        assert_eq!(result, true)
    }

    #[test]
    fn anonymity_percentile_fail() {
        let ks: Vec<f64> = vec![5.7, 5.4, 1.0, 2.0, 3.0];
        let conf = AnonymityConf {
            min_k: 4,
            min_k_percentile: 0.5,
            min_area_size: 2.0,
        };

        let result = evaluate_route_anonymity(&conf, ks.iter()).unwrap();

        assert_eq!(result, false)
    }

    #[test]
    fn aabb_creation_test() {
        let ks: LineString<f64> = vec![(9.991835, 57.012622), (9.990884, 57.013152)].into();

        let conf = AnonymityConf {
            min_k: 3,
            min_k_percentile: 0.5,
            min_area_size: 3500.0,
        };

        let result = calculate_aabb(&conf, &ks).unwrap();

        let rectangle = Rect::new(result.lower(), result.upper());

        let area = rectangle.geodesic_area_unsigned().round();

        assert!(area.ge(&conf.min_area_size));

        assert!(Rect::new(result.lower(), result.upper()).contains(&ks))
    }

    #[test]
    fn aabb_no_change() {
        let ks: LineString<f64> = vec![(9.68, 57.10), (10.17, 56.95)].into();

        let conf = AnonymityConf {
            min_k: 3,
            min_k_percentile: 0.5,
            min_area_size: 10.0_f64.powi(2),
        };

        let result = calculate_aabb(&conf, &ks).unwrap();

        assert_eq!(
            AABB::from_corners((9.68, 57.10).into(), (10.17, 56.95).into()),
            result
        );
    }
}
