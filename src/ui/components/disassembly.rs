use crate::emulator::EmulatorState;
use crate::ui::UIState;
use crate::ui::{as_byte_range, UIContext};
use egui::text::{LayoutJob, LayoutSection};
use egui::{Context, TextFormat, TextStyle, Ui};
use egui_extras::{Column, TableBuilder};
use syntect::easy::HighlightLines;
use syntect::highlighting::FontStyle;

pub(crate) fn render(
    ui: &mut Ui,
    ui_context: &mut UIContext,
    egui_context: &Context,
    ui_state: &mut UIState,
    emu_state: &EmulatorState,
) {
    let syntax = ui_context
        .ps
        .find_syntax_by_extension("rgbasm")
        .expect("Failed to find rgbasm syntax definition");

    let mut h = HighlightLines::new(syntax, &ui_context.ts.themes["base16-ocean.light"]);

    // Disassembler
    let available_height = ui.available_height();
    let disassembly_table = TableBuilder::new(ui)
        .id_salt(1)
        .striped(true)
        .resizable(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .sense(egui::Sense::click())
        .column(Column::exact(45.0).clip(false))
        .column(Column::remainder())
        .min_scrolled_height(0.0)
        .max_scroll_height(available_height);

    let index = 1;
    // let index = ui_context
    //     .disassembly
    //     .iter()
    //     .position(|a| a.0.is_some() && a.clone().0.unwrap().address == emu_state.registers.PC)
    //     .unwrap_or(0);
    // // TODO: proper handling, will error if outside of disassembled area!!!
    // // TODO: handle disassembly of RAM
    //
    // if emu_state.registers.PC != ui_context.previous_pc {
    //     disassembly_table = disassembly_table.scroll_to_row(index, Option::from(Align::Center));
    //     ui_context.previous_pc = emu_state.registers.PC;
    // }

    disassembly_table.body(|mut body| {
        body.rows(18.0, ui_context.disassembly.len(), |mut row| {
            let row_index = row.index();

            if ui_context.disassembly[row_index].0.is_some() {
                row.set_selected(
                    ui_state
                        .breakpoints
                        .breakpoints
                        .contains(&(ui_context.disassembly[row_index].clone().0.unwrap().address)),
                );
            }

            row.set_hovered(row_index == index);

            if let Some(address) = ui_context.disassembly[row_index].clone().0 {
                row.col(|ui| {
                    ui.label(format!("{:#06X}", address.address));
                });
            }

            row.col(|ui| {
                let text = &ui_context.disassembly[row_index].1;

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
                            font_id: ui
                                .style()
                                .override_font_id
                                .clone()
                                .unwrap_or_else(|| TextStyle::Monospace.resolve(ui.style())),
                            color: text_color,
                            italics,
                            underline,
                            ..Default::default()
                        },
                    });
                }

                ui.label(job);
            });

            if ui_context.disassembly[row_index].0.is_none() {
                row.col(|_| {});
            }

            ui_state
                .toggle_row_selection(ui_context.disassembly[row_index].clone().0, &row.response());
        })
    });
}
