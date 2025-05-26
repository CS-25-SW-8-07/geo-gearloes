use std::{ops::Deref, sync::Arc, time::Duration};

use bytes::Bytes;
use eframe::egui::Color32;
use geo::{CoordNum, Point};
use pyo3::{
    prelude::*,
    types::{PyFunction, PyList},
};
use pyo3_bytes::PyBytes;
use rusty_roads::AnonymityConf;

use crate::car::Car;

#[pyclass]
#[pyo3(name = "AnonymityConf")]
#[derive(Debug, Clone, Copy)]
pub struct PyAnonymityConf {
    #[pyo3(get, set)]
    min_k: u32,
    #[pyo3(get, set)]
    min_k_percentile: f64,
    #[pyo3(get, set)]
    min_area_size: f64,
}

#[pymethods]
impl PyAnonymityConf {
    #[new]
    pub fn new(min_k: u32, min_k_percentile: f64, min_area_size: f64) -> Self {
        Self {
            min_k,
            min_k_percentile,
            min_area_size,
        }
    }
}

impl From<PyAnonymityConf> for AnonymityConf {
    fn from(
        PyAnonymityConf {
            min_k,
            min_k_percentile,
            min_area_size,
        }: PyAnonymityConf,
    ) -> Self {
        Self {
            min_k,
            min_k_percentile,
            min_area_size,
        }
    }
}

#[pyclass]
#[pyo3(name = "Car")]
pub struct PyCar {
    pub trajectory: Trajectory,
    #[pyo3(get, set)]
    pub color: String,
    #[pyo3(get, set)]
    pub record_delay: Duration,
    #[pyo3(get, set)]
    pub send_delay: Duration,
    #[pyo3(get, set)]
    pub drive_delay: Duration,
    pub annon_conf: AnonymityConf,
}

#[pymethods]
impl PyCar {
    #[new]
    #[pyo3(signature = (
        trajectory, color = "#0033ee".into(), record_delay = Duration::from_secs(60), 
        send_delay = Duration::from_secs(120), drive_delay = Duration::from_secs(0), 
        annon_conf = PyAnonymityConf {
            min_k: 100,
            min_area_size: 100000000.0,
            min_k_percentile: 95.0
        }
    ))]
    pub fn new(
        trajectory: Bound<'_, PyTrajectory>,
        color: String,
        record_delay: Duration,
        send_delay: Duration,
        drive_delay: Duration,
        annon_conf: PyAnonymityConf,
    ) -> Self {
        Self {
            trajectory: Trajectory::from(&trajectory),
            color,
            record_delay,
            send_delay,
            drive_delay,
            annon_conf: annon_conf.into(),
        }
    }
}

impl From<&Bound<'_, PyCar>> for Car {
    fn from(value: &Bound<PyCar>) -> Self {
        Self {
            drive: value.borrow().trajectory.clone(),
            color: Color32::from_hex(&value.borrow().color).unwrap(),
            record_delay: value.borrow().record_delay.clone(),
            send_delay: value.borrow().send_delay.clone(),
            drive_delay: value.borrow().drive_delay.clone(),
            record: Trajectory::default(),
            predicted: Trajectory::default(),
            anonymity_config: value.borrow().annon_conf.clone(),
        }
    }
}

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

#[derive(Debug, Clone, Default)]
pub struct Trajectory {
    pub points: Vec<Point>,
    pub timestamps: Vec<Duration>,
}

impl Trajectory {
    pub fn push(&mut self, point: Point, timestamp: Duration) {
        self.points.push(point);
        self.timestamps.push(timestamp);
    }
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
    #[pyo3(set, get)]
    pub server_url: String,
    pub predict: Py<pyo3::types::PyFunction>,
    pub predict_n: usize,
    pub bbox_min: Point,
    pub bbox_max: Point,
    pub map: Option<Bytes>,
    pub cars: Vec<crate::car::Car>,
    pub steps: usize,
}

#[pymethods]
impl SimConfig {
    #[new]
    #[pyo3(signature = (
        bbox_p1, bbox_p2, map, cars, predict, predict_n, server_url = "localhost:8080".into(),
        projection_from = "EPSG:4326".into(), projection_to = "EPSG:4326".into(),
        step_delta = Duration::from_secs(60), steps = 1000,
    ))]
    fn new(
        bbox_p1: Bound<'_, PyPoint>,
        bbox_p2: Bound<'_, PyPoint>,
        map: Option<PyBytes>,
        cars: Bound<'_, PyList>,
        predict: Bound<'_, PyFunction>,
        predict_n: usize,
        server_url: String,
        projection_from: String,
        projection_to: String,
        step_delta: Duration,
        steps: usize
    ) -> SimConfig {
        let cars = cars
            .iter()
            .map(|maybe_t| maybe_t.downcast_exact::<PyCar>().unwrap().into())
            .collect::<Vec<_>>();

        let p1: Point = (&*bbox_p1.borrow()).into();
        let p2: Point = (&*bbox_p2.borrow()).into();

        let bbox_max = Point::new(p1.x().max(p2.x()), p1.y().max(p2.y()));
        let bbox_min = Point::new(p1.x().min(p2.x()), p1.y().min(p2.y()));

        let predict = predict.unbind();

        Self {
            bbox_max,
            bbox_min,
            map: map.map(PyBytes::into_inner),
            cars,
            predict,
            predict_n,
            projection_from,
            projection_to,
            step_delta,
            server_url,
            steps
        }
    }
}

pub fn config_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SimConfig>()?;
    m.add_class::<PyTrajectory>()?;
    m.add_class::<PyPoint>()?;
    m.add_class::<PyCar>()?;

    Ok(())
}
