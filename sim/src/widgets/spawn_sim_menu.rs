use eframe::egui::{Button, Id, Slider, Widget};

use crate::worker::{Worker, Workers};

pub struct SpawnSimMenu;

impl Widget for SpawnSimMenu {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        ui.menu_button("Spawn simulation", |ui| {
            static mut N_WORKERS: usize = 10;
            static mut N_THREADS: usize = 2;

            ui.add(Slider::new(unsafe { &mut *&raw mut N_WORKERS }, 1..=100));
            ui.add(Slider::new(unsafe { &mut *&raw mut N_THREADS }, 1..=100));
            let btn = ui.add(Button::new("Spwan Workers"));
            if btn.clicked() {
                let workers = Workers::new(unsafe { N_WORKERS }, unsafe { N_THREADS }, |_, _| {
                    Worker::new()
                });

                ui.data_mut(|writer| writer.insert_temp(Id::NULL, workers));
            }
        })
        .response
    }
}
