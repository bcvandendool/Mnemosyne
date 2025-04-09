use crate::emulator::EmulatorState;
use crate::gb::registers::Flag;
use crate::ui::UIState;
use egui::{Context, Ui};
use egui_extras::{Column, TableBuilder};
use std::ops::Add;

pub(crate) fn render(
    ui: &mut Ui,
    egui_context: &Context,
    ui_state: &mut UIState,
    emu_state: &EmulatorState,
) {
    match emu_state {
        EmulatorState::GameBoy(emu_state) => {
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
                        let mut text = String::new();
                        if emu_state.registers.has_flag(Flag::ZERO) {
                            text = text.add("Z");
                        }
                        if emu_state.registers.has_flag(Flag::SUBTRACTION) {
                            text = text.add("N");
                        }
                        if emu_state.registers.has_flag(Flag::HALF_CARRY) {
                            text = text.add("H");
                        }
                        if emu_state.registers.has_flag(Flag::CARRY) {
                            text = text.add("C");
                        }
                        ui.label(text);
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
                });
            });

            ui.label(format!("Reg IR: {:#06X}", emu_state.registers.IR));
        }
    }
}
