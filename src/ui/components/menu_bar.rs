use crate::emulator::EmulatorControlMessage;
use crate::emulator::EmulatorState;
use crate::ui::{UIState, Views};
use egui::text::LayoutJob;
use egui::{
    menu, Align, Context, FontFamily, FontId, Label, Layout, RichText, TextFormat, Ui, Vec2,
};
use rfd::FileDialog;

pub(crate) fn render(
    ui: &mut Ui,
    egui_context: &Context,
    ui_state: &mut UIState,
    emu_state: &EmulatorState,
) {
    puffin::profile_scope!("UI - Menu bar");
    menu::bar(ui, |ui| {
        ui.columns(3, |columns| {
            columns[0].with_layout(Layout::left_to_right(Align::LEFT), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Open").clicked() {
                            let path = FileDialog::new()
                                .add_filter("gb", &["gb", "gbc"])
                                .pick_file();

                            if let Some(path) = path {
                                ui_state
                                    .tx_ui
                                    .send(EmulatorControlMessage::Load(
                                        path.to_str()
                                            .expect("Failed to parse path to string")
                                            .to_string(),
                                    ))
                                    .expect("Failed to send control message to emulator thread");
                                ui_state
                                    .tx_ui
                                    .send(EmulatorControlMessage::Start)
                                    .expect("Failed to send control message to emulator thread");
                                ui_state.current_view = Views::Fullscreen;
                            }
                            ui.close_menu();
                        }

                        ui.separator();

                        if ui.button("Open user folder").clicked() {}

                        ui.separator();

                        if ui.button("Exit").clicked() {}
                    });

                    ui.menu_button("Emulation", |ui| {
                        // Save state load/save/slot
                        // Reset core
                        // Set hardware revision, probably only dmg/cgb
                        // Local 2 or 4 player coop
                    });
                    ui.menu_button("Options", |ui| {
                        // Graphics settings
                        // Game overview settings
                        // Input settings
                        // Audio settings
                    });

                    ui.menu_button("Input", |ui| {
                        // Quick player / controller mapping
                        // Link to full input settings
                    });

                    ui.menu_button("View", |ui| {
                        if ui.button("Fullscreen").clicked() {
                            ui_state.current_view = Views::Fullscreen;
                            ui.close_menu();
                        }

                        if ui.button("Debugger").clicked() {
                            ui_state.current_view = Views::Debugger;
                            ui.close_menu();
                        }

                        if ui.button("Game list").clicked() {
                            ui_state.current_view = Views::GameList;
                            ui.close_menu();
                        }

                        // Multi screen positioning
                    });

                    ui.menu_button("Multiplayer", |ui| {
                        // Become host
                        // Kick players
                        // Set (optional) password
                        // Join host
                    });
                });
            });

            columns[1].with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.columns(2, |columns| {
                    columns[0].allocate_ui_with_layout(
                        Vec2::ZERO,
                        Layout::right_to_left(Align::Center),
                        |ui| {
                            if ui
                                .button(
                                    RichText::new(egui_material_icons::icons::ICON_STEP_OVER)
                                        .size(20.0),
                                )
                                .clicked()
                            {
                                ui_state
                                    .tx_ui
                                    .send(EmulatorControlMessage::StepOver)
                                    .expect("Failed to send control message to emulator");
                            }
                            if ui
                                .button(
                                    RichText::new(egui_material_icons::icons::ICON_PAUSE)
                                        .size(20.0),
                                )
                                .clicked()
                            {
                                ui_state
                                    .tx_ui
                                    .send(EmulatorControlMessage::Pause)
                                    .expect("Failed to send control message to emulator");
                            }
                            if ui
                                .button(
                                    RichText::new(egui_material_icons::icons::ICON_PLAY_ARROW)
                                        .size(20.0),
                                )
                                .clicked()
                            {
                                ui_state
                                    .tx_ui
                                    .send(EmulatorControlMessage::Start)
                                    .expect("Failed to send control message to emulator");
                            }
                            if ui
                                .button(
                                    RichText::new(egui_material_icons::icons::ICON_FAST_REWIND)
                                        .size(20.0),
                                )
                                .clicked()
                            {
                                ui_state
                                    .tx_ui
                                    .send(EmulatorControlMessage::FastRewind(2))
                                    .expect("Failed to send control message to emulator");
                            }
                        },
                    );

                    columns[1].allocate_ui_with_layout(
                        Vec2::ZERO,
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            if ui
                                .button(
                                    RichText::new(egui_material_icons::icons::ICON_STEP_INTO)
                                        .size(20.0),
                                )
                                .clicked()
                            {
                                ui_state
                                    .tx_ui
                                    .send(EmulatorControlMessage::StepInto)
                                    .expect("Failed to send control message to emulator");
                            }
                            if ui
                                .button(
                                    RichText::new(egui_material_icons::icons::ICON_STEP_OUT)
                                        .size(20.0),
                                )
                                .clicked()
                            {
                                ui_state
                                    .tx_ui
                                    .send(EmulatorControlMessage::StepOut)
                                    .expect("Failed to send control message to emulator");
                            }
                            if ui
                                .button(
                                    RichText::new(egui_material_icons::icons::ICON_STOP).size(20.0),
                                )
                                .clicked()
                            {
                                ui_state
                                    .tx_ui
                                    .send(EmulatorControlMessage::Stop)
                                    .expect("Failed to send control message to emulator");
                                ui_state.current_view = Views::GameList;
                            }
                            if ui
                                .button(
                                    RichText::new(egui_material_icons::icons::ICON_FAST_FORWARD)
                                        .size(20.0),
                                )
                                .clicked()
                            {
                                ui_state
                                    .tx_ui
                                    .send(EmulatorControlMessage::FastForward(2))
                                    .expect("Failed to send control message to emulator");
                            }
                        },
                    );
                });
            });

            columns[2].with_layout(Layout::right_to_left(Align::LEFT), |ui| {
                ui.add(egui::Slider::new(&mut ui_state.volume, 0.0..=100.0).text("Volume"));
            });
        })
    });
}
