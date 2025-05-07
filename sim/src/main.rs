//pub mod map;
pub mod args;
pub mod widgets;
pub mod worker;

// AALBORG
const NORTH_EAST: Coord = Coord {
    y: 57.133836,
    x: 10.147676,
};

const SOUTH_WEST: Coord = Coord {
    y: 56.927713,
    x: 9.776412,
};
/*: DENMARK does crash :(
const NORTH_EAST: Coord = Coord {
    y: 58.039624,
    x: 17.413168,
};

const SOUTH_WEST: Coord = Coord {
    y: 54.494441,
    x: 7.536414,
};
*/

#[derive(Default, Debug, Clone, Copy)]
struct StepCount(usize);

use comms::Parquet;
use eframe::{
    NativeOptions,
    egui::{self, Id, Visuals},
};
use geo::Coord;
use widgets::{Map, SpawnSimMenu, StepCounter, StepperButton};

fn main() {
    eframe::run_native(
        "GeoGearløsSim",
        NativeOptions {
            ..Default::default()
        },
        Box::new(|cc| {
            let reader = ureq::get(format!(
                "http://localhost:8080/get_roads_in_bbox.parquet?lat1={}&lon1={}&lat2={}&lon2={}",
                NORTH_EAST.y, NORTH_EAST.x, SOUTH_WEST.y, SOUTH_WEST.x
            ))
            .call()
            .unwrap()
            .body_mut()
            .read_to_vec()
            .unwrap();

            let bytes = comms::Bytes::from_owner(reader);

            let roads = rusty_roads::Roads::from_parquet(bytes).unwrap();
            let index = rusty_roads::RoadIndex::from_ids_and_roads(&roads.id, &roads.geom);

            cc.egui_ctx.data_mut(|data| {
                data.insert_temp(Id::NULL, roads);
                data.insert_temp(Id::NULL, index);
            });

            Ok(Box::new(App))
        }),
    )
    .unwrap();
}

struct App;

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        ctx.style_mut(|s| s.visuals = Visuals::dark());
        egui::TopBottomPanel::new(egui::panel::TopBottomSide::Top, "TopPannel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(SpawnSimMenu);
                ui.separator();
                ui.add(StepperButton);
                ui.separator();
                ui.add(StepCounter);
            });
        });

        ctx.style_mut(|s| s.visuals = Visuals::light());
        egui::CentralPanel::default().show(ctx, |ui| ui.add(Map));
    }
}
