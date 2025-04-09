use crate::emulator::EmulatorState;
use crate::ui::{Memories, UIContext, UIState};
use egui::{Align, Context, Ui};
use egui_extras::{Column, TableBuilder};

pub(crate) fn render(
    ui: &mut Ui,
    egui_context: &Context,
    ui_state: &mut UIState,
    emu_state: &EmulatorState,
) {
    match emu_state {
        EmulatorState::GameBoy(emu_state) => {
            puffin::profile_scope!("UI - Memory viewer");
            ui.label("Memory viewer");
            egui::ComboBox::from_label("Select memory to view")
                .selected_text(format!("{:?}", ui_state.selected_memory))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut ui_state.selected_memory, Memories::WRAM1, "WRAM1");
                    ui.selectable_value(&mut ui_state.selected_memory, Memories::WRAM2, "WRAM2");
                    ui.selectable_value(&mut ui_state.selected_memory, Memories::HRAM, "HRAM");
                    ui.selectable_value(&mut ui_state.selected_memory, Memories::OAM, "OAM");
                    ui.selectable_value(
                        &mut ui_state.selected_memory,
                        Memories::BackgroundMaps,
                        "Background maps",
                    );
                    ui.selectable_value(
                        &mut ui_state.selected_memory,
                        Memories::TileData,
                        "Tile data",
                    );
                });
            //ui.separator();

            if ui_state.selected_memory == Memories::WRAM1
                || ui_state.selected_memory == Memories::WRAM2
                || ui_state.selected_memory == Memories::HRAM
                || ui_state.selected_memory == Memories::BackgroundMaps
                || ui_state.selected_memory == Memories::OAM
            {
                // Memory table
                //ui.label("Memory view");
                let memory_table = TableBuilder::new(ui)
                    .id_salt(0)
                    .striped(true)
                    .resizable(false)
                    .cell_layout(egui::Layout::left_to_right(Align::Center))
                    .column(Column::auto())
                    .columns(Column::exact(16.0).clip(false), 8)
                    .column(Column::exact(2.0))
                    .columns(Column::exact(16.0).clip(false), 8);

                let memory_prefix = match ui_state.selected_memory {
                    Memories::WRAM1 => 0xC000,
                    Memories::WRAM2 => 0xD000,
                    Memories::HRAM => 0xFF80,
                    Memories::OAM => 0xFE00,
                    _ => 0x0,
                };

                memory_table.body(|mut body| {
                    body.rows(18.0, emu_state.ram.len().div_ceil(16), |mut row| {
                        let row_index = row.index();

                        row.col(|ui| {
                            ui.label(format!("{:#06X}:  ", memory_prefix + (row_index << 4)));
                        });

                        for i in 0x00..=0x07 {
                            row.col(|ui| {
                                ui.label(format!("{:02X}", emu_state.ram[(row_index << 4) + i]));
                            });
                        }

                        row.col(|_| {});

                        for i in 0x08..=0x0F {
                            row.col(|ui| {
                                ui.label(format!("{:02X}", emu_state.ram[(row_index << 4) + i]));
                            });
                        }
                    })
                });
            }

            if ui_state.selected_memory == Memories::TileData {
                if emu_state.ram.len() != 6144 {
                    // Data not loaded yet, skip
                } else {
                    // TODO: do not create images every frame, only on data received from emu
                    // and make sure that it actually changed, as the image creation is expensive
                    for j in 0..24 {
                        ui.horizontal(|ui| {
                            for k in 0..16 {
                                let i = j * 8 + k;
                                let mut image_bytes: Vec<egui::Color32> = Vec::new();
                                for y in 0..8 {
                                    let tile_hi = emu_state.ram[i * 16 + y * 2];
                                    let tile_lo = emu_state.ram[i * 16 + y * 2 + 1];
                                    for x in 0..8 {
                                        let idx = 7 - x;
                                        let color_id =
                                            ((tile_hi >> idx) & 1) << 1 | ((tile_lo >> idx) & 1);
                                        let color: u8 = match color_id {
                                            0 => 0xFF,
                                            1 => 0xAA,
                                            2 => 0x55,
                                            3 => 0x00,
                                            _ => panic!("Received invalid color code"),
                                        };
                                        let pixel = egui::Color32::from_rgb(color, color, color);
                                        image_bytes.push(pixel)
                                    }
                                }
                                let image = egui::ColorImage {
                                    size: [8, 8],
                                    pixels: image_bytes,
                                };
                                let texture = egui_context.load_texture(
                                    format!("tile_{}", i),
                                    image,
                                    egui::TextureOptions::NEAREST,
                                );
                                let size = texture.size_vec2();
                                let sized_texture = egui::load::SizedTexture::new(&texture, size);
                                ui.add(
                                    egui::Image::new(sized_texture)
                                        .fit_to_exact_size([size.x * 2.0, size.y * 2.0].into()),
                                );
                            }
                        });
                    }
                }
            }
        }
    }
}
