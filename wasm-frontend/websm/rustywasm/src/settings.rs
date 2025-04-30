use eframe::egui::{Response, Ui};

use crate::state::StateHandle;

pub trait Settings {
    fn settings_menu(&mut self, state: StateHandle) -> Response;
}

impl Settings for Ui {
    fn settings_menu(&mut self, state: StateHandle) -> Response {
        self.menu_button("Settings", |ui| {
            ui.label("CAKE");
        })
        .response
    }
}
