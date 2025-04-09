use crate::emulator::EmulatorState;
use crate::ui::{components, UIContext, UIState};
use crate::vulkan_renderer::EmulatorRenderer;
use egui::Context;
use std::sync::{Arc, Mutex};

pub(crate) fn render(
    egui_context: &Context,
    ui_state: &mut UIState,
    emu_state: &EmulatorState,
    ui_context: &mut UIContext,
    emulator_renderer: Arc<Mutex<dyn EmulatorRenderer>>,
) {
    egui::TopBottomPanel::top("menu_bar").show(egui_context, |ui| {
        components::menu_bar::render(ui, egui_context, ui_state, &emu_state);
    });

    egui::CentralPanel::default().show(egui_context, |ui| {
        components::game_screen::render(ui, egui_context, ui_state, emu_state, emulator_renderer);
    });
}
