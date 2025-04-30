use eframe::egui::{Button, Id, Widget};

use crate::{StepCount, worker::AWorkers};

pub struct StepperButton;

impl Widget for StepperButton {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let btn = ui.add(Button::new("Step"));
        if btn.clicked() {
            if let Some(wokers) = ui.data(|r| r.get_temp::<AWorkers>(Id::NULL)) {
                wokers.step_all();
            };

            ui.data_mut(|r| r.get_temp_mut_or_default::<StepCount>(Id::NULL).0 += 1);
        }

        btn
    }
}
