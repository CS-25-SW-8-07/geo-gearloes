use std::{collections::HashSet, sync::Arc};

use comms::Parquet;
use eframe::egui::Color32;
use geo::{Line, LineString, Point, Translate};
use numpy::{PyArrayDyn, PyArrayMethods, ToPyArray, ndarray::Array};
use pyo3::{Py, Python, types::PyFunction};
use rstar::AABB;
use rusty_roads::{Anonymities, AnonymityConf, RoadIndex, Trajectories};
use std::time::Duration;

use crate::{
    config::Trajectory,
    sim::{BBox, Delta, NPredict, Time, Uri},
};

const AI_LEN: usize = 11;

#[derive(Debug, Clone)]
pub struct Car {
    pub drive: Trajectory,
    pub record: Trajectory,
    pub predicted: Trajectory,
    pub color: Color32,
    pub record_delay: Duration,
    pub send_delay: Duration,
    pub drive_delay: Duration,
    pub anonymity_config: AnonymityConf,
}

impl Car {
    pub fn should_draw(&self, time: Duration) -> bool {
        (time > self.drive_delay)
            && ((time + self.drive_delay) <= *self.drive.timestamps.last().unwrap())
    }

    pub fn step(
        &mut self,
        Uri(uri): &Uri,
        ai: &Arc<Py<PyFunction>>,
        NPredict(n): &NPredict,
        index: &Option<Arc<RoadIndex>>,
        Time(time): &Time,
        Delta(delta): &Delta,
    ) {
        if Self::is_time(self.drive_delay, self.record_delay, *time, *delta) {
            self.record(*time)
        }

        if Self::is_time(self.drive_delay, self.send_delay, *time, *delta)
            && self.record.timestamps.len() > AI_LEN
        {
            self.predict(ai.clone(), *n);
            self.send(
                uri.as_str(),
                index.as_ref().map_or(&Default::default(), AsRef::as_ref),
            )
        }
    }

    fn is_time(drive_delay: Duration, delay: Duration, time: Duration, delta: Duration) -> bool {
        let Some(projected_time) = (time > drive_delay).then_some(time - drive_delay) else {
            return false;
        };

        projected_time.as_secs_f64() / delay.as_secs_f64() < delta.as_secs_f64()
    }

    fn record(&mut self, time: Duration) {
        let d_t = &self.drive.timestamps;
        let d_p = &self.drive.points;

        let t = time - self.drive_delay;
        match d_t.binary_search(&t) {
            Ok(index) => self.record.push(d_p[index], t),
            Err(index) if index < d_t.len() => {
                let tm =
                    (d_t[index] - t).as_secs_f64() / (d_t[index] - d_t[index - 1]).as_secs_f64();

                let p = d_p[index].translate(
                    (d_p[index].x() - d_p[index - 1].x()) * tm,
                    (d_p[index].y() - d_p[index - 1].y()) * tm,
                );

                self.record.push(p, t);
            }
            _ => {}
        }
    }

    fn predict(&mut self, ai: Arc<Py<PyFunction>>, n_predict: usize) {
        self.predicted = ML { ai }.gen_trajectory(self.record.clone(), n_predict);
    }

    fn send(&self, uri: &str, index: &RoadIndex) {
        let t = &self.predicted;
        let bbox = Traj::gen_bbox(&t, &self.anonymity_config);
        let ks = Server { uri }.request_k(&bbox);
        let overlap = Traj::map_match_roads_id(&t, &index).unwrap();
        let id_set = ks.road_id.iter().cloned().collect();
        let kt: HashSet<_> = overlap.union(&id_set).collect();

        if let Ok(_) = location_obfuscation::anonymity::evaluate_route_anonymity(
            &self.anonymity_config,
            Iterator::zip(ks.road_id.iter(), ks.current_k.iter())
                .filter_map(|(i, k)| kt.contains(&i).then_some(k)),
        ) {
            Server { uri }.upload_trajectory(&t);
        } else {
            let roads = index.box_query(&AABB::from_corners(
                unsafe { std::mem::transmute::<Point, [f64; 2]>(bbox.0) }.into(),
                unsafe { std::mem::transmute::<Point, [f64; 2]>(bbox.1) }.into(),
            ));
            let road_count = roads.count();

            let percentile_coverage = Traj::calc_percentile_coverage(overlap.len(), road_count);
            Server { uri }.send_anonymus(&bbox, percentile_coverage);
        }
    }
}

pub struct ML {
    ai: Arc<Py<PyFunction>>,
}
impl ML {
    pub fn gen_trajectory(&self, mut trajectory: Trajectory, n: usize) -> Trajectory {
        if n == 0 {
            return trajectory;
        }

        let data: Vec<f32> = Iterator::zip(
            trajectory.points.windows(2),
            trajectory.timestamps.windows(2),
        )
        .map(|(p, t)| {
            let t1 = t[0];
            let t2 = t[1];
            let __ = p[0];
            let p2 = p[1];

            [(t2 - t1).as_secs_f32(), p2.y() as f32, p2.x() as f32]
        })
        .rev()
        .take(AI_LEN)
        .rev()
        .flatten()
        .collect();

        debug_assert_eq!(data.len(), AI_LEN * 3);

        let data = Array::from_shape_vec((1, AI_LEN, 3), data).unwrap();

        dbg!("ACCURING GIL");
        let data = Python::with_gil(|py| {
            dbg!("ACCURED GIL");
            let data = self
                .ai
                .call1(
                    py,
                    pyo3::types::PyTuple::new(py, vec![data.to_pyarray(py)].iter()).unwrap(),
                )
                .unwrap();

            let data = data
                .downcast_bound::<PyArrayDyn<f32>>(py)
                .unwrap()
                .to_vec()
                .unwrap();
            data
        });

        let time = trajectory.timestamps.last().unwrap().clone() + Duration::from_secs_f32(data[0]);
        let point = Point::new(data[2] as f64, data[1] as f64);

        trajectory.points.push(point);
        trajectory.timestamps.push(time);

        self.gen_trajectory(trajectory, n - 1)
    }
}

pub struct Traj;
impl Traj {
    pub fn gen_bbox(trajectory: &Trajectory, anon_conf: &AnonymityConf) -> BBox {
        let aabb = location_obfuscation::anonymity::calculate_aabb(
            anon_conf,
            &LineString::from_iter(trajectory.points.iter().cloned()),
        )
        .unwrap();
        BBox(aabb.lower().into(), aabb.upper().into())
    }

    pub fn map_match_roads_id(
        t: &Trajectory,
        index: &RoadIndex,
    ) -> Option<HashSet<rusty_roads::Id>> {
        rusty_roads::map_match::segment_road(
            t.points.windows(2).map(|p| Line::new(p[0], p[1])),
            &index,
        )
        .map(|r| r.into_iter().collect())
        .ok()
    }

    #[inline]
    pub fn calc_percentile_coverage(overlap_count: usize, road_count: usize) -> f64 {
        overlap_count as f64 / road_count as f64
    }
}

pub struct Server<'a> {
    uri: &'a str,
}
impl<'a> Server<'a> {
    pub fn request_k(&self, bbox: &BBox) -> Anonymities {
        let Self { uri } = self;
        let mut req = ureq::get(format!("{uri}/get_ks_in_bbox?{}", bbox.query_parameters()))
            .call()
            .unwrap();

        Anonymities::from_parquet(bytes::Bytes::from_owner(
            req.body_mut().read_to_vec().unwrap(),
        ))
        .unwrap()
    }

    pub fn upload_trajectory(&self, t: &Trajectory) {
        let Self { uri } = self;
        let ls = LineString::from_iter(t.points.clone().into_iter());
        let ts = Trajectories { geom: vec![ls] };
        ureq::post(format!("{uri}/add_trajectory"))
            .send(ts.to_parquet().unwrap().iter().as_slice())
            .unwrap();
    }

    pub fn send_anonymus(&self, bbox: &BBox, percentile_coverage: f64) {
        let Self { uri } = self;
        ureq::post(format!(
            "{uri}/add_unknown_visit?probability={percentile_coverage}&{}",
            bbox.query_parameters()
        ))
        .send("")
        .unwrap();
    }
}
