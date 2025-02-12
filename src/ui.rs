use egui::Context;

pub(crate) fn create_ui(egui_context: &Context) {
    egui::SidePanel::left("my_left_panel").show(&egui_context, |ui| {
        ui.label("Hello egui! Lorem ipsum dolor sit amet");
    });
}
