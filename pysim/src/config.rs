use std::{ops::Deref, sync::Arc, time::Duration};

use bytes::Bytes;
use chrono::NaiveDateTime;
use geo::{CoordNum, Point};
use pyo3::{prelude::*, types::PyList};
use pyo3_bytes::PyBytes;

#[pyclass]
#[pyo3(name = "Point")]
pub struct PyPoint {
    #[pyo3(get, set)]
    lon: f64,
    #[pyo3(get, set)]
    lat: f64,
}

#[pymethods]
impl PyPoint {
    #[new]
    pub fn new(lon: f64, lat: f64) -> Self {
        Self { lon, lat }
    }
}

impl<T: CoordNum + From<f64>> From<&PyPoint> for Point<T> {
    fn from(value: &PyPoint) -> Self {
        Point::new(value.lon.into(), value.lat.into())
    }
}

#[derive(Default)]
#[pyclass]
#[pyo3(name = "Trajectory")]
pub struct PyTrajectory {
    points: Vec<Point>,
    timestamps: Vec<Duration>,
}

#[pymethods]
impl PyTrajectory {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_point(&mut self, p: Bound<'_, PyPoint>, d: Duration) {
        self.points.push(p.borrow().deref().into());
        self.timestamps.push(d);
    }
}

pub struct Trajectory {
    pub points: Vec<Point>,
    pub timestamps: Vec<Duration>,
}

impl From<&Bound<'_, PyTrajectory>> for Trajectory {
    fn from(value: &Bound<PyTrajectory>) -> Self {
        Self {
            points: value.borrow().points.clone(),
            timestamps: value.borrow().timestamps.clone(),
        }
    }
}

#[pyclass]
pub struct SimConfig {
    #[pyo3(set, get)]
    pub step_delta: Duration,
    #[pyo3(set, get)]
    pub projection_from: String,
    #[pyo3(set, get)]
    pub projection_to: String,
    pub bbox_min: Point,
    pub bbox_max: Point,
    pub map: Bytes,
    pub trajectories: Vec<Arc<Trajectory>>,
}

#[pymethods]
impl SimConfig {
    #[new]
    #[pyo3(signature = (
        bbox_p1, bbox_p2, map, trajectories, projection_from = "EPSG:4326".into(), 
        projection_to = "EPSG:4326".into(), step_delta = Duration::from_secs(60)
    ))]
    fn new<'py>(
        bbox_p1: Bound<'_, PyPoint>,
        bbox_p2: Bound<'_, PyPoint>,
        map: PyBytes,
        trajectories: Bound<'py, PyList>,
        projection_from: String,
        projection_to: String,
        step_delta: Duration,
    ) -> SimConfig {
        let trajectories = trajectories
            .iter()
            .map(|maybe_t| {
                let t = maybe_t.downcast_exact::<PyTrajectory>().unwrap();
                Arc::new(Trajectory::from(t))
            })
            .collect::<Vec<_>>();

        let p1: Point = (&*bbox_p1.borrow()).into();
        let p2: Point = (&*bbox_p2.borrow()).into();

        let bbox_max = Point::new(p1.x().max(p2.x()), p1.y().max(p2.y()));
        let bbox_min = Point::new(p1.x().min(p2.x()), p1.y().min(p2.y()));

        Self {
            bbox_max,
            bbox_min,
            map: map.into_inner(),
            trajectories,
            projection_from,
            projection_to,
            step_delta,
        }
    }
}

pub fn config_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SimConfig>()?;
    m.add_class::<PyTrajectory>()?;
    m.add_class::<PyPoint>()?;

    Ok(())
}
