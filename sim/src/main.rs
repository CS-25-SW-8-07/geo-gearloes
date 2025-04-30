//pub mod map;
pub mod widgets;
pub mod worker;

#[derive(Default, Debug, Clone, Copy)]
struct StepCount(usize);

use eframe::{
    NativeOptions,
    egui::{self, Button, Id, Slider, Visuals},
};
use widgets::{SpawnSimMenu, StepCounter, StepperButton};
use worker::{Worker, WorkerState, Workers};

fn main() {
    eframe::run_native(
        "GeoGearløsSim",
        NativeOptions {
            ..Default::default()
        },
        Box::new(|cc| {
            let r = ();
            cc.egui_ctx.data_mut(|data| {});
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
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_width_range(ui.available_width()..=ui.available_width());
            ui.set_height_range(ui.available_height()..=ui.available_height());
            let painter = ui.painter();
        });
    }
}
