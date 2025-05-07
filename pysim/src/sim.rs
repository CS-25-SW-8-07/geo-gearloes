use std::{ops::Deref, sync::Arc, time::Duration};

use comms::Parquet;
use eframe::{
    egui::{self, Id, Style, Visuals},
    egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew},
    App, NativeOptions,
};
use geo::{Point, Simplify};
use proj::Proj;
use pyo3::Bound;
use rusty_roads::{RoadIndex, Roads};

use crate::{
    config::{SimConfig, Trajectory},
    widgets::map::Map,
};

#[derive(Debug, Clone)]
pub struct StepCounter(pub usize);

#[derive(Debug, Clone)]
pub struct Delta(pub Duration);

#[derive(Debug, Clone)]
pub struct Time(pub Duration);

#[derive(Debug, Clone)]
pub struct BBox(pub Point, pub Point);

pub struct Car {
    pub trajectory: Arc<Trajectory>,
}

impl From<Arc<Trajectory>> for Car {
    fn from(value: Arc<Trajectory>) -> Self {
        Self { trajectory: value }
    }
}

pub type Cars = Arc<Vec<Car>>;

pub struct Sim<'a>(pub Bound<'a, SimConfig>);

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

impl Sim<'_> {
    pub fn run(self) -> eframe::Result<()> {
        let config = &*self.0.borrow();
        let projection = Projection(Arc::new(
            Proj::new_known_crs(
                config.projection_from.as_str(),
                config.projection_to.as_str(),
                None,
            )
            .unwrap(),
        ));

        let bbox = BBox(
            projection.project(config.bbox_min, true).unwrap(),
            projection.project(config.bbox_max, true).unwrap(),
        );
        let mut roads = Roads::from_parquet(config.map.clone()).unwrap();

        let epsilon = if roads.geom.len() > 1_000_000 {
            0.3
        } else {
            0.0
        };

        roads.geom = roads
            .geom
            .iter()
            .map(|l| {
                geo::LineString::from_iter(l.points().map(|p| projection.project(p, true).unwrap()))
                    .simplify(&epsilon)
            })
            .collect();

        let roads = Arc::new(roads);

        let index = Arc::new(RoadIndex::from_ids_and_roads(&roads.id, &roads.geom));
        let cars: Cars = Arc::new(
            config
                .trajectories
                .iter()
                .cloned()
                .map(Car::from)
                .collect::<Vec<_>>(),
        );

        eframe::run_native(
            "Simultation",
            NativeOptions {
                ..Default::default()
            },
            Box::new(|cc| {
                cc.egui_ctx.data_mut(|data| {
                    data.insert_temp(Id::NULL, roads);
                    data.insert_temp(Id::NULL, index);
                    data.insert_temp(Id::NULL, cars);
                    data.insert_temp(Id::NULL, Delta(config.step_delta.clone()));
                    data.insert_temp(Id::NULL, Time(Duration::ZERO));
                    data.insert_temp(Id::NULL, StepCounter(0));
                    data.insert_temp(Id::NULL, projection);
                    data.insert_temp(Id::NULL, bbox);
                });

                // Removing the bind to config allowing it to be garbage coollected
                drop(self);
                Ok(Box::new(SimApp))
            }),
        )
    }
}

struct SimApp;

impl App for SimApp {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        ctx.set_visuals(Visuals::dark());
        egui::TopBottomPanel::new(egui::panel::TopBottomSide::Top, Id::NULL).show(ctx, |ui| {
            let StepCounter(counter) = ui.data(|d| d.get_temp::<StepCounter>(Id::NULL)).unwrap();
            let Time(time) = ui.data(|d| d.get_temp::<Time>(Id::NULL)).unwrap();
            let Delta(delta) = ui.data(|d| d.get_temp::<Delta>(Id::NULL)).unwrap();
            ui.horizontal(|ui| {
                if ui.button("Step").clicked() {
                    ui.data_mut(|w| w.insert_temp(Id::NULL, StepCounter(counter + 1)));
                    ui.data_mut(|w| w.insert_temp(Id::NULL, Time(time + delta)));
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
