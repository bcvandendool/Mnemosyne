use crate::emulator::EmulatorState;
use crate::ui::{components, UIContext};
use crate::ui::{BottomPanels, UIState};
use crate::vulkan_renderer::EmulatorRenderer;
use egui::{Context, Stroke};
use std::sync::{Arc, Mutex};

pub(crate) fn render(
    egui_context: &Context,
    ui_state: &mut UIState,
    emu_state: &EmulatorState,
    ui_context: &mut UIContext,
    emulator_renderer: Arc<Mutex<dyn EmulatorRenderer>>,
) {
    egui::TopBottomPanel::top("menu_bar").show(egui_context, |ui| {
        components::menu_bar::render(ui, egui_context, ui_state, emu_state);
    });

    egui::SidePanel::left("left_panel")
        .resizable(false)
        .show(egui_context, |ui| {
            components::memory_viewer::render(ui, egui_context, ui_state, emu_state);
        });

    egui::SidePanel::right("right_panel")
        .exact_width(300.5)
        .resizable(false)
        .show(egui_context, |ui| {
            components::register_viewer::render(ui, egui_context, ui_state, emu_state);

            ui.separator();

            components::disassembly::render(ui, ui_context, egui_context, ui_state, emu_state);

            ui.separator();

            components::breakpoints::render(ui, egui_context, ui_state, emu_state);
        });

    egui::TopBottomPanel::bottom("bottom_panel")
        .min_height(300.0)
        .show(egui_context, |ui| {
            puffin::profile_scope!("UI - Bottom panel");

            ui.add_space(5.0);
            ui.horizontal(|ui| {
                let mut logger_button = ui.button("Logger");
                if ui_state.bottom_panel == BottomPanels::Logger {
                    logger_button = logger_button.highlight();
                }
                if logger_button.clicked() {
                    ui_state.bottom_panel = BottomPanels::Logger;
                    puffin::set_scopes_on(false);
                }

                let mut profiler_button = ui.button("Profiler");
                if ui_state.bottom_panel == BottomPanels::Profiler {
                    profiler_button = profiler_button.highlight();
                }
                if profiler_button.clicked() {
                    ui_state.bottom_panel = BottomPanels::Profiler;
                    puffin::set_scopes_on(true);
                }
            });

            egui::Frame::default()
                .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                .inner_margin(5.0)
                .show(ui, |ui| {
                    if ui_state.bottom_panel == BottomPanels::Logger {
                        egui_logger::logger_ui().show_target(false).show(ui);
                    } else if ui_state.bottom_panel == BottomPanels::Profiler {
                        puffin_egui::profiler_ui(ui);
                    }
                });
        });

    egui::CentralPanel::default().show(egui_context, |ui| {
        components::game_screen::render(ui, egui_context, ui_state, emu_state, emulator_renderer);
    });
}
