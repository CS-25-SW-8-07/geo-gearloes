use eframe::egui::{Id, Response, Widget};

use crate::StepCount;

pub struct StepCounter;

impl Widget for StepCounter {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        if let Some(StepCount(count)) = ui.data(|data| data.get_temp(Id::NULL)) {
            ui.label(format!("Stepcounter: {count}"))
        } else {
            ui.label("Stepcounter: None")
        }
    }
}
