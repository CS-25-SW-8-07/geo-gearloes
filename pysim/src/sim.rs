use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, RwLock, RwLockReadGuard},
    time::Duration,
};

use comms::Parquet;
use eframe::{
    App, NativeOptions,
    egui::{self, Id, Visuals},
};
use geo::{Coord, Point, Simplify};
use numpy::ndarray::parallel::prelude::IntoParallelRefMutIterator;
use proj::Proj;
use pyo3::{Bound, Python};
use rusty_roads::{RoadIndex, Roads};

use crate::{
    car::{Car, Server},
    config::SimConfig,
    widgets::map::Map,
};

use rayon::prelude::*;

#[derive(Debug, Clone)]
pub struct NPredict(pub usize);

#[derive(Debug, Clone)]
pub struct Uri(pub Arc<String>);

#[derive(Debug, Clone)]
pub struct StepCounter(pub usize);

#[derive(Debug, Clone)]
pub struct Delta(pub Duration);

#[derive(Debug, Clone)]
pub struct Time(pub Duration);

#[derive(Debug, Clone)]
pub struct BBox(pub Point, pub Point);
impl BBox {
    pub fn query_parameters(&self) -> String {
        let Self(Point(Coord { x: min_x, y: min_y }), Point(Coord { x: max_x, y: max_y })) = self;
        format!("lat1={min_x}&lon1={min_y}&lat2={max_x}&lon2={max_y}",)
    }
}

pub type Cars = SwapChain<Vec<crate::car::Car>>;

pub struct Sim(pub pyo3::Py<SimConfig>);

#[derive(Debug, Clone)]
pub struct Projection(Arc<Proj>);
unsafe impl Send for Projection {}
unsafe impl Sync for Projection {}

impl Deref for Projection {
    type Target = Proj;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl Sim {
    pub fn run(self) -> eframe::Result<()> {
        let Self(config) = self;
        let (
            roads,
            index,
            cars,
            step_delta,
            projection,
            bbox,
            server_url,
            predict,
            predict_n,
            steps,
        ) = Python::with_gil(|py| {
            let config = config.borrow(py);
            let projection = Projection(Arc::new(
                Proj::new_known_crs(
                    config.projection_from.as_str(),
                    config.projection_to.as_str(),
                    None,
                )
                .unwrap(),
            ));

            let cars = SwapChain::new(config.cars.clone());

            let bbox = BBox(
                projection.project(config.bbox_min, true).unwrap(),
                projection.project(config.bbox_max, true).unwrap(),
            );
            let (roads, index) = config.map.clone().map_or((None, None), |parquet| {
                let mut roads = Roads::from_parquet(parquet).unwrap();
                let epsilon = if roads.geom.len() > 1_000_000 {
                    0.3
                } else {
                    0.0
                };

                roads.geom = roads
                    .geom
                    .iter()
                    .map(|l| {
                        geo::LineString::from_iter(
                            l.points().map(|p| projection.project(p, true).unwrap()),
                        )
                        .simplify(&epsilon)
                    })
                    .collect();

                let index = Arc::new(RoadIndex::from_ids_and_roads(&roads.id, &roads.geom));
                let roads = Arc::new(roads);

                (Some(roads), Some(index))
            });

            let predict = config.predict.clone_ref(py);
            let predict_n = config.predict_n.clone();

            let step_delta = config.step_delta.clone();
            let server_url = config.server_url.clone();

            let steps = config.steps;

            (
                roads, index, cars, step_delta, projection, bbox, server_url, predict, predict_n,
                steps,
            )
        });

        let uri = Uri(Arc::new(server_url));

        Server {
            uri: uri.0.as_str(),
        }
        .reset();

        eframe::run_native(
            "Simultation",
            NativeOptions {
                ..Default::default()
            },
            Box::new(|cc| {
                let ctx = cc.egui_ctx.clone();
                let move_cars = cars.clone();
                std::thread::spawn(move || {
                    for _ in 0..steps {
                        let StepCounter(counter) =
                            ctx.data(|d| d.get_temp::<StepCounter>(Id::NULL)).unwrap();
                        let Time(time) = ctx.data(|d| d.get_temp::<Time>(Id::NULL)).unwrap();
                        let Delta(delta) = ctx.data(|d| d.get_temp::<Delta>(Id::NULL)).unwrap();
                        ctx.data_mut(|w| w.insert_temp(Id::NULL, StepCounter(counter + 1)));
                        ctx.data_mut(|w| w.insert_temp(Id::NULL, Time(time + delta)));
                        let ctx = ctx.clone();
                        let (p1, p2, p3, p4, p5, p6) = ctx.data(|r| {
                            (
                                r.get_temp(Id::NULL).unwrap(),
                                r.get_temp(Id::NULL).unwrap(),
                                r.get_temp(Id::NULL).unwrap(),
                                r.get_temp(Id::NULL).unwrap(),
                                r.get_temp(Id::NULL).unwrap(),
                                r.get_temp(Id::NULL).unwrap(),
                            )
                        });
                        move_cars.work(move |cars| {
                            cars.par_iter_mut().enumerate().for_each(|(idx, car)| {
                                dbg!("STEP START");
                                car.step(idx, &p1, &p2, &p3, &p4, &p5, &p6);
                                dbg!("STEP END");
                            });
                        });
                        ctx.request_repaint();
                    }
                });
                cc.egui_ctx.data_mut(|data| {
                    data.insert_temp(Id::NULL, roads);
                    data.insert_temp(Id::NULL, index);
                    data.insert_temp(Id::NULL, cars);
                    data.insert_temp(Id::NULL, Delta(step_delta));
                    data.insert_temp(Id::NULL, Time(Duration::ZERO));
                    data.insert_temp(Id::NULL, StepCounter(0));
                    data.insert_temp(Id::NULL, projection);
                    data.insert_temp(Id::NULL, bbox);
                    data.insert_temp(Id::NULL, uri);
                    data.insert_temp(Id::NULL, Arc::new(predict));
                    data.insert_temp(Id::NULL, NPredict(predict_n));
                });

                // Removing the bind to config allowing it to be garbage coollected
                drop(config);
                Ok(Box::new(SimApp))
            }),
        )
    }
}

struct SimApp;

#[derive(Clone)]
pub struct SwapChain<T> {
    pub active: Arc<RwLock<T>>,
    pub work: Arc<RwLock<T>>,
}

impl<T: Clone> SwapChain<T> {
    pub fn new(item: T) -> Self {
        Self {
            active: Arc::new(RwLock::new(item.clone())),
            work: Arc::new(RwLock::new(item)),
        }
    }
}
impl<T: Send + Sync + Clone + 'static> SwapChain<T> {
    pub fn get(&self) -> RwLockReadGuard<T> {
        self.active.read().unwrap()
    }
    pub fn work(&self, f: impl Fn(&mut T) + Send + 'static) {
        let work = self.work.clone();
        let active = self.active.clone();

        dbg!("WORKING !!");
        let mut w = work.write().unwrap();
        f(w.deref_mut());
        let mut a = active.write().unwrap();
        *a.deref_mut() = w.deref().clone();
        dbg!("STOPED WORKING !!");
    }
}

impl App for SimApp {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        ctx.set_visuals(Visuals::dark());
        egui::TopBottomPanel::new(egui::panel::TopBottomSide::Top, Id::NULL).show(ctx, |ui| {
            let StepCounter(counter) = ui.data(|d| d.get_temp::<StepCounter>(Id::NULL)).unwrap();
            let Time(time) = ui.data(|d| d.get_temp::<Time>(Id::NULL)).unwrap();
            let Delta(delta) = ui.data(|d| d.get_temp::<Delta>(Id::NULL)).unwrap();
            ui.horizontal(|ui| {
                if ui.button("Step").clicked() {
                    let cars = ui.data(|w| w.get_temp::<Cars>(Id::NULL));
                    if let Some(cars) = cars {
                        let ctx = ui.ctx().clone();
                    }
                }
                ui.label(format!("Step: {counter}"));
                ui.separator();
                ui.label(format!("Time: {time:?}"));
                ui.separator();
                ui.label(format!("Delta: {delta:?}"));
            })
        });
        ctx.set_visuals(Visuals::light());
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(Map);
        });
    }
}
