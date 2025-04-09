use crate::egui_renderer::CallbackFn;
use crate::emulator::EmulatorState;
use crate::ui::UIState;
use crate::vulkan_renderer::EmulatorRenderer;
use egui::{vec2, Context, PaintCallback, Rgba, Sense, Ui};
use std::sync::{Arc, Mutex};

pub(crate) fn render(
    ui: &mut Ui,
    egui_context: &Context,
    ui_state: &mut UIState,
    emu_state: &EmulatorState,
    emulator_renderer: Arc<Mutex<dyn EmulatorRenderer>>,
) {
    egui::Frame::canvas(ui.style())
        .fill(Rgba::BLACK.into())
        .show(ui, |ui| {
            puffin::profile_scope!("UI - Emulator renderer");
            // Allocate all the space in the frame for the image
            let (rect, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), ui.available_height()),
                Sense::empty(),
            );

            // Render the scene in the allocated space
            let paint_callback = PaintCallback {
                rect,
                callback: Arc::new(CallbackFn::new(move |info, context| {
                    let emu_renderer = emulator_renderer
                        .lock()
                        .expect("Failed to lock emulator renderer");
                    emu_renderer.render(info, context);
                })),
            };

            ui.painter().add(paint_callback);
        });
}
