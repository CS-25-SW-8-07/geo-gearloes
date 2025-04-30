use eframe::{
    egui::{self, Color32, Pos2, Sense, Shape, Stroke, Vec2},
    epaint::{CircleShape, PathShape, PathStroke},
};
use geo_types::{Coord, coord};
use map::{Cartograph, CompasRose};
use settings::Settings;
use state::StateHandle;
use wasm_bindgen::prelude::*;

pub mod map;
pub mod settings;
pub mod state;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
struct WebHandle {
    runner: eframe::WebRunner,
    state: state::StateHandle,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WebHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(state: state::StateHandle) -> Self {
        eframe::WebLogger::init(log::LevelFilter::Debug).ok();

        Self {
            runner: eframe::WebRunner::new(),
            state,
        }
    }

    #[wasm_bindgen]
    pub async fn start(
        &self,
        canvas: web_sys::HtmlCanvasElement,
    ) -> Result<(), wasm_bindgen::JsValue> {
        let state = self.state.clone();
        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| Ok(Box::new(App::new(cc, state)))),
            )
            .await
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct App {
    state: state::StateHandle,
}

impl App {
    #[allow(dead_code)]
    fn new(ctx: &eframe::CreationContext<'_>, state: StateHandle) -> Self {
        log::debug!("Starting APP");
        let mut context = state.context.lock().unwrap();
        *context = ctx.egui_ctx.clone();
        drop(context);
        Self { state }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal_top(|ui| {
                    ui.settings_menu(self.state.clone());
                    ui.shrink_height_to_current();
                    ui.add_space(ui.available_width() - ui.available_height());
                    ui.compas_rose(self.state.clone());
                });
                ui.cartograph(self.state.clone());
            });
        });
    }
}

#[allow(unused)]
pub(crate) trait PrintErr {
    fn print_err(self) -> Self;
}

impl<T, E: std::fmt::Debug> PrintErr for Result<T, E> {
    #[inline]
    fn print_err(self) -> Self {
        if let Err(e) = &self {
            log::error!("{e:?}");
        }

        self
    }
}

#[allow(unused)]
pub(crate) trait PrintDebug: std::fmt::Debug + Sized {
    #[cfg(debug_assertions)]
    #[inline]
    fn print_debug(self, header: &'static str) -> Self {
        log::debug!("{header}: {self:?}");
        self
    }

    #[cfg(not(debug_assertions))]
    #[inline]
    fn print_debug(self, _: &'static str) -> Self {
        self
    }
}

impl<T: std::fmt::Debug + Sized> PrintDebug for T {}
