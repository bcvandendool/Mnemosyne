use crate::config::THREAD_LOCAL_CONFIG;
use crate::emulator::{EmulatorControlMessage, EmulatorState};
use crate::ui::{components, UIContext};
use crate::ui::{UIState, Views};
use crate::vulkan_renderer::EmulatorRenderer;
use egui::{Button, Color32, Context, CornerRadius, Label, Margin, Sense, Stroke, UiBuilder, Vec2};
use egui_extras::{Size, Strip};
use std::fs;
use std::sync::{Arc, Mutex};
use urlencoding::encode;
use winit::dpi::PhysicalSize;

pub(crate) fn render(
    egui_context: &Context,
    size: PhysicalSize<u32>,
    ui_state: &mut UIState,
    emu_state: &EmulatorState,
    ui_context: &mut UIContext,
    emulator_renderer: Arc<Mutex<dyn EmulatorRenderer>>,
) {
    egui::TopBottomPanel::top("menu_bar").show(egui_context, |ui| {
        components::menu_bar::render(ui, egui_context, ui_state, emu_state);
    });

    egui::SidePanel::left("game_details")
        .exact_width((size.width / 4) as f32)
        .resizable(false)
        .show(egui_context, |ui| {
            if !ui_state.selected_game.is_empty() {
                let mut game_name = ui_state.selected_game.clone();
                game_name.truncate(game_name.len() - 3);
                let cleaned_name = game_name.clone();
                game_name = encode(&game_name).to_string();
                ui.add(
                    egui::Image::new(format!("https://thumbnails.libretro.com/Nintendo%20-%20Game%20Boy/Named_Boxarts/{}.png", &game_name))
                        .maintain_aspect_ratio(true)
                        .fit_to_exact_size(Vec2 {
                            x: ui.available_width(),
                            y: ui.available_height(),
                        })
                        .sense(Sense::empty()),
                );
                ui.add(Label::new(cleaned_name));
                let response = ui.add_sized(Vec2 {x: ui.available_width(), y: 40.0}, Button::new("Play").frame(true));
                if response.clicked() {
                    let mut game_dir =
                        THREAD_LOCAL_CONFIG.with(|c| c.borrow_mut().load().ui_config.game_folder.clone());
                    game_dir += &ui_state.selected_game;
                    ui_state.tx_ui.send(EmulatorControlMessage::Load(game_dir)).expect("Failed to send control message to emulator thread");
                    ui_state.tx_ui.send(EmulatorControlMessage::Start).expect("Failed to send control message to emulator thread");
                    ui_state.current_view = Views::Fullscreen;
                }
            }
        });

    egui::TopBottomPanel::top("game list search bar").show(egui_context, |ui| {
        let response = ui.text_edit_singleline(&mut ui_state.search_string);
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            scan_game_folder(ui_state);
            ui_state.game_list = ui_state
                .game_list
                .clone()
                .into_iter()
                .filter(|game| {
                    game.to_ascii_lowercase()
                        .contains(&ui_state.search_string.to_ascii_lowercase())
                })
                .collect();
        }
    });

    egui::CentralPanel::default().show(egui_context, |ui| {
        let available_width = size.width as f32 * 3.0 / 4.0;
        let row_height = 300.0;

        if ui_state.game_list.is_empty() {
            scan_game_folder(ui_state);
        }

        egui::ScrollArea::vertical().show_rows(
            ui,
            row_height,
            ui_state.game_list.len().div_ceil(5),
            |ui, row_range| {
                ui.horizontal_wrapped(|ui| {
                    for row in row_range {
                        egui_extras::StripBuilder::new(ui)
                            .sizes(Size::remainder(), 5)
                            .horizontal(|mut strip| {
                                for col in row * 5..row * 5 + 5 {
                                    if col < ui_state.game_list.len() {
                                        render_game_card(
                                            row_height,
                                            &ui_state.game_list,
                                            &mut strip,
                                            col,
                                            &mut ui_state.selected_game,
                                        );
                                    }
                                }
                            });

                        ui.end_row();
                    }
                });
            },
        );
    });
}

fn scan_game_folder(ui_state: &mut UIState) {
    let game_dir =
        THREAD_LOCAL_CONFIG.with(|c| c.borrow_mut().load().ui_config.game_folder.clone());
    let paths = fs::read_dir(game_dir).unwrap();
    ui_state.game_list = paths
        .filter_map(|dir_entry| dir_entry.ok())
        .map(|dir_entry| dir_entry.file_name().into_string().unwrap())
        .collect();

    ui_state.game_list.sort_unstable();

    ui_state.selected_game = ui_state.game_list[0].clone();
}

fn render_game_card(
    row_height: f32,
    games: &[String],
    strip: &mut Strip,
    col: usize,
    selected_game: &mut String,
) {
    strip.cell(|ui| {
        let response = ui.scope_builder(
            UiBuilder::new()
                .id_salt(format!("{}", col))
                .sense(Sense::click()),
            |ui| {
                egui::Frame::new()
                    .fill(Color32::from_rgb(22, 22, 22))
                    .inner_margin(Margin::same(4))
                    .outer_margin(Margin::same(2))
                    .stroke(Stroke::new(2.0, if games[col] != *selected_game { Color32::from_rgb(47, 47, 47) } else { Color32::from_rgb(0, 91, 127) }))
                    .corner_radius(CornerRadius::same(3))
                    .show(ui, |ui| {
                        let mut game_name = games[col].clone();
                        game_name.truncate(games[col].len() - 3);
                        let cleaned_name = game_name.clone();
                        game_name = encode(&game_name).to_string();
                        ui.add(
                            egui::Image::new(format!("https://thumbnails.libretro.com/Nintendo%20-%20Game%20Boy/Named_Boxarts/{}.png", game_name))
                                .maintain_aspect_ratio(true)
                                .fit_to_exact_size(Vec2 {
                                    x: ui.available_width(),
                                    y: row_height,
                                })
                                .sense(Sense::empty()),
                        );
                        ui.add(Label::new(cleaned_name).selectable(false));
                    });
            }).response;

        if response.clicked() {
            *selected_game = games[col].clone();
        }
    });
}
