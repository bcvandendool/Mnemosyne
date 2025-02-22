use crate::emulator::EmulatorState;
use crate::gameboy::disassembler::Disassembler;
use crate::gameboy::registers::Flag;
use egui::text::{LayoutJob, LayoutSection};
use egui::{Align, Context, TextFormat, TextStyle};
use egui_extras::{Column, TableBuilder};
use std::collections::HashSet;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::{SyntaxSet, SyntaxSetBuilder};
use winit::dpi::PhysicalSize;

#[derive(Clone)]
pub struct UIState {
    pub(crate) emulator_running: bool,
    pub(crate) emulator_should_step: bool,
    breakpoints: HashSet<usize>,
    pub(crate) selected_memory: Memories,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Memories {
    ExternalRAM,
    WRAM1,
    WRAM2,
    TileData,
    BackgroundMaps,
    OAM,
    HRAM,
    IO,
}

pub(crate) struct UIContext {
    ts: ThemeSet,
    ps: SyntaxSet,
    previous_pc: u16,
    disassembler: Disassembler,
    boot_rom: Vec<(u16, String)>,
}

impl UIContext {
    pub(crate) fn new() -> Self {
        let ts = ThemeSet::load_defaults();
        let mut builder = SyntaxSetBuilder::new();
        builder
            .add_from_folder(Path::new("./"), false)
            .expect("Failed to load syntax files");
        let ps = builder.build();
        let disassembler = Disassembler::new();
        let boot_rom = disassembler.disassemble_section(
            &include_bytes!("../tests/game-boy-test-roms/artifacts/mooneye-test-suite/emulator-only/mbc5/rom_512kb.gb")
                .to_vec(),
            0x00,
            0x7FFF,
        );

        UIContext {
            ts,
            ps,
            previous_pc: 0x00FF,
            disassembler,
            boot_rom,
        }
    }
}

impl UIState {
    pub(crate) fn new() -> Self {
        UIState {
            emulator_running: false,
            emulator_should_step: false,
            breakpoints: HashSet::new(),
            selected_memory: Memories::WRAM1,
        }
    }

    fn toggle_row_selection(&mut self, row_index: usize, row_response: &egui::Response) {
        if row_response.clicked() {
            if self.breakpoints.contains(&row_index) {
                self.breakpoints.remove(&row_index);
            } else {
                self.breakpoints.insert(row_index);
            }
        }
    }
}

pub(crate) fn create_ui(
    egui_context: &Context,
    size: PhysicalSize<u32>,
    ui_state: &mut UIState,
    emu_state: EmulatorState,
    ui_context: &mut UIContext,
) {
    // Calculate size of gameboy screen
    let mut gameboy_width = (size.width as f32 / 6.0) * 4.0;
    let mut gameboy_height = (gameboy_width / 10.0) * 9.0;

    if gameboy_height > (size.height as f32 / 5.0) * 4.0 {
        gameboy_height = (size.height as f32 / 5.0) * 4.0;
        gameboy_width = (gameboy_height / 9.0) * 10.0;
    }
    let gameboy_offset_x = (size.width as f32 / 2.0 - gameboy_width / 2.0) * 1.20;

    egui::SidePanel::left("left_panel")
        .resizable(false)
        .exact_width(gameboy_offset_x.round())
        .show(egui_context, |ui| {
            ui.label("Memory viewer");
            egui::ComboBox::from_label("Select memory to view")
                .selected_text(format!("{:?}", ui_state.selected_memory))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut ui_state.selected_memory, Memories::WRAM1, "WRAM1");
                    ui.selectable_value(&mut ui_state.selected_memory, Memories::WRAM2, "WRAM2");
                    ui.selectable_value(&mut ui_state.selected_memory, Memories::HRAM, "HRAM");
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
            ui.separator();

            if ui_state.selected_memory == Memories::WRAM1
                || ui_state.selected_memory == Memories::WRAM2
                || ui_state.selected_memory == Memories::HRAM
                || ui_state.selected_memory == Memories::BackgroundMaps
            {
                // Memory table
                ui.label("Memory view");
                let memory_table = TableBuilder::new(ui)
                    .id_salt(0)
                    .striped(true)
                    .resizable(false)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::auto())
                    .columns(Column::exact(18.0), 16);

                let memory_prefix = match ui_state.selected_memory {
                    Memories::WRAM1 => 0xC000,
                    Memories::WRAM2 => 0xD000,
                    Memories::HRAM => 0xFF80,
                    _ => 0x0,
                };

                memory_table
                    .header(18.0, |mut header| {
                        header.col(|ui| {
                            ui.label("Address");
                        });
                        for i in 0x00..=0x0F {
                            header.col(|ui| {
                                ui.label(format!("{:X}   ", i));
                            });
                        }
                    })
                    .body(|mut body| {
                        body.rows(18.0, emu_state.ram.len().div_ceil(16), |mut row| {
                            let row_index = row.index();

                            row.col(|ui| {
                                ui.label(format!("{:#06X}", memory_prefix + (row_index << 4)));
                            });

                            for i in 0x00..=0x0F {
                                row.col(|ui| {
                                    ui.label(format!("{:X}", emu_state.ram[(row_index << 4) + i]));
                                });
                            }
                        })
                    });
            }

            if ui_state.selected_memory == Memories::TileData {
                if emu_state.ram.len() != 6144 {
                    // Data not loaded yet, skip
                } else {
                    // TODO: do not create images every frame, only on data recevied from emu
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
        });

    egui::SidePanel::right("right_panel")
        .resizable(false)
        .exact_width(size.width as f32 - gameboy_offset_x.round() - gameboy_width.round())
        .show(egui_context, |ui| {
            ui.label("Gameboy emulator controls");

            // Emulator controls
            ui.horizontal(|ui| {
                let button_text = if ui_state.emulator_running {
                    "Pause"
                } else {
                    "Start"
                };
                if ui.button(button_text).clicked() {
                    ui_state.emulator_running = !ui_state.emulator_running;
                }

                ui_state.emulator_should_step = ui.button("Step").clicked();
            });
            ui.separator();

            // Register viewer
            ui.label("Register view");
            let register_table = TableBuilder::new(ui)
                .id_salt(0)
                .striped(false)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto());

            register_table.body(|mut body| {
                body.row(18.0, |mut row| {
                    row.col(|ui| {
                        ui.label("Reg A: ");
                    });
                    row.col(|ui| {
                        ui.label(format!("{:#04X}", emu_state.registers.A));
                    });
                    row.col(|ui| {
                        ui.label("Reg F: ");
                    });
                    row.col(|ui| {
                        ui.label(format!("{:#04X}", emu_state.registers.F));
                    });
                    row.col(|ui| {
                        ui.label("Flags: ");
                    });
                    row.col(|ui| {
                        let mut flags = Vec::new();
                        if emu_state.registers.has_flag(Flag::ZERO) {
                            flags.push("Z");
                        }
                        if emu_state.registers.has_flag(Flag::SUBTRACTION) {
                            flags.push("N");
                        }
                        if emu_state.registers.has_flag(Flag::HALF_CARRY) {
                            flags.push("H");
                        }
                        if emu_state.registers.has_flag(Flag::CARRY) {
                            flags.push("C");
                        }
                        ui.label(format!("{:?}", flags));
                    });
                });
                body.row(18.0, |mut row| {
                    row.col(|ui| {
                        ui.label("Reg B: ");
                    });
                    row.col(|ui| {
                        ui.label(format!("{:#04X}", emu_state.registers.B));
                    });
                    row.col(|ui| {
                        ui.label("Reg C: ");
                    });
                    row.col(|ui| {
                        ui.label(format!("{:#04X}", emu_state.registers.C));
                    });
                    row.col(|ui| {
                        ui.label("Reg SP: ");
                    });
                    row.col(|ui| {
                        ui.label(format!("{:#06X}", emu_state.registers.SP));
                    });
                });
                body.row(18.0, |mut row| {
                    row.col(|ui| {
                        ui.label("Reg D: ");
                    });
                    row.col(|ui| {
                        ui.label(format!("{:#04X}", emu_state.registers.D));
                    });
                    row.col(|ui| {
                        ui.label("Reg E: ");
                    });
                    row.col(|ui| {
                        ui.label(format!("{:#04X}", emu_state.registers.E));
                    });
                    row.col(|ui| {
                        ui.label("Reg PC: ");
                    });
                    row.col(|ui| {
                        ui.label(format!("{:#06X}", emu_state.registers.PC));
                    });
                });
                body.row(18.0, |mut row| {
                    row.col(|ui| {
                        ui.label("Reg H: ");
                    });
                    row.col(|ui| {
                        ui.label(format!("{:#04X}", emu_state.registers.H));
                    });
                    row.col(|ui| {
                        ui.label("Reg L: ");
                    });
                    row.col(|ui| {
                        ui.label(format!("{:#04X}", emu_state.registers.L));
                    });
                    row.col(|ui| {
                        ui.label("Reg IME: ");
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", u8::from(emu_state.registers.IME)));
                    });
                })
            });
            ui.separator();

            let syntax = ui_context
                .ps
                .find_syntax_by_extension("rgbasm")
                .expect("Failed to find rgbasm syntax definition");

            let mut h = HighlightLines::new(syntax, &ui_context.ts.themes["base16-mocha.dark"]);

            // Disassembler
            let available_height = ui.available_height();
            let mut disassembly_table = TableBuilder::new(ui)
                .id_salt(1)
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .sense(egui::Sense::click())
                .column(Column::auto())
                .column(Column::remainder())
                .min_scrolled_height(0.0)
                .max_scroll_height(available_height);

            let index = ui_context
                .boot_rom
                .iter()
                .position(|a| a.0 == emu_state.registers.PC)
                .unwrap_or(0);
            // TODO: proper handling, will error if outside of disassembled area!!!
            // TODO: handle disassembly of RAM

            if emu_state.registers.PC != ui_context.previous_pc {
                disassembly_table =
                    disassembly_table.scroll_to_row(index, Option::from(Align::Center));
                ui_context.previous_pc = emu_state.registers.PC;
            }

            disassembly_table.body(|mut body| {
                body.rows(18.0, ui_context.boot_rom.len(), |mut row| {
                    let row_index = row.index();

                    row.set_selected(ui_state.breakpoints.contains(&row_index));
                    row.set_hovered(row_index == index);

                    row.col(|ui| {
                        ui.label(format!("{:#06X}", ui_context.boot_rom[row_index].0));
                    });
                    row.col(|ui| {
                        let text = &ui_context.boot_rom[row_index].1;

                        let mut job = LayoutJob {
                            text: text.clone(),
                            ..Default::default()
                        };

                        for (style, range) in h
                            .highlight_line(text.as_str(), &ui_context.ps)
                            .ok()
                            .unwrap()
                        {
                            let fg = style.foreground;
                            let text_color = egui::Color32::from_rgb(fg.r, fg.g, fg.b);
                            let italics = style.font_style.contains(FontStyle::ITALIC);
                            let underline = style.font_style.contains(FontStyle::ITALIC);
                            let underline = if underline {
                                egui::Stroke::new(1.0, text_color)
                            } else {
                                egui::Stroke::NONE
                            };
                            job.sections.push(LayoutSection {
                                leading_space: 0.0,
                                byte_range: as_byte_range(text.as_str(), range),
                                format: TextFormat {
                                    font_id: ui.style().override_font_id.clone().unwrap_or_else(
                                        || TextStyle::Monospace.resolve(ui.style()),
                                    ),
                                    color: text_color,
                                    italics,
                                    underline,
                                    ..Default::default()
                                },
                            });
                        }

                        ui.label(job);
                    });

                    ui_state.toggle_row_selection(row_index, &row.response());
                })
            });

            // Breakpoint list
            // TODO:
        });

    egui::TopBottomPanel::bottom("bottom_panel")
        .exact_height(size.height as f32 - gameboy_height.round())
        .show(egui_context, |ui| ui.label("Log output"));
}

fn as_byte_range(whole: &str, range: &str) -> std::ops::Range<usize> {
    let whole_start = whole.as_ptr() as usize;
    let range_start = range.as_ptr() as usize;
    assert!(whole_start <= range_start);
    assert!(range_start + range.len() <= whole_start + whole.len());
    let offset = range_start - whole_start;
    offset..(offset + range.len())
}
