#![allow(non_snake_case)]
#![feature(adt_const_params)]
#![allow(incomplete_features)]

use crate::gb::breakpoints::Breakpoints;
use crate::gb::mmu::MMU;
use crate::gb::registers::{ConditionCode, Flag, Reg, Registers};
use log::{log, Level};
use std::time::Duration;

pub struct CPU {
    pub(crate) registers: Registers,
    pub(crate) mmu: MMU,
    to_set_IME: u8,
    halted: bool,
    halt_bug: bool,
    pub breakpoints: Breakpoints,
    pub(crate) time_ppu: Duration,
    pub(crate) time_io: Duration,
}

impl CPU {
    pub fn new(registers: Registers, mmu: MMU) -> Self {
        CPU {
            registers,
            mmu,
            to_set_IME: 0,
            halted: false,
            halt_bug: false,
            breakpoints: Breakpoints::new(),
            time_ppu: Duration::new(0, 0),
            time_io: Duration::new(0, 0),
        }
    }

    fn fetch_byte(&mut self) -> u8 {
        let value = self.mmu.read(self.registers.PC);
        if self.halt_bug {
            self.halt_bug = false;
        } else {
            self.registers.PC = self.registers.PC.wrapping_add(1);
        }
        value
    }

    fn handle_interrupt(&mut self) -> u32 {
        self.tick_dot(4);
        self.tick_dot(4);

        let value = self.registers.PC;
        self.registers.SP = self.registers.SP.wrapping_sub(1);
        self.mmu.write(self.registers.SP, (value >> 8) as u8);
        let IE = self.mmu.read(0xFFFF);
        let IF = self.mmu.read(0xFF0F);
        self.tick_dot(4);

        self.registers.SP = self.registers.SP.wrapping_sub(1);
        self.mmu.write(self.registers.SP, value as u8);

        self.registers.IME = false;
        let mut address = 0x00;
        if IF & 0b1 > 0 && IE & 0b1 > 0 {
            // Check VBlank
            self.mmu.write(0xFF0F, IF & !0b1);
            address = 0x40;
        } else if IF & 0b10 > 0 && IE & 0b10 > 0 {
            // Check LCD
            self.mmu.write(0xFF0F, IF & !0b10);
            address = 0x48;
        } else if IF & 0b100 > 0 && IE & 0b100 > 0 {
            // Check Timer
            self.mmu.write(0xFF0F, IF & !0b100);
            address = 0x50;
        } else if IF & 0b1000 > 0 && IE & 0b1000 > 0 {
            // Check Serial
            self.mmu.write(0xFF0F, IF & !0b1000);
            address = 0x58;
        } else if IF & 0b10000 > 0 && IE & 0b10000 > 0 {
            // Check Joypad
            self.mmu.write(0xFF0F, IF & !0b10000);
            address = 0x60;
        }

        self.registers.PC = address;
        self.tick_dot(4);
        self.tick_dot(4);
        5
    }

    fn tick_dot(&mut self, cycles: u32) {
        let mut start = fastant::Instant::now();
        for _ in 0..cycles {
            self.mmu.ppu.tick();
        }
        self.time_ppu += start.elapsed();

        start = fastant::Instant::now();
        for _ in 0..cycles {
            let DIV_APU = self.mmu.io_registers.update_timers();
            self.mmu.tick();
            self.mmu.handle_ppu_interrupts();
            self.mmu.apu.tick(DIV_APU);
        }
        self.time_io += start.elapsed();
    }

    pub(crate) fn process_instruction(&mut self) -> (bool, u32) {
        if self.halted {
            if self.mmu.read(0xFFFF) & self.mmu.read(0xFF0F) & 0x1F > 0 {
                self.halted = false;
                if self.registers.IME {
                    return (false, self.handle_interrupt());
                }
            } else {
                self.tick_dot(4);
                return (false, 1);
            }
        }

        if self.registers.IME && (self.mmu.read(0xFFFF) & self.mmu.read(0xFF0F) & 0x1F > 0) {
            return (false, self.handle_interrupt());
        }

        self.registers.IR = self.fetch_byte() as u16;
        self.tick_dot(4);

        let cycles = match self.registers.IR {
            0x00 => 1,
            0x01 => self.instr_LD_r16_n16::<{ Reg::BC }>(),
            0x02 => self.instr_LD_r16_A::<{ Reg::BC }>(),
            0x03 => self.instr_INC_r16::<{ Reg::BC }>(),
            0x04 => self.instr_INC_r8::<{ Reg::B }>(),
            0x05 => self.instr_DEC_r8::<{ Reg::B }>(),
            0x06 => self.instr_LD_r8_n8::<{ Reg::B }>(),
            0x07 => self.instr_RLCA(),
            0x08 => self.instr_LD_n16_SP(),
            0x09 => self.instr_ADD_HL_r16::<{ Reg::BC }>(),
            0x0A => self.instr_LD_A_r16::<{ Reg::BC }>(),
            0x0B => self.instr_DEC_r16::<{ Reg::BC }>(),
            0x0C => self.instr_INC_r8::<{ Reg::C }>(),
            0x0D => self.instr_DEC_r8::<{ Reg::C }>(),
            0x0E => self.instr_LD_r8_n8::<{ Reg::C }>(),
            0x0F => self.instr_RRCA(),
            0x10 => self.instr_STOP(),
            0x11 => self.instr_LD_r16_n16::<{ Reg::DE }>(),
            0x12 => self.instr_LD_r16_A::<{ Reg::DE }>(),
            0x13 => self.instr_INC_r16::<{ Reg::DE }>(),
            0x14 => self.instr_INC_r8::<{ Reg::D }>(),
            0x15 => self.instr_DEC_r8::<{ Reg::D }>(),
            0x16 => self.instr_LD_r8_n8::<{ Reg::D }>(),
            0x17 => self.instr_RLA(),
            0x18 => self.instr_JR_n16(),
            0x19 => self.instr_ADD_HL_r16::<{ Reg::DE }>(),
            0x1A => self.instr_LD_A_r16::<{ Reg::DE }>(),
            0x1B => self.instr_DEC_r16::<{ Reg::DE }>(),
            0x1C => self.instr_INC_r8::<{ Reg::E }>(),
            0x1D => self.instr_DEC_r8::<{ Reg::E }>(),
            0x1E => self.instr_LD_r8_n8::<{ Reg::E }>(),
            0x1F => self.instr_RRA(),
            0x20 => self.instr_JR_cc_n16::<{ ConditionCode::NZ }>(),
            0x21 => self.instr_LD_r16_n16::<{ Reg::HL }>(),
            0x22 => self.instr_LD_HLI_A(),
            0x23 => self.instr_INC_r16::<{ Reg::HL }>(),
            0x24 => self.instr_INC_r8::<{ Reg::H }>(),
            0x25 => self.instr_DEC_r8::<{ Reg::H }>(),
            0x26 => self.instr_LD_r8_n8::<{ Reg::H }>(),
            0x27 => self.instr_DAA(),
            0x28 => self.instr_JR_cc_n16::<{ ConditionCode::Z }>(),
            0x29 => self.instr_ADD_HL_r16::<{ Reg::HL }>(),
            0x2A => self.instr_LD_A_HLI(),
            0x2B => self.instr_DEC_r16::<{ Reg::HL }>(),
            0x2C => self.instr_INC_r8::<{ Reg::L }>(),
            0x2D => self.instr_DEC_r8::<{ Reg::L }>(),
            0x2E => self.instr_LD_r8_n8::<{ Reg::L }>(),
            0x2F => self.instr_CPL(),
            0x30 => self.instr_JR_cc_n16::<{ ConditionCode::NC }>(),
            0x31 => self.instr_LD_r16_n16::<{ Reg::SP }>(),
            0x32 => self.instr_LD_HLD_A(),
            0x33 => self.instr_INC_r16::<{ Reg::SP }>(),
            0x34 => self.instr_INC_HL(),
            0x35 => self.instr_DEC_HL(),
            0x36 => self.instr_LD_HL_n8(),
            0x37 => self.instr_SCF(),
            0x38 => self.instr_JR_cc_n16::<{ ConditionCode::C }>(),
            0x39 => self.instr_ADD_HL_r16::<{ Reg::SP }>(),
            0x3A => self.instr_LD_A_HLD(),
            0x3B => self.instr_DEC_r16::<{ Reg::SP }>(),
            0x3C => self.instr_INC_r8::<{ Reg::A }>(),
            0x3D => self.instr_DEC_r8::<{ Reg::A }>(),
            0x3E => self.instr_LD_r8_n8::<{ Reg::A }>(),
            0x3F => self.instr_CCF(),
            0x40 => self.instr_LD_r8_r8::<{ Reg::B }, { Reg::B }>(),
            0x41 => self.instr_LD_r8_r8::<{ Reg::B }, { Reg::C }>(),
            0x42 => self.instr_LD_r8_r8::<{ Reg::B }, { Reg::D }>(),
            0x43 => self.instr_LD_r8_r8::<{ Reg::B }, { Reg::E }>(),
            0x44 => self.instr_LD_r8_r8::<{ Reg::B }, { Reg::H }>(),
            0x45 => self.instr_LD_r8_r8::<{ Reg::B }, { Reg::L }>(),
            0x46 => self.instr_LD_r8_HL::<{ Reg::B }>(),
            0x47 => self.instr_LD_r8_r8::<{ Reg::B }, { Reg::A }>(),
            0x48 => self.instr_LD_r8_r8::<{ Reg::C }, { Reg::B }>(),
            0x49 => self.instr_LD_r8_r8::<{ Reg::C }, { Reg::C }>(),
            0x4A => self.instr_LD_r8_r8::<{ Reg::C }, { Reg::D }>(),
            0x4B => self.instr_LD_r8_r8::<{ Reg::C }, { Reg::E }>(),
            0x4C => self.instr_LD_r8_r8::<{ Reg::C }, { Reg::H }>(),
            0x4D => self.instr_LD_r8_r8::<{ Reg::C }, { Reg::L }>(),
            0x4E => self.instr_LD_r8_HL::<{ Reg::C }>(),
            0x4F => self.instr_LD_r8_r8::<{ Reg::C }, { Reg::A }>(),
            0x50 => self.instr_LD_r8_r8::<{ Reg::D }, { Reg::B }>(),
            0x51 => self.instr_LD_r8_r8::<{ Reg::D }, { Reg::C }>(),
            0x52 => self.instr_LD_r8_r8::<{ Reg::D }, { Reg::D }>(),
            0x53 => self.instr_LD_r8_r8::<{ Reg::D }, { Reg::E }>(),
            0x54 => self.instr_LD_r8_r8::<{ Reg::D }, { Reg::H }>(),
            0x55 => self.instr_LD_r8_r8::<{ Reg::D }, { Reg::L }>(),
            0x56 => self.instr_LD_r8_HL::<{ Reg::D }>(),
            0x57 => self.instr_LD_r8_r8::<{ Reg::D }, { Reg::A }>(),
            0x58 => self.instr_LD_r8_r8::<{ Reg::E }, { Reg::B }>(),
            0x59 => self.instr_LD_r8_r8::<{ Reg::E }, { Reg::C }>(),
            0x5A => self.instr_LD_r8_r8::<{ Reg::E }, { Reg::D }>(),
            0x5B => self.instr_LD_r8_r8::<{ Reg::E }, { Reg::E }>(),
            0x5C => self.instr_LD_r8_r8::<{ Reg::E }, { Reg::H }>(),
            0x5D => self.instr_LD_r8_r8::<{ Reg::E }, { Reg::L }>(),
            0x5E => self.instr_LD_r8_HL::<{ Reg::E }>(),
            0x5F => self.instr_LD_r8_r8::<{ Reg::E }, { Reg::A }>(),
            0x60 => self.instr_LD_r8_r8::<{ Reg::H }, { Reg::B }>(),
            0x61 => self.instr_LD_r8_r8::<{ Reg::H }, { Reg::C }>(),
            0x62 => self.instr_LD_r8_r8::<{ Reg::H }, { Reg::D }>(),
            0x63 => self.instr_LD_r8_r8::<{ Reg::H }, { Reg::E }>(),
            0x64 => self.instr_LD_r8_r8::<{ Reg::H }, { Reg::H }>(),
            0x65 => self.instr_LD_r8_r8::<{ Reg::H }, { Reg::L }>(),
            0x66 => self.instr_LD_r8_HL::<{ Reg::H }>(),
            0x67 => self.instr_LD_r8_r8::<{ Reg::H }, { Reg::A }>(),
            0x68 => self.instr_LD_r8_r8::<{ Reg::L }, { Reg::B }>(),
            0x69 => self.instr_LD_r8_r8::<{ Reg::L }, { Reg::C }>(),
            0x6A => self.instr_LD_r8_r8::<{ Reg::L }, { Reg::D }>(),
            0x6B => self.instr_LD_r8_r8::<{ Reg::L }, { Reg::E }>(),
            0x6C => self.instr_LD_r8_r8::<{ Reg::L }, { Reg::H }>(),
            0x6D => self.instr_LD_r8_r8::<{ Reg::L }, { Reg::L }>(),
            0x6E => self.instr_LD_r8_HL::<{ Reg::L }>(),
            0x6F => self.instr_LD_r8_r8::<{ Reg::L }, { Reg::A }>(),
            0x70 => self.instr_LD_HL_r8::<{ Reg::B }>(),
            0x71 => self.instr_LD_HL_r8::<{ Reg::C }>(),
            0x72 => self.instr_LD_HL_r8::<{ Reg::D }>(),
            0x73 => self.instr_LD_HL_r8::<{ Reg::E }>(),
            0x74 => self.instr_LD_HL_r8::<{ Reg::H }>(),
            0x75 => self.instr_LD_HL_r8::<{ Reg::L }>(),
            0x76 => self.instr_HALT(),
            0x77 => self.instr_LD_HL_r8::<{ Reg::A }>(),
            0x78 => self.instr_LD_r8_r8::<{ Reg::A }, { Reg::B }>(),
            0x79 => self.instr_LD_r8_r8::<{ Reg::A }, { Reg::C }>(),
            0x7A => self.instr_LD_r8_r8::<{ Reg::A }, { Reg::D }>(),
            0x7B => self.instr_LD_r8_r8::<{ Reg::A }, { Reg::E }>(),
            0x7C => self.instr_LD_r8_r8::<{ Reg::A }, { Reg::H }>(),
            0x7D => self.instr_LD_r8_r8::<{ Reg::A }, { Reg::L }>(),
            0x7E => self.instr_LD_r8_HL::<{ Reg::A }>(),
            0x7F => self.instr_LD_r8_r8::<{ Reg::A }, { Reg::A }>(),
            0x80 => self.instr_ADD_A_r8::<{ Reg::B }>(),
            0x81 => self.instr_ADD_A_r8::<{ Reg::C }>(),
            0x82 => self.instr_ADD_A_r8::<{ Reg::D }>(),
            0x83 => self.instr_ADD_A_r8::<{ Reg::E }>(),
            0x84 => self.instr_ADD_A_r8::<{ Reg::H }>(),
            0x85 => self.instr_ADD_A_r8::<{ Reg::L }>(),
            0x86 => self.instr_ADD_A_HL(),
            0x87 => self.instr_ADD_A_r8::<{ Reg::A }>(),
            0x88 => self.instr_ADC_A_r8::<{ Reg::B }>(),
            0x89 => self.instr_ADC_A_r8::<{ Reg::C }>(),
            0x8A => self.instr_ADC_A_r8::<{ Reg::D }>(),
            0x8B => self.instr_ADC_A_r8::<{ Reg::E }>(),
            0x8C => self.instr_ADC_A_r8::<{ Reg::H }>(),
            0x8D => self.instr_ADC_A_r8::<{ Reg::L }>(),
            0x8E => self.instr_ADC_A_HL(),
            0x8F => self.instr_ADC_A_r8::<{ Reg::A }>(),
            0x90 => self.instr_SUB_A_r8::<{ Reg::B }>(),
            0x91 => self.instr_SUB_A_r8::<{ Reg::C }>(),
            0x92 => self.instr_SUB_A_r8::<{ Reg::D }>(),
            0x93 => self.instr_SUB_A_r8::<{ Reg::E }>(),
            0x94 => self.instr_SUB_A_r8::<{ Reg::H }>(),
            0x95 => self.instr_SUB_A_r8::<{ Reg::L }>(),
            0x96 => self.instr_SUB_A_HL(),
            0x97 => self.instr_SUB_A_r8::<{ Reg::A }>(),
            0x98 => self.instr_SBC_A_r8::<{ Reg::B }>(),
            0x99 => self.instr_SBC_A_r8::<{ Reg::C }>(),
            0x9A => self.instr_SBC_A_r8::<{ Reg::D }>(),
            0x9B => self.instr_SBC_A_r8::<{ Reg::E }>(),
            0x9C => self.instr_SBC_A_r8::<{ Reg::H }>(),
            0x9D => self.instr_SBC_A_r8::<{ Reg::L }>(),
            0x9E => self.instr_SBC_A_HL(),
            0x9F => self.instr_SBC_A_r8::<{ Reg::A }>(),
            0xA0 => self.instr_AND_A_r8::<{ Reg::B }>(),
            0xA1 => self.instr_AND_A_r8::<{ Reg::C }>(),
            0xA2 => self.instr_AND_A_r8::<{ Reg::D }>(),
            0xA3 => self.instr_AND_A_r8::<{ Reg::E }>(),
            0xA4 => self.instr_AND_A_r8::<{ Reg::H }>(),
            0xA5 => self.instr_AND_A_r8::<{ Reg::L }>(),
            0xA6 => self.instr_AND_A_HL(),
            0xA7 => self.instr_AND_A_r8::<{ Reg::A }>(),
            0xA8 => self.instr_XOR_A_r8::<{ Reg::B }>(),
            0xA9 => self.instr_XOR_A_r8::<{ Reg::C }>(),
            0xAA => self.instr_XOR_A_r8::<{ Reg::D }>(),
            0xAB => self.instr_XOR_A_r8::<{ Reg::E }>(),
            0xAC => self.instr_XOR_A_r8::<{ Reg::H }>(),
            0xAD => self.instr_XOR_A_r8::<{ Reg::L }>(),
            0xAE => self.instr_XOR_A_HL(),
            0xAF => self.instr_XOR_A_r8::<{ Reg::A }>(),
            0xB0 => self.instr_OR_A_r8::<{ Reg::B }>(),
            0xB1 => self.instr_OR_A_r8::<{ Reg::C }>(),
            0xB2 => self.instr_OR_A_r8::<{ Reg::D }>(),
            0xB3 => self.instr_OR_A_r8::<{ Reg::E }>(),
            0xB4 => self.instr_OR_A_r8::<{ Reg::H }>(),
            0xB5 => self.instr_OR_A_r8::<{ Reg::L }>(),
            0xB6 => self.instr_OR_A_HL(),
            0xB7 => self.instr_OR_A_r8::<{ Reg::A }>(),
            0xB8 => self.instr_CP_A_r8::<{ Reg::B }>(),
            0xB9 => self.instr_CP_A_r8::<{ Reg::C }>(),
            0xBA => self.instr_CP_A_r8::<{ Reg::D }>(),
            0xBB => self.instr_CP_A_r8::<{ Reg::E }>(),
            0xBC => self.instr_CP_A_r8::<{ Reg::H }>(),
            0xBD => self.instr_CP_A_r8::<{ Reg::L }>(),
            0xBE => self.instr_CP_A_HL(),
            0xBF => self.instr_CP_A_r8::<{ Reg::A }>(),
            0xC0 => self.instr_RET_cc::<{ ConditionCode::NZ }>(),
            0xC1 => self.instr_POP_r16::<{ Reg::BC }>(),
            0xC2 => self.instr_JP_cc_a16::<{ ConditionCode::NZ }>(),
            0xC3 => self.instr_JP_a16(),
            0xC4 => self.instr_CALL_cc_a16::<{ ConditionCode::NZ }>(),
            0xC5 => self.instr_PUSH_r16::<{ Reg::BC }>(),
            0xC6 => self.instr_ADD_A_n8(),
            0xC7 => self.instr_RST(0x00),
            0xC8 => self.instr_RET_cc::<{ ConditionCode::Z }>(),
            0xC9 => self.instr_RET(),
            0xCA => self.instr_JP_cc_a16::<{ ConditionCode::Z }>(),
            0xCB => self.process_CB_instruction(),
            0xCC => self.instr_CALL_cc_a16::<{ ConditionCode::Z }>(),
            0xCD => self.instr_CALL_a16(),
            0xCE => self.instr_ADC_A_n8(),
            0xCF => self.instr_RST(0x08),
            0xD0 => self.instr_RET_cc::<{ ConditionCode::NC }>(),
            0xD1 => self.instr_POP_r16::<{ Reg::DE }>(),
            0xD2 => self.instr_JP_cc_a16::<{ ConditionCode::NC }>(),
            0xD4 => self.instr_CALL_cc_a16::<{ ConditionCode::NC }>(),
            0xD5 => self.instr_PUSH_r16::<{ Reg::DE }>(),
            0xD6 => self.instr_SUB_A_n8(),
            0xD7 => self.instr_RST(0x10),
            0xD8 => self.instr_RET_cc::<{ ConditionCode::C }>(),
            0xD9 => self.instr_RETI(),
            0xDA => self.instr_JP_cc_a16::<{ ConditionCode::C }>(),
            0xDC => self.instr_CALL_cc_a16::<{ ConditionCode::C }>(),
            0xDE => self.instr_SBC_A_n8(),
            0xDF => self.instr_RST(0x18),
            0xE0 => self.instr_LDH_n16_A(),
            0xE1 => self.instr_POP_r16::<{ Reg::HL }>(),
            0xE2 => self.instr_LDH_C_A(),
            0xE5 => self.instr_PUSH_r16::<{ Reg::HL }>(),
            0xE6 => self.instr_AND_A_n8(),
            0xE7 => self.instr_RST(0x20),
            0xE8 => self.instr_ADD_SP_e8(),
            0xE9 => self.instr_JP_HL(),
            0xEA => self.instr_LD_n16_A(),
            0xEE => self.instr_XOR_A_n8(),
            0xEF => self.instr_RST(0x28),
            0xF0 => self.instr_LDH_A_n16(),
            0xF1 => self.instr_POP_r16::<{ Reg::AF }>(),
            0xF2 => self.instr_LDH_A_C(),
            0xF3 => self.instr_DI(),
            0xF5 => self.instr_PUSH_r16::<{ Reg::AF }>(),
            0xF6 => self.instr_OR_A_n8(),
            0xF7 => self.instr_RST(0x30),
            0xF8 => self.instr_LD_HL_SP_e8(),
            0xF9 => self.instr_LD_SP_HL(),
            0xFA => self.instr_LD_A_n16(),
            0xFB => self.instr_EI(),
            0xFE => self.instr_CP_A_n8(),
            0xFF => self.instr_RST(0x38),
            _ => panic!(
                "Received invalid opcode: {:#04X}, PC={:#06X}",
                self.registers.IR, self.registers.PC
            ),
        };

        let mut hit_breakpoint: bool = false;
        if self.breakpoints.breakpoints.contains(&self.registers.PC) {
            hit_breakpoint = true;
        }

        if self.registers.IR == 0x40 {
            hit_breakpoint = true;
        }

        if self.to_set_IME == 1 {
            self.to_set_IME = 2;
        } else if self.to_set_IME == 2 {
            self.to_set_IME = 0;
            self.registers.IME = true;
        }

        (hit_breakpoint, cycles)
    }

    fn process_CB_instruction(&mut self) -> u32 {
        let instr = self.fetch_byte();
        self.tick_dot(4);
        match instr {
            0x00 => self.instr_RLC_r8::<{ Reg::B }>(),
            0x01 => self.instr_RLC_r8::<{ Reg::C }>(),
            0x02 => self.instr_RLC_r8::<{ Reg::D }>(),
            0x03 => self.instr_RLC_r8::<{ Reg::E }>(),
            0x04 => self.instr_RLC_r8::<{ Reg::H }>(),
            0x05 => self.instr_RLC_r8::<{ Reg::L }>(),
            0x06 => self.instr_RLC_HL(),
            0x07 => self.instr_RLC_r8::<{ Reg::A }>(),
            0x08 => self.instr_RRC_r8::<{ Reg::B }>(),
            0x09 => self.instr_RRC_r8::<{ Reg::C }>(),
            0x0A => self.instr_RRC_r8::<{ Reg::D }>(),
            0x0B => self.instr_RRC_r8::<{ Reg::E }>(),
            0x0C => self.instr_RRC_r8::<{ Reg::H }>(),
            0x0D => self.instr_RRC_r8::<{ Reg::L }>(),
            0x0E => self.instr_RRC_HL(),
            0x0F => self.instr_RRC_r8::<{ Reg::A }>(),
            0x10 => self.instr_RL_r8::<{ Reg::B }>(),
            0x11 => self.instr_RL_r8::<{ Reg::C }>(),
            0x12 => self.instr_RL_r8::<{ Reg::D }>(),
            0x13 => self.instr_RL_r8::<{ Reg::E }>(),
            0x14 => self.instr_RL_r8::<{ Reg::H }>(),
            0x15 => self.instr_RL_r8::<{ Reg::L }>(),
            0x16 => self.instr_RL_HL(),
            0x17 => self.instr_RL_r8::<{ Reg::A }>(),
            0x18 => self.instr_RR_r8::<{ Reg::B }>(),
            0x19 => self.instr_RR_r8::<{ Reg::C }>(),
            0x1A => self.instr_RR_r8::<{ Reg::D }>(),
            0x1B => self.instr_RR_r8::<{ Reg::E }>(),
            0x1C => self.instr_RR_r8::<{ Reg::H }>(),
            0x1D => self.instr_RR_r8::<{ Reg::L }>(),
            0x1E => self.instr_RR_HL(),
            0x1F => self.instr_RR_r8::<{ Reg::A }>(),
            0x20 => self.instr_SLA_r8::<{ Reg::B }>(),
            0x21 => self.instr_SLA_r8::<{ Reg::C }>(),
            0x22 => self.instr_SLA_r8::<{ Reg::D }>(),
            0x23 => self.instr_SLA_r8::<{ Reg::E }>(),
            0x24 => self.instr_SLA_r8::<{ Reg::H }>(),
            0x25 => self.instr_SLA_r8::<{ Reg::L }>(),
            0x26 => self.instr_SLA_HL(),
            0x27 => self.instr_SLA_r8::<{ Reg::A }>(),
            0x28 => self.instr_SRA_r8::<{ Reg::B }>(),
            0x29 => self.instr_SRA_r8::<{ Reg::C }>(),
            0x2A => self.instr_SRA_r8::<{ Reg::D }>(),
            0x2B => self.instr_SRA_r8::<{ Reg::E }>(),
            0x2C => self.instr_SRA_r8::<{ Reg::H }>(),
            0x2D => self.instr_SRA_r8::<{ Reg::L }>(),
            0x2E => self.instr_SRA_HL(),
            0x2F => self.instr_SRA_r8::<{ Reg::A }>(),
            0x30 => self.instr_SWAP_r8::<{ Reg::B }>(),
            0x31 => self.instr_SWAP_r8::<{ Reg::C }>(),
            0x32 => self.instr_SWAP_r8::<{ Reg::D }>(),
            0x33 => self.instr_SWAP_r8::<{ Reg::E }>(),
            0x34 => self.instr_SWAP_r8::<{ Reg::H }>(),
            0x35 => self.instr_SWAP_r8::<{ Reg::L }>(),
            0x36 => self.instr_SWAP_HL(),
            0x37 => self.instr_SWAP_r8::<{ Reg::A }>(),
            0x38 => self.instr_SRL_r8::<{ Reg::B }>(),
            0x39 => self.instr_SRL_r8::<{ Reg::C }>(),
            0x3A => self.instr_SRL_r8::<{ Reg::D }>(),
            0x3B => self.instr_SRL_r8::<{ Reg::E }>(),
            0x3C => self.instr_SRL_r8::<{ Reg::H }>(),
            0x3D => self.instr_SRL_r8::<{ Reg::L }>(),
            0x3E => self.instr_SRL_HL(),
            0x3F => self.instr_SRL_r8::<{ Reg::A }>(),
            0x40 => self.instr_BIT_u3_r8::<0, { Reg::B }>(),
            0x41 => self.instr_BIT_u3_r8::<0, { Reg::C }>(),
            0x42 => self.instr_BIT_u3_r8::<0, { Reg::D }>(),
            0x43 => self.instr_BIT_u3_r8::<0, { Reg::E }>(),
            0x44 => self.instr_BIT_u3_r8::<0, { Reg::H }>(),
            0x45 => self.instr_BIT_u3_r8::<0, { Reg::L }>(),
            0x46 => self.instr_BIT_u3_HL::<0>(),
            0x47 => self.instr_BIT_u3_r8::<0, { Reg::A }>(),
            0x48 => self.instr_BIT_u3_r8::<1, { Reg::B }>(),
            0x49 => self.instr_BIT_u3_r8::<1, { Reg::C }>(),
            0x4A => self.instr_BIT_u3_r8::<1, { Reg::D }>(),
            0x4B => self.instr_BIT_u3_r8::<1, { Reg::E }>(),
            0x4C => self.instr_BIT_u3_r8::<1, { Reg::H }>(),
            0x4D => self.instr_BIT_u3_r8::<1, { Reg::L }>(),
            0x4E => self.instr_BIT_u3_HL::<1>(),
            0x4F => self.instr_BIT_u3_r8::<1, { Reg::A }>(),
            0x50 => self.instr_BIT_u3_r8::<2, { Reg::B }>(),
            0x51 => self.instr_BIT_u3_r8::<2, { Reg::C }>(),
            0x52 => self.instr_BIT_u3_r8::<2, { Reg::D }>(),
            0x53 => self.instr_BIT_u3_r8::<2, { Reg::E }>(),
            0x54 => self.instr_BIT_u3_r8::<2, { Reg::H }>(),
            0x55 => self.instr_BIT_u3_r8::<2, { Reg::L }>(),
            0x56 => self.instr_BIT_u3_HL::<2>(),
            0x57 => self.instr_BIT_u3_r8::<2, { Reg::A }>(),
            0x58 => self.instr_BIT_u3_r8::<3, { Reg::B }>(),
            0x59 => self.instr_BIT_u3_r8::<3, { Reg::C }>(),
            0x5A => self.instr_BIT_u3_r8::<3, { Reg::D }>(),
            0x5B => self.instr_BIT_u3_r8::<3, { Reg::E }>(),
            0x5C => self.instr_BIT_u3_r8::<3, { Reg::H }>(),
            0x5D => self.instr_BIT_u3_r8::<3, { Reg::L }>(),
            0x5E => self.instr_BIT_u3_HL::<3>(),
            0x5F => self.instr_BIT_u3_r8::<3, { Reg::A }>(),
            0x60 => self.instr_BIT_u3_r8::<4, { Reg::B }>(),
            0x61 => self.instr_BIT_u3_r8::<4, { Reg::C }>(),
            0x62 => self.instr_BIT_u3_r8::<4, { Reg::D }>(),
            0x63 => self.instr_BIT_u3_r8::<4, { Reg::E }>(),
            0x64 => self.instr_BIT_u3_r8::<4, { Reg::H }>(),
            0x65 => self.instr_BIT_u3_r8::<4, { Reg::L }>(),
            0x66 => self.instr_BIT_u3_HL::<4>(),
            0x67 => self.instr_BIT_u3_r8::<4, { Reg::A }>(),
            0x68 => self.instr_BIT_u3_r8::<5, { Reg::B }>(),
            0x69 => self.instr_BIT_u3_r8::<5, { Reg::C }>(),
            0x6A => self.instr_BIT_u3_r8::<5, { Reg::D }>(),
            0x6B => self.instr_BIT_u3_r8::<5, { Reg::E }>(),
            0x6C => self.instr_BIT_u3_r8::<5, { Reg::H }>(),
            0x6D => self.instr_BIT_u3_r8::<5, { Reg::L }>(),
            0x6E => self.instr_BIT_u3_HL::<5>(),
            0x6F => self.instr_BIT_u3_r8::<5, { Reg::A }>(),
            0x70 => self.instr_BIT_u3_r8::<6, { Reg::B }>(),
            0x71 => self.instr_BIT_u3_r8::<6, { Reg::C }>(),
            0x72 => self.instr_BIT_u3_r8::<6, { Reg::D }>(),
            0x73 => self.instr_BIT_u3_r8::<6, { Reg::E }>(),
            0x74 => self.instr_BIT_u3_r8::<6, { Reg::H }>(),
            0x75 => self.instr_BIT_u3_r8::<6, { Reg::L }>(),
            0x76 => self.instr_BIT_u3_HL::<6>(),
            0x77 => self.instr_BIT_u3_r8::<6, { Reg::A }>(),
            0x78 => self.instr_BIT_u3_r8::<7, { Reg::B }>(),
            0x79 => self.instr_BIT_u3_r8::<7, { Reg::C }>(),
            0x7A => self.instr_BIT_u3_r8::<7, { Reg::D }>(),
            0x7B => self.instr_BIT_u3_r8::<7, { Reg::E }>(),
            0x7C => self.instr_BIT_u3_r8::<7, { Reg::H }>(),
            0x7D => self.instr_BIT_u3_r8::<7, { Reg::L }>(),
            0x7E => self.instr_BIT_u3_HL::<7>(),
            0x7F => self.instr_BIT_u3_r8::<7, { Reg::A }>(),
            0x80 => self.instr_RES_u3_r8::<0, { Reg::B }>(),
            0x81 => self.instr_RES_u3_r8::<0, { Reg::C }>(),
            0x82 => self.instr_RES_u3_r8::<0, { Reg::D }>(),
            0x83 => self.instr_RES_u3_r8::<0, { Reg::E }>(),
            0x84 => self.instr_RES_u3_r8::<0, { Reg::H }>(),
            0x85 => self.instr_RES_u3_r8::<0, { Reg::L }>(),
            0x86 => self.instr_RES_u3_HL::<0>(),
            0x87 => self.instr_RES_u3_r8::<0, { Reg::A }>(),
            0x88 => self.instr_RES_u3_r8::<1, { Reg::B }>(),
            0x89 => self.instr_RES_u3_r8::<1, { Reg::C }>(),
            0x8A => self.instr_RES_u3_r8::<1, { Reg::D }>(),
            0x8B => self.instr_RES_u3_r8::<1, { Reg::E }>(),
            0x8C => self.instr_RES_u3_r8::<1, { Reg::H }>(),
            0x8D => self.instr_RES_u3_r8::<1, { Reg::L }>(),
            0x8E => self.instr_RES_u3_HL::<1>(),
            0x8F => self.instr_RES_u3_r8::<1, { Reg::A }>(),
            0x90 => self.instr_RES_u3_r8::<2, { Reg::B }>(),
            0x91 => self.instr_RES_u3_r8::<2, { Reg::C }>(),
            0x92 => self.instr_RES_u3_r8::<2, { Reg::D }>(),
            0x93 => self.instr_RES_u3_r8::<2, { Reg::E }>(),
            0x94 => self.instr_RES_u3_r8::<2, { Reg::H }>(),
            0x95 => self.instr_RES_u3_r8::<2, { Reg::L }>(),
            0x96 => self.instr_RES_u3_HL::<2>(),
            0x97 => self.instr_RES_u3_r8::<2, { Reg::A }>(),
            0x98 => self.instr_RES_u3_r8::<3, { Reg::B }>(),
            0x99 => self.instr_RES_u3_r8::<3, { Reg::C }>(),
            0x9A => self.instr_RES_u3_r8::<3, { Reg::D }>(),
            0x9B => self.instr_RES_u3_r8::<3, { Reg::E }>(),
            0x9C => self.instr_RES_u3_r8::<3, { Reg::H }>(),
            0x9D => self.instr_RES_u3_r8::<3, { Reg::L }>(),
            0x9E => self.instr_RES_u3_HL::<3>(),
            0x9F => self.instr_RES_u3_r8::<3, { Reg::A }>(),
            0xA0 => self.instr_RES_u3_r8::<4, { Reg::B }>(),
            0xA1 => self.instr_RES_u3_r8::<4, { Reg::C }>(),
            0xA2 => self.instr_RES_u3_r8::<4, { Reg::D }>(),
            0xA3 => self.instr_RES_u3_r8::<4, { Reg::E }>(),
            0xA4 => self.instr_RES_u3_r8::<4, { Reg::H }>(),
            0xA5 => self.instr_RES_u3_r8::<4, { Reg::L }>(),
            0xA6 => self.instr_RES_u3_HL::<4>(),
            0xA7 => self.instr_RES_u3_r8::<4, { Reg::A }>(),
            0xA8 => self.instr_RES_u3_r8::<5, { Reg::B }>(),
            0xA9 => self.instr_RES_u3_r8::<5, { Reg::C }>(),
            0xAA => self.instr_RES_u3_r8::<5, { Reg::D }>(),
            0xAB => self.instr_RES_u3_r8::<5, { Reg::E }>(),
            0xAC => self.instr_RES_u3_r8::<5, { Reg::H }>(),
            0xAD => self.instr_RES_u3_r8::<5, { Reg::L }>(),
            0xAE => self.instr_RES_u3_HL::<5>(),
            0xAF => self.instr_RES_u3_r8::<5, { Reg::A }>(),
            0xB0 => self.instr_RES_u3_r8::<6, { Reg::B }>(),
            0xB1 => self.instr_RES_u3_r8::<6, { Reg::C }>(),
            0xB2 => self.instr_RES_u3_r8::<6, { Reg::D }>(),
            0xB3 => self.instr_RES_u3_r8::<6, { Reg::E }>(),
            0xB4 => self.instr_RES_u3_r8::<6, { Reg::H }>(),
            0xB5 => self.instr_RES_u3_r8::<6, { Reg::L }>(),
            0xB6 => self.instr_RES_u3_HL::<6>(),
            0xB7 => self.instr_RES_u3_r8::<6, { Reg::A }>(),
            0xB8 => self.instr_RES_u3_r8::<7, { Reg::B }>(),
            0xB9 => self.instr_RES_u3_r8::<7, { Reg::C }>(),
            0xBA => self.instr_RES_u3_r8::<7, { Reg::D }>(),
            0xBB => self.instr_RES_u3_r8::<7, { Reg::E }>(),
            0xBC => self.instr_RES_u3_r8::<7, { Reg::H }>(),
            0xBD => self.instr_RES_u3_r8::<7, { Reg::L }>(),
            0xBE => self.instr_RES_u3_HL::<7>(),
            0xBF => self.instr_RES_u3_r8::<7, { Reg::A }>(),
            0xC0 => self.instr_SET_u3_r8::<0, { Reg::B }>(),
            0xC1 => self.instr_SET_u3_r8::<0, { Reg::C }>(),
            0xC2 => self.instr_SET_u3_r8::<0, { Reg::D }>(),
            0xC3 => self.instr_SET_u3_r8::<0, { Reg::E }>(),
            0xC4 => self.instr_SET_u3_r8::<0, { Reg::H }>(),
            0xC5 => self.instr_SET_u3_r8::<0, { Reg::L }>(),
            0xC6 => self.instr_SET_u3_HL::<0>(),
            0xC7 => self.instr_SET_u3_r8::<0, { Reg::A }>(),
            0xC8 => self.instr_SET_u3_r8::<1, { Reg::B }>(),
            0xC9 => self.instr_SET_u3_r8::<1, { Reg::C }>(),
            0xCA => self.instr_SET_u3_r8::<1, { Reg::D }>(),
            0xCB => self.instr_SET_u3_r8::<1, { Reg::E }>(),
            0xCC => self.instr_SET_u3_r8::<1, { Reg::H }>(),
            0xCD => self.instr_SET_u3_r8::<1, { Reg::L }>(),
            0xCE => self.instr_SET_u3_HL::<1>(),
            0xCF => self.instr_SET_u3_r8::<1, { Reg::A }>(),
            0xD0 => self.instr_SET_u3_r8::<2, { Reg::B }>(),
            0xD1 => self.instr_SET_u3_r8::<2, { Reg::C }>(),
            0xD2 => self.instr_SET_u3_r8::<2, { Reg::D }>(),
            0xD3 => self.instr_SET_u3_r8::<2, { Reg::E }>(),
            0xD4 => self.instr_SET_u3_r8::<2, { Reg::H }>(),
            0xD5 => self.instr_SET_u3_r8::<2, { Reg::L }>(),
            0xD6 => self.instr_SET_u3_HL::<2>(),
            0xD7 => self.instr_SET_u3_r8::<2, { Reg::A }>(),
            0xD8 => self.instr_SET_u3_r8::<3, { Reg::B }>(),
            0xD9 => self.instr_SET_u3_r8::<3, { Reg::C }>(),
            0xDA => self.instr_SET_u3_r8::<3, { Reg::D }>(),
            0xDB => self.instr_SET_u3_r8::<3, { Reg::E }>(),
            0xDC => self.instr_SET_u3_r8::<3, { Reg::H }>(),
            0xDD => self.instr_SET_u3_r8::<3, { Reg::L }>(),
            0xDE => self.instr_SET_u3_HL::<3>(),
            0xDF => self.instr_SET_u3_r8::<3, { Reg::A }>(),
            0xE0 => self.instr_SET_u3_r8::<4, { Reg::B }>(),
            0xE1 => self.instr_SET_u3_r8::<4, { Reg::C }>(),
            0xE2 => self.instr_SET_u3_r8::<4, { Reg::D }>(),
            0xE3 => self.instr_SET_u3_r8::<4, { Reg::E }>(),
            0xE4 => self.instr_SET_u3_r8::<4, { Reg::H }>(),
            0xE5 => self.instr_SET_u3_r8::<4, { Reg::L }>(),
            0xE6 => self.instr_SET_u3_HL::<4>(),
            0xE7 => self.instr_SET_u3_r8::<4, { Reg::A }>(),
            0xE8 => self.instr_SET_u3_r8::<5, { Reg::B }>(),
            0xE9 => self.instr_SET_u3_r8::<5, { Reg::C }>(),
            0xEA => self.instr_SET_u3_r8::<5, { Reg::D }>(),
            0xEB => self.instr_SET_u3_r8::<5, { Reg::E }>(),
            0xEC => self.instr_SET_u3_r8::<5, { Reg::H }>(),
            0xED => self.instr_SET_u3_r8::<5, { Reg::L }>(),
            0xEE => self.instr_SET_u3_HL::<5>(),
            0xEF => self.instr_SET_u3_r8::<5, { Reg::A }>(),
            0xF0 => self.instr_SET_u3_r8::<6, { Reg::B }>(),
            0xF1 => self.instr_SET_u3_r8::<6, { Reg::C }>(),
            0xF2 => self.instr_SET_u3_r8::<6, { Reg::D }>(),
            0xF3 => self.instr_SET_u3_r8::<6, { Reg::E }>(),
            0xF4 => self.instr_SET_u3_r8::<6, { Reg::H }>(),
            0xF5 => self.instr_SET_u3_r8::<6, { Reg::L }>(),
            0xF6 => self.instr_SET_u3_HL::<6>(),
            0xF7 => self.instr_SET_u3_r8::<6, { Reg::A }>(),
            0xF8 => self.instr_SET_u3_r8::<7, { Reg::B }>(),
            0xF9 => self.instr_SET_u3_r8::<7, { Reg::C }>(),
            0xFA => self.instr_SET_u3_r8::<7, { Reg::D }>(),
            0xFB => self.instr_SET_u3_r8::<7, { Reg::E }>(),
            0xFC => self.instr_SET_u3_r8::<7, { Reg::H }>(),
            0xFD => self.instr_SET_u3_r8::<7, { Reg::L }>(),
            0xFE => self.instr_SET_u3_HL::<7>(),
            0xFF => self.instr_SET_u3_r8::<7, { Reg::A }>(),
            _ => panic!("Received invalid BC opcode: {:}", instr),
        }
    }

    #[inline(always)]
    fn get_r8<const register: Reg>(&self) -> u8 {
        match register {
            Reg::A => self.registers.A,
            Reg::B => self.registers.B,
            Reg::C => self.registers.C,
            Reg::D => self.registers.D,
            Reg::E => self.registers.E,
            Reg::F => self.registers.F,
            Reg::H => self.registers.H,
            Reg::L => self.registers.L,
            _ => {
                panic!("get_r8 received invalid register: {:?}", register)
            }
        }
    }

    #[inline(always)]
    fn set_r8<const register: Reg>(&mut self, value: u8) {
        match register {
            Reg::A => {
                self.registers.A = value;
            }
            Reg::B => {
                self.registers.B = value;
            }
            Reg::C => {
                self.registers.C = value;
            }
            Reg::D => {
                self.registers.D = value;
            }
            Reg::E => {
                self.registers.E = value;
            }
            Reg::F => {
                self.registers.F = value;
            }
            Reg::H => {
                self.registers.H = value;
            }
            Reg::L => {
                self.registers.L = value;
            }
            _ => {
                panic!("set_r8 received invalid register: {:?}", register)
            }
        }
    }

    #[inline(always)]
    fn get_r16<const register: Reg>(&self) -> u16 {
        match register {
            Reg::AF => self.registers.AF(),
            Reg::BC => self.registers.BC(),
            Reg::DE => self.registers.DE(),
            Reg::HL => self.registers.HL(),
            Reg::SP => self.registers.SP,
            _ => {
                log!(
                    Level::Error,
                    "get_r16 received invalid register: {:?}",
                    register
                );
                panic!()
                // TODO: handle error better
            }
        }
    }

    #[inline(always)]
    fn set_r16<const register: Reg>(&mut self, value: u16) {
        match register {
            Reg::AF => self.registers.set_AF(value),
            Reg::BC => self.registers.set_BC(value),
            Reg::DE => self.registers.set_DE(value),
            Reg::HL => self.registers.set_HL(value),
            Reg::SP => self.registers.SP = value,
            _ => {
                log!(
                    Level::Error,
                    "get_r16 received invalid register: {:?}",
                    register
                )
            }
        }
    }

    fn instr_LD_r16_n16<const register: Reg>(&mut self) -> u32 {
        let lower = self.fetch_byte();
        self.tick_dot(4);
        let higher = self.fetch_byte();
        self.tick_dot(4);
        let immediate = ((higher as u16) << 8) | (lower as u16);

        self.set_r16::<register>(immediate);
        3
    }

    fn instr_LD_r16_A<const register: Reg>(&mut self) -> u32 {
        let address = self.get_r16::<register>();

        self.mmu.write(address, self.registers.A);
        self.tick_dot(4);
        2
    }

    fn instr_INC_r16<const register: Reg>(&mut self) -> u32 {
        self.tick_dot(4);
        match register {
            Reg::AF => self.registers.set_AF(self.registers.AF().wrapping_add(1)),
            Reg::BC => self.registers.set_BC(self.registers.BC().wrapping_add(1)),
            Reg::DE => self.registers.set_DE(self.registers.DE().wrapping_add(1)),
            Reg::HL => self.registers.set_HL(self.registers.HL().wrapping_add(1)),
            Reg::SP => self.registers.SP = self.registers.SP.wrapping_add(1),
            _ => {
                panic!("instr_INC_r16 received invalid register: {:?}", register)
            }
        }
        2
    }

    fn instr_INC_r8<const register: Reg>(&mut self) -> u32 {
        match register {
            Reg::A => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.A & 0xF) + 1 > 0xF);
                self.registers.A = self.registers.A.wrapping_add(1);
                self.registers.set_flag(Flag::SUBTRACTION, false);
                self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
            }
            Reg::B => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.B & 0xF) + 1 > 0xF);
                self.registers.B = self.registers.B.wrapping_add(1);
                self.registers.set_flag(Flag::SUBTRACTION, false);
                self.registers.set_flag(Flag::ZERO, self.registers.B == 0);
            }
            Reg::C => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.C & 0xF) + 1 > 0xF);
                self.registers.C = self.registers.C.wrapping_add(1);
                self.registers.set_flag(Flag::SUBTRACTION, false);
                self.registers.set_flag(Flag::ZERO, self.registers.C == 0);
            }
            Reg::D => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.D & 0xF) + 1 > 0xF);
                self.registers.D = self.registers.D.wrapping_add(1);
                self.registers.set_flag(Flag::SUBTRACTION, false);
                self.registers.set_flag(Flag::ZERO, self.registers.D == 0);
            }
            Reg::E => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.E & 0xF) + 1 > 0xF);
                self.registers.E = self.registers.E.wrapping_add(1);
                self.registers.set_flag(Flag::SUBTRACTION, false);
                self.registers.set_flag(Flag::ZERO, self.registers.E == 0);
            }
            Reg::F => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.F & 0xF) + 1 > 0xF);
                self.registers.F = self.registers.F.wrapping_add(1);
                self.registers.set_flag(Flag::SUBTRACTION, false);
                self.registers.set_flag(Flag::ZERO, self.registers.F == 0);
            }
            Reg::H => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.H & 0xF) + 1 > 0xF);
                self.registers.H = self.registers.H.wrapping_add(1);
                self.registers.set_flag(Flag::SUBTRACTION, false);
                self.registers.set_flag(Flag::ZERO, self.registers.H == 0);
            }
            Reg::L => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.L & 0xF) + 1 > 0xF);
                self.registers.L = self.registers.L.wrapping_add(1);
                self.registers.set_flag(Flag::SUBTRACTION, false);
                self.registers.set_flag(Flag::ZERO, self.registers.L == 0);
            }
            _ => {
                panic!("instr_INC_r8 received invalid register: {:?}", register)
            }
        }
        1
    }

    fn instr_DEC_r8<const register: Reg>(&mut self) -> u32 {
        match register {
            Reg::A => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.A & 0xF) == 0);
                self.registers.A = self.registers.A.wrapping_sub(1);
                self.registers.set_flag(Flag::SUBTRACTION, true);
                self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
            }
            Reg::B => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.B & 0xF) == 0);
                self.registers.B = self.registers.B.wrapping_sub(1);
                self.registers.set_flag(Flag::SUBTRACTION, true);
                self.registers.set_flag(Flag::ZERO, self.registers.B == 0);
            }
            Reg::C => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.C & 0xF) == 0);
                self.registers.C = self.registers.C.wrapping_sub(1);
                self.registers.set_flag(Flag::SUBTRACTION, true);
                self.registers.set_flag(Flag::ZERO, self.registers.C == 0);
            }
            Reg::D => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.D & 0xF) == 0);
                self.registers.D = self.registers.D.wrapping_sub(1);
                self.registers.set_flag(Flag::SUBTRACTION, true);
                self.registers.set_flag(Flag::ZERO, self.registers.D == 0);
            }
            Reg::E => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.E & 0xF) == 0);
                self.registers.E = self.registers.E.wrapping_sub(1);
                self.registers.set_flag(Flag::SUBTRACTION, true);
                self.registers.set_flag(Flag::ZERO, self.registers.E == 0);
            }
            Reg::F => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.F & 0xF) == 0);
                self.registers.F = self.registers.F.wrapping_sub(1);
                self.registers.set_flag(Flag::SUBTRACTION, true);
                self.registers.set_flag(Flag::ZERO, self.registers.F == 0);
            }
            Reg::H => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.H & 0xF) == 0);
                self.registers.H = self.registers.H.wrapping_sub(1);
                self.registers.set_flag(Flag::SUBTRACTION, true);
                self.registers.set_flag(Flag::ZERO, self.registers.H == 0);
            }
            Reg::L => {
                self.registers
                    .set_flag(Flag::HALF_CARRY, (self.registers.L & 0xF) == 0);
                self.registers.L = self.registers.L.wrapping_sub(1);
                self.registers.set_flag(Flag::SUBTRACTION, true);
                self.registers.set_flag(Flag::ZERO, self.registers.L == 0);
            }
            _ => {
                panic!("instr_DEC_r8 received invalid register: {:?}", register);
            }
        }
        1
    }

    fn instr_LD_r8_n8<const register: Reg>(&mut self) -> u32 {
        let immediate = self.fetch_byte();
        self.tick_dot(4);
        self.set_r8::<register>(immediate);
        2
    }

    fn instr_RLCA(&mut self) -> u32 {
        self.registers.A = self.registers.A.rotate_left(1);

        self.registers
            .set_flag(Flag::CARRY, (self.registers.A & 0x1) == 0x1);
        self.registers.set_flag(Flag::ZERO, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        1
    }

    fn instr_LD_n16_SP(&mut self) -> u32 {
        let address_lo = self.fetch_byte();
        self.tick_dot(4);
        let address_hi = self.fetch_byte();
        self.tick_dot(4);
        let address = ((address_hi as u16) << 8) | (address_lo as u16);
        self.mmu.write(address, (self.registers.SP & 0xFF) as u8);
        self.tick_dot(4);
        self.mmu.write(address + 1, (self.registers.SP >> 8) as u8);
        self.tick_dot(4);
        5
    }

    fn instr_ADD_HL_r16<const register: Reg>(&mut self) -> u32 {
        let value = self.get_r16::<register>();
        self.tick_dot(4);

        self.registers.set_flag(
            Flag::HALF_CARRY,
            (self.registers.HL() & 0xFFF) + (value & 0xFFF) > 0xFFF,
        );
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers
            .set_flag(Flag::CARRY, self.registers.HL() > 0xFFFF - value);
        self.registers
            .set_HL(self.registers.HL().wrapping_add(value));
        2
    }

    fn instr_LD_A_r16<const register: Reg>(&mut self) -> u32 {
        let address = self.get_r16::<register>();
        let value = self.mmu.read(address);
        self.tick_dot(4);

        self.registers.A = value;
        2
    }

    fn instr_DEC_r16<const register: Reg>(&mut self) -> u32 {
        self.tick_dot(4);
        match register {
            Reg::AF => self.registers.set_AF(self.registers.AF().wrapping_sub(1)),
            Reg::BC => self.registers.set_BC(self.registers.BC().wrapping_sub(1)),
            Reg::DE => self.registers.set_DE(self.registers.DE().wrapping_sub(1)),
            Reg::HL => self.registers.set_HL(self.registers.HL().wrapping_sub(1)),
            Reg::SP => self.registers.SP = self.registers.SP.wrapping_sub(1),
            _ => {
                panic!("instr_INC_r16 received invalid register: {:?}", register)
            }
        }
        2
    }

    fn instr_RRCA(&mut self) -> u32 {
        self.registers.A = self.registers.A.rotate_right(1);
        self.registers.set_flag(Flag::ZERO, false);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        self.registers
            .set_flag(Flag::CARRY, self.registers.A & 0x80 == 0x80);
        1
    }

    fn instr_RLA(&mut self) -> u32 {
        let carry = self.registers.has_flag(Flag::CARRY);
        let highest_bit = self.registers.A & 0x80 == 0x80;
        self.registers.A <<= 1;
        self.registers.set_flag(Flag::CARRY, highest_bit);
        if carry {
            self.registers.A |= 0x1;
        }
        self.registers.set_flag(Flag::ZERO, false);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        1
    }

    fn instr_STOP(&mut self) -> u32 {
        0
        // TODO: https://gbdev.io/pandocs/Reducing_Power_Consumption.html#using-the-stop-instruction
    }

    fn instr_JR_n16(&mut self) -> u32 {
        let offset = self.fetch_byte() as i8;
        self.tick_dot(4);
        self.tick_dot(4);
        if offset >= 0 {
            self.registers.PC = self.registers.PC.wrapping_add(offset as u16);
        } else {
            self.registers.PC = self.registers.PC.wrapping_sub(offset.unsigned_abs() as u16);
        }
        3
    }

    fn instr_RRA(&mut self) -> u32 {
        let carry = self.registers.has_flag(Flag::CARRY);
        let lowest_bit = self.registers.A & 0x01 == 0x01;
        self.registers.A >>= 1;
        self.registers.set_flag(Flag::CARRY, lowest_bit);
        if carry {
            self.registers.A |= 0x80;
        }
        self.registers.set_flag(Flag::ZERO, false);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        1
    }

    fn instr_JR_cc_n16<const cc: ConditionCode>(&mut self) -> u32 {
        let offset = self.fetch_byte() as i8;
        self.tick_dot(4);

        if (cc == ConditionCode::C && self.registers.has_flag(Flag::CARRY))
            || (cc == ConditionCode::NC && !self.registers.has_flag(Flag::CARRY))
            || (cc == ConditionCode::Z && self.registers.has_flag(Flag::ZERO))
            || (cc == ConditionCode::NZ && !self.registers.has_flag(Flag::ZERO))
        {
            self.tick_dot(4);
            if offset >= 0 {
                self.registers.PC = self.registers.PC.wrapping_add(offset as u16);
            } else {
                self.registers.PC = self.registers.PC.wrapping_sub((-(offset as i16)) as u16);
            }
            3
        } else {
            2
        }
    }

    fn instr_LD_HLI_A(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.registers.A;
        self.mmu.write(address, value);
        self.tick_dot(4);
        self.registers.set_HL(self.registers.HL() + 1);
        2
    }

    fn instr_LD_HLD_A(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.registers.A;
        self.mmu.write(address, value);
        self.tick_dot(4);
        self.registers.set_HL(self.registers.HL() - 1);
        2
    }

    fn instr_DAA(&mut self) -> u32 {
        let mut adjustment: u8 = 0x0;

        if (self.registers.has_flag(Flag::SUBTRACTION) && self.registers.has_flag(Flag::HALF_CARRY))
            || (!self.registers.has_flag(Flag::SUBTRACTION)
                && (self.registers.has_flag(Flag::HALF_CARRY) || (self.registers.A & 0xF > 0x9)))
        {
            adjustment += 0x6;
        }

        if (self.registers.has_flag(Flag::SUBTRACTION) && self.registers.has_flag(Flag::CARRY))
            || (!self.registers.has_flag(Flag::SUBTRACTION)
                && (self.registers.has_flag(Flag::CARRY) || self.registers.A > 0x99))
        {
            adjustment += 0x60;
            if !self.registers.has_flag(Flag::SUBTRACTION) {
                self.registers.set_flag(Flag::CARRY, true);
            }
        }

        if self.registers.has_flag(Flag::SUBTRACTION) {
            self.registers.A = self.registers.A.wrapping_sub(adjustment);
        } else {
            self.registers.A = self.registers.A.wrapping_add(adjustment);
        }

        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        1
    }

    fn instr_LD_A_HLI(&mut self) -> u32 {
        let address = self.registers.HL();
        self.registers.A = self.mmu.read(address);
        self.tick_dot(4);
        self.registers.set_HL(self.registers.HL().wrapping_add(1));
        2
    }

    fn instr_LD_A_HLD(&mut self) -> u32 {
        let address = self.registers.HL();
        self.registers.A = self.mmu.read(address);
        self.tick_dot(4);
        self.registers.set_HL(self.registers.HL().wrapping_sub(1));
        2
    }

    fn instr_CPL(&mut self) -> u32 {
        self.registers.A = !self.registers.A;
        self.registers.set_flag(Flag::SUBTRACTION, true);
        self.registers.set_flag(Flag::HALF_CARRY, true);
        1
    }

    fn instr_INC_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let old_value = self.mmu.read(address);
        self.tick_dot(4);
        let value = old_value.wrapping_add(1);
        self.mmu.write(address, value);
        self.tick_dot(4);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers
            .set_flag(Flag::HALF_CARRY, (old_value & 0xF) + 1 > 0xF);
        3
    }

    fn instr_DEC_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let old_value = self.mmu.read(address);
        self.tick_dot(4);
        let value = old_value.wrapping_sub(1);
        self.mmu.write(address, value);
        self.tick_dot(4);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, true);
        self.registers
            .set_flag(Flag::HALF_CARRY, (old_value & 0xF) == 0);
        3
    }

    fn instr_LD_HL_n8(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.fetch_byte();
        self.tick_dot(4);
        self.mmu.write(address, value);
        self.tick_dot(4);
        3
    }

    fn instr_SCF(&mut self) -> u32 {
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        self.registers.set_flag(Flag::CARRY, true);
        1
    }

    fn instr_CCF(&mut self) -> u32 {
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        self.registers
            .set_flag(Flag::CARRY, !self.registers.has_flag(Flag::CARRY));
        1
    }

    fn instr_LD_r8_r8<const register_a: Reg, const register_b: Reg>(&mut self) -> u32 {
        let value = self.get_r8::<register_b>();
        self.set_r8::<register_a>(value);
        1
    }

    fn instr_LD_r8_HL<const register: Reg>(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.mmu.read(address);
        self.tick_dot(4);
        self.set_r8::<register>(value);
        2
    }

    fn instr_LD_HL_r8<const register: Reg>(&mut self) -> u32 {
        let value = self.get_r8::<register>();
        let address = self.registers.HL();
        self.mmu.write(address, value);
        self.tick_dot(4);
        2
    }

    fn instr_HALT(&mut self) -> u32 {
        if self.registers.IME {
            self.halted = true;
        } else if self.mmu.read(0xFFFF) & self.mmu.read(0xFF0F) & 0x1F > 0 {
            self.halt_bug = true;
        } else {
            self.halted = true;
        }

        0
    }

    fn instr_ADD_A_r8<const register: Reg>(&mut self) -> u32 {
        let value = self.get_r8::<register>();
        self.registers.set_flag(
            Flag::HALF_CARRY,
            (self.registers.A & 0xF) + (value & 0xF) > 0xF,
        );
        let (new_value, overflowed) = self.registers.A.overflowing_add(value);
        self.registers.A = new_value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::CARRY, overflowed);
        1
    }

    fn instr_ADD_A_n8(&mut self) -> u32 {
        let value = self.fetch_byte();
        self.tick_dot(4);
        let (new_value, overflowed) = self.registers.A.overflowing_add(value);
        self.registers.set_flag(
            Flag::HALF_CARRY,
            (self.registers.A & 0xF) + (value & 0xF) > 0xF,
        );
        self.registers.A = new_value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::CARRY, overflowed);
        2
    }

    fn instr_ADD_A_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.mmu.read(address);
        self.tick_dot(4);
        let (new_value, overflowed) = self.registers.A.overflowing_add(value);
        self.registers.set_flag(
            Flag::HALF_CARRY,
            (self.registers.A & 0xF) + (value & 0xF) > 0xF,
        );
        self.registers.A = new_value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::CARRY, overflowed);
        2
    }

    fn instr_ADC_A_r8<const register: Reg>(&mut self) -> u32 {
        let value = self.get_r8::<register>();
        let (new_value_, overflowed_) = self.registers.A.overflowing_add(value);
        let (new_value, mut overflowed) =
            new_value_.overflowing_add(u8::from(self.registers.has_flag(Flag::CARRY)));
        overflowed |= overflowed_;
        self.registers.set_flag(
            Flag::HALF_CARRY,
            (self.registers.A & 0xF)
                + (value & 0xF)
                + u8::from(self.registers.has_flag(Flag::CARRY))
                > 0xF,
        );
        self.registers.A = new_value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::CARRY, overflowed);
        1
    }

    fn instr_ADC_A_n8(&mut self) -> u32 {
        let value = self.fetch_byte();
        self.tick_dot(4);
        let (new_value_, overflowed_) = self.registers.A.overflowing_add(value);
        let (new_value, mut overflowed) =
            new_value_.overflowing_add(u8::from(self.registers.has_flag(Flag::CARRY)));
        overflowed |= overflowed_;
        self.registers.set_flag(
            Flag::HALF_CARRY,
            (self.registers.A & 0xF)
                + (value & 0xF)
                + u8::from(self.registers.has_flag(Flag::CARRY))
                > 0xF,
        );
        self.registers.A = new_value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::CARRY, overflowed);
        2
    }

    fn instr_ADC_A_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.mmu.read(address);
        self.tick_dot(4);
        let (new_value_, overflowed_) = self.registers.A.overflowing_add(value);
        let (new_value, mut overflowed) =
            new_value_.overflowing_add(u8::from(self.registers.has_flag(Flag::CARRY)));
        overflowed |= overflowed_;
        self.registers.set_flag(
            Flag::HALF_CARRY,
            (self.registers.A & 0xF)
                + (value & 0xF)
                + u8::from(self.registers.has_flag(Flag::CARRY))
                > 0xF,
        );
        self.registers.A = new_value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::CARRY, overflowed);
        2
    }

    fn instr_SUB_A_r8<const register: Reg>(&mut self) -> u32 {
        let value = self.get_r8::<register>();
        let (new_value, overflowed) = self.registers.A.overflowing_sub(value);
        self.registers
            .set_flag(Flag::HALF_CARRY, (self.registers.A & 0xF) < (value & 0xF));
        self.registers.A = new_value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, true);
        self.registers.set_flag(Flag::CARRY, overflowed);
        1
    }

    fn instr_SUB_A_n8(&mut self) -> u32 {
        let value = self.fetch_byte();
        self.tick_dot(4);
        let (new_value, overflowed) = self.registers.A.overflowing_sub(value);
        self.registers
            .set_flag(Flag::HALF_CARRY, (self.registers.A & 0xF) < (value & 0xF));
        self.registers.A = new_value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, true);
        self.registers.set_flag(Flag::CARRY, overflowed);
        2
    }

    fn instr_SUB_A_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.mmu.read(address);
        self.tick_dot(4);
        let (new_value, overflowed) = self.registers.A.overflowing_sub(value);
        self.registers
            .set_flag(Flag::HALF_CARRY, (self.registers.A & 0xF) < (value & 0xF));
        self.registers.A = new_value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, true);
        self.registers.set_flag(Flag::CARRY, overflowed);
        2
    }

    fn instr_SBC_A_r8<const register: Reg>(&mut self) -> u32 {
        let value = self.get_r8::<register>();
        let (new_value_, overflowed_) = self.registers.A.overflowing_sub(value);
        let (new_value, mut overflowed) =
            new_value_.overflowing_sub(u8::from(self.registers.has_flag(Flag::CARRY)));
        overflowed |= overflowed_;
        self.registers.set_flag(
            Flag::HALF_CARRY,
            (self.registers.A & 0xF)
                < (value & 0xF) + u8::from(self.registers.has_flag(Flag::CARRY)),
        );
        self.registers.A = new_value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, true);
        self.registers.set_flag(Flag::CARRY, overflowed);
        1
    }

    fn instr_SBC_A_n8(&mut self) -> u32 {
        let value = self.fetch_byte();
        self.tick_dot(4);
        let (new_value_, overflowed_) = self.registers.A.overflowing_sub(value);
        let (new_value, mut overflowed) =
            new_value_.overflowing_sub(u8::from(self.registers.has_flag(Flag::CARRY)));
        overflowed |= overflowed_;
        self.registers.set_flag(
            Flag::HALF_CARRY,
            (self.registers.A & 0xF)
                < (value & 0xF) + u8::from(self.registers.has_flag(Flag::CARRY)),
        );
        self.registers.A = new_value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, true);
        self.registers.set_flag(Flag::CARRY, overflowed);
        2
    }

    fn instr_SBC_A_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.mmu.read(address);
        self.tick_dot(4);
        let (new_value_, overflowed_) = self.registers.A.overflowing_sub(value);
        let (new_value, mut overflowed) =
            new_value_.overflowing_sub(u8::from(self.registers.has_flag(Flag::CARRY)));
        overflowed |= overflowed_;
        self.registers.set_flag(
            Flag::HALF_CARRY,
            (self.registers.A & 0xF)
                < (value & 0xF) + u8::from(self.registers.has_flag(Flag::CARRY)),
        );
        self.registers.A = new_value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, true);
        self.registers.set_flag(Flag::CARRY, overflowed);
        2
    }

    fn instr_AND_A_r8<const register: Reg>(&mut self) -> u32 {
        let value = self.get_r8::<register>();
        self.registers.A &= value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, true);
        self.registers.set_flag(Flag::CARRY, false);
        1
    }

    fn instr_AND_A_n8(&mut self) -> u32 {
        let value = self.fetch_byte();
        self.tick_dot(4);
        self.registers.A &= value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, true);
        self.registers.set_flag(Flag::CARRY, false);
        2
    }

    fn instr_AND_A_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.mmu.read(address);
        self.tick_dot(4);
        self.registers.A &= value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, true);
        self.registers.set_flag(Flag::CARRY, false);
        2
    }

    fn instr_XOR_A_r8<const register: Reg>(&mut self) -> u32 {
        let value = self.get_r8::<register>();
        self.registers.A ^= value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        self.registers.set_flag(Flag::CARRY, false);
        1
    }

    fn instr_XOR_A_n8(&mut self) -> u32 {
        let value = self.fetch_byte();
        self.tick_dot(4);
        self.registers.A ^= value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        self.registers.set_flag(Flag::CARRY, false);
        2
    }

    fn instr_XOR_A_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.mmu.read(address);
        self.tick_dot(4);
        self.registers.A ^= value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        self.registers.set_flag(Flag::CARRY, false);
        2
    }

    fn instr_OR_A_r8<const register: Reg>(&mut self) -> u32 {
        let value = self.get_r8::<register>();
        self.registers.A |= value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        self.registers.set_flag(Flag::CARRY, false);
        1
    }

    fn instr_OR_A_n8(&mut self) -> u32 {
        let value = self.fetch_byte();
        self.tick_dot(4);
        self.registers.A |= value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        self.registers.set_flag(Flag::CARRY, false);
        2
    }

    fn instr_OR_A_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.mmu.read(address);
        self.tick_dot(4);
        self.registers.A |= value;
        self.registers.set_flag(Flag::ZERO, self.registers.A == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        self.registers.set_flag(Flag::CARRY, false);
        2
    }

    fn instr_CP_A_r8<const register: Reg>(&mut self) -> u32 {
        let value = self.get_r8::<register>();
        let (new_value, overflowed) = self.registers.A.overflowing_sub(value);
        self.registers
            .set_flag(Flag::HALF_CARRY, (self.registers.A & 0xF) < (value & 0xF));
        self.registers.set_flag(Flag::ZERO, new_value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, true);
        self.registers.set_flag(Flag::CARRY, overflowed);
        1
    }

    fn instr_CP_A_n8(&mut self) -> u32 {
        let value = self.fetch_byte();
        self.tick_dot(4);
        let (new_value, overflowed) = self.registers.A.overflowing_sub(value);
        self.registers
            .set_flag(Flag::HALF_CARRY, (self.registers.A & 0xF) < (value & 0xF));
        self.registers.set_flag(Flag::ZERO, new_value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, true);
        self.registers.set_flag(Flag::CARRY, overflowed);
        2
    }

    fn instr_CP_A_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.mmu.read(address);
        self.tick_dot(4);
        let (new_value, overflowed) = self.registers.A.overflowing_sub(value);
        self.registers
            .set_flag(Flag::HALF_CARRY, (self.registers.A & 0xF) < (value & 0xF));
        self.registers.set_flag(Flag::ZERO, new_value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, true);
        self.registers.set_flag(Flag::CARRY, overflowed);
        2
    }

    fn instr_RET_cc<const cc: ConditionCode>(&mut self) -> u32 {
        self.tick_dot(4);
        if (cc == ConditionCode::C && self.registers.has_flag(Flag::CARRY))
            || (cc == ConditionCode::NC && !self.registers.has_flag(Flag::CARRY))
            || (cc == ConditionCode::Z && self.registers.has_flag(Flag::ZERO))
            || (cc == ConditionCode::NZ && !self.registers.has_flag(Flag::ZERO))
        {
            let value_lo = self.mmu.read(self.registers.SP);
            self.registers.SP = self.registers.SP.wrapping_add(1);
            self.tick_dot(4);
            let value_hi = self.mmu.read(self.registers.SP);
            self.tick_dot(4);
            self.registers.SP = self.registers.SP.wrapping_add(1);
            self.registers.PC = (value_hi as u16) << 8 | value_lo as u16;
            self.tick_dot(4);
            5
        } else {
            2
        }
    }

    fn instr_POP_r16<const register: Reg>(&mut self) -> u32 {
        let value_lo = self.mmu.read(self.registers.SP);
        self.registers.SP = self.registers.SP.wrapping_add(1);
        self.tick_dot(4);
        let value_hi = self.mmu.read(self.registers.SP);
        self.registers.SP = self.registers.SP.wrapping_add(1);
        self.tick_dot(4);
        let value = (value_hi as u16) << 8 | value_lo as u16;

        match register {
            Reg::AF => self.registers.set_AF(value & 0xFFF0),
            Reg::BC => self.registers.set_BC(value),
            Reg::DE => self.registers.set_DE(value),
            Reg::HL => self.registers.set_HL(value),
            Reg::SP => self.registers.SP = value,
            _ => {
                panic!("instr_POP_r16 received invalid register: {:?}", register)
            }
        }

        3
    }

    fn instr_PUSH_r16<const register: Reg>(&mut self) -> u32 {
        let value = self.get_r16::<register>();
        self.registers.SP = self.registers.SP.wrapping_sub(1);
        self.tick_dot(4);
        self.mmu.write(self.registers.SP, (value >> 8) as u8);
        self.registers.SP = self.registers.SP.wrapping_sub(1);
        self.tick_dot(4);
        self.mmu.write(self.registers.SP, value as u8);
        self.tick_dot(4);
        4
    }

    fn instr_JP_cc_a16<const cc: ConditionCode>(&mut self) -> u32 {
        let address_lo = self.fetch_byte();
        self.tick_dot(4);
        let address_hi = self.fetch_byte();
        self.tick_dot(4);
        let address = ((address_hi as u16) << 8) | address_lo as u16;
        if (cc == ConditionCode::C && self.registers.has_flag(Flag::CARRY))
            || (cc == ConditionCode::NC && !self.registers.has_flag(Flag::CARRY))
            || (cc == ConditionCode::Z && self.registers.has_flag(Flag::ZERO))
            || (cc == ConditionCode::NZ && !self.registers.has_flag(Flag::ZERO))
        {
            self.tick_dot(4);
            self.registers.PC = address;
            4
        } else {
            3
        }
    }

    fn instr_JP_a16(&mut self) -> u32 {
        let address_lo = self.fetch_byte();
        self.tick_dot(4);
        let address_hi = self.fetch_byte();
        self.tick_dot(4);
        let address = (address_hi as u16) << 8 | address_lo as u16;
        self.registers.PC = address;
        self.tick_dot(4);
        4
    }

    fn instr_JP_HL(&mut self) -> u32 {
        self.registers.PC = self.registers.HL();
        1
    }

    fn instr_CALL_a16(&mut self) -> u32 {
        let address_lo = self.fetch_byte();
        self.tick_dot(4);
        let address_hi = self.fetch_byte();
        self.tick_dot(4);
        let address = (address_hi as u16) << 8 | address_lo as u16;

        self.tick_dot(4);

        let value = self.registers.PC;
        self.registers.SP = self.registers.SP.wrapping_sub(1);
        self.mmu.write(self.registers.SP, (value >> 8) as u8);
        self.tick_dot(4);
        self.registers.SP = self.registers.SP.wrapping_sub(1);
        self.mmu.write(self.registers.SP, value as u8);
        self.tick_dot(4);

        self.registers.PC = address;
        6
    }

    fn instr_CALL_cc_a16<const cc: ConditionCode>(&mut self) -> u32 {
        let address_lo = self.fetch_byte();
        self.tick_dot(4);
        let address_hi = self.fetch_byte();
        self.tick_dot(4);
        let address = (address_hi as u16) << 8 | address_lo as u16;
        if (cc == ConditionCode::C && self.registers.has_flag(Flag::CARRY))
            || (cc == ConditionCode::NC && !self.registers.has_flag(Flag::CARRY))
            || (cc == ConditionCode::Z && self.registers.has_flag(Flag::ZERO))
            || (cc == ConditionCode::NZ && !self.registers.has_flag(Flag::ZERO))
        {
            let value = self.registers.PC;
            self.registers.SP = self.registers.SP.wrapping_sub(1);
            self.tick_dot(4);
            self.mmu.write(self.registers.SP, (value >> 8) as u8);
            self.registers.SP = self.registers.SP.wrapping_sub(1);
            self.tick_dot(4);
            self.mmu.write(self.registers.SP, value as u8);
            self.tick_dot(4);
            self.registers.PC = address;
            6
        } else {
            3
        }
    }

    fn instr_RST(&mut self, vec: u16) -> u32 {
        self.tick_dot(4);
        let value = self.registers.PC;
        self.registers.SP = self.registers.SP.wrapping_sub(1);
        self.mmu.write(self.registers.SP, (value >> 8) as u8);
        self.tick_dot(4);
        self.registers.SP = self.registers.SP.wrapping_sub(1);
        self.mmu.write(self.registers.SP, value as u8);
        self.tick_dot(4);

        self.registers.PC = vec;

        4
    }

    fn instr_RET(&mut self) -> u32 {
        let value_lo = self.mmu.read(self.registers.SP);
        self.tick_dot(4);
        self.registers.SP = self.registers.SP.wrapping_add(1);
        let value_hi = self.mmu.read(self.registers.SP);
        self.tick_dot(4);
        self.registers.SP = self.registers.SP.wrapping_add(1);
        self.tick_dot(4);
        self.registers.PC = (value_hi as u16) << 8 | value_lo as u16;
        4
    }

    fn instr_RETI(&mut self) -> u32 {
        let value_lo = self.mmu.read(self.registers.SP);
        self.tick_dot(4);
        self.registers.SP = self.registers.SP.wrapping_add(1);
        let value_hi = self.mmu.read(self.registers.SP);
        self.tick_dot(4);
        self.registers.SP = self.registers.SP.wrapping_add(1);
        self.tick_dot(4);
        self.registers.PC = (value_hi as u16) << 8 | value_lo as u16;
        self.registers.IME = true;
        4
    }

    fn instr_LDH_A_n16(&mut self) -> u32 {
        let address = 0xFF00 + self.fetch_byte() as u16;
        self.tick_dot(4);
        self.registers.A = self.mmu.read(address);
        self.tick_dot(4);
        3
    }

    fn instr_LDH_A_C(&mut self) -> u32 {
        let address = 0xFF00 + self.registers.C as u16;
        self.registers.A = self.mmu.read(address);
        self.tick_dot(4);
        2
    }

    fn instr_LDH_n16_A(&mut self) -> u32 {
        let address = 0xFF00 + self.fetch_byte() as u16;
        self.tick_dot(4);
        self.mmu.write(address, self.registers.A);
        self.tick_dot(4);
        3
    }

    fn instr_LDH_C_A(&mut self) -> u32 {
        let address = 0xFF00 + self.registers.C as u16;
        self.mmu.write(address, self.registers.A);
        self.tick_dot(4);
        2
    }

    fn instr_DI(&mut self) -> u32 {
        self.registers.IME = false;
        self.to_set_IME = 0;
        1
    }

    fn instr_EI(&mut self) -> u32 {
        if self.to_set_IME == 0 {
            self.to_set_IME = 1;
        }
        1
    }

    // no clue: https://github.com/alexcrichton/jba/blob/rust/src/cpu/z80/imp.rs#L81
    fn instr_ADD_SP_e8(&mut self) -> u32 {
        let value = self.fetch_byte() as i8 as i16 as u16;
        self.tick_dot(4);
        let res = self.registers.SP.wrapping_add(value);
        let tmp = value ^ res ^ self.registers.SP;
        self.tick_dot(4);
        self.tick_dot(4);

        self.registers.SP = res;

        self.registers.set_flag(Flag::CARRY, tmp & 0x100 != 0);
        self.registers.set_flag(Flag::HALF_CARRY, tmp & 0x010 != 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::ZERO, false);

        4
    }

    // no clue: https://github.com/alexcrichton/jba/blob/rust/src/cpu/z80/imp.rs#L65
    fn instr_LD_HL_SP_e8(&mut self) -> u32 {
        let value = self.fetch_byte() as i8 as i16 as u16;
        self.tick_dot(4);
        let res = self.registers.SP.wrapping_add(value);
        let tmp = value ^ res ^ self.registers.SP;

        self.tick_dot(4);

        self.registers.set_HL(res);

        self.registers.set_flag(Flag::CARRY, tmp & 0x100 != 0);
        self.registers.set_flag(Flag::HALF_CARRY, tmp & 0x010 != 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::ZERO, false);
        3
    }

    fn instr_LD_SP_HL(&mut self) -> u32 {
        self.tick_dot(4);
        self.registers.SP = self.registers.HL();
        2
    }

    fn instr_LD_n16_A(&mut self) -> u32 {
        let address_lo = self.fetch_byte();
        self.tick_dot(4);
        let address_hi = self.fetch_byte();
        self.tick_dot(4);
        let address = (address_hi as u16) << 8 | address_lo as u16;
        self.mmu.write(address, self.registers.A);
        self.tick_dot(4);
        4
    }

    fn instr_LD_A_n16(&mut self) -> u32 {
        let address_lo = self.fetch_byte();
        self.tick_dot(4);
        let address_hi = self.fetch_byte();
        self.tick_dot(4);
        let address = (address_hi as u16) << 8 | address_lo as u16;
        self.registers.A = self.mmu.read(address);
        self.tick_dot(4);
        4
    }

    // CB instructions
    fn instr_RLC_r8<const register: Reg>(&mut self) -> u32 {
        let mut value = self.get_r8::<register>();

        value = value.rotate_left(1);
        self.registers.set_flag(Flag::CARRY, (value & 0x1) == 0x1);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        self.registers.set_flag(Flag::SUBTRACTION, false);

        self.set_r8::<register>(value);
        2
    }

    fn instr_RLC_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let mut value = self.mmu.read(address);
        self.tick_dot(4);

        value = value.rotate_left(1);
        self.registers.set_flag(Flag::CARRY, (value & 0x1) == 0x1);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::HALF_CARRY, false);
        self.registers.set_flag(Flag::SUBTRACTION, false);

        self.mmu.write(address, value);
        self.tick_dot(4);
        4
    }

    fn instr_RRC_r8<const register: Reg>(&mut self) -> u32 {
        let mut value = self.get_r8::<register>();

        value = value.rotate_right(1);
        self.registers.set_flag(Flag::CARRY, value & 0x80 == 0x80);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);

        self.set_r8::<register>(value);
        2
    }

    fn instr_RRC_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let mut value = self.mmu.read(address);
        self.tick_dot(4);

        value = value.rotate_right(1);
        self.registers.set_flag(Flag::CARRY, value & 0x80 == 0x80);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);

        self.mmu.write(address, value);
        self.tick_dot(4);
        4
    }

    fn instr_RL_r8<const register: Reg>(&mut self) -> u32 {
        let mut value = self.get_r8::<register>();

        let new_carry = value & 0x80 == 0x80;
        value = (value << 1) | u8::from(self.registers.has_flag(Flag::CARRY));

        self.registers.set_flag(Flag::CARRY, new_carry);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);

        self.set_r8::<register>(value);
        2
    }

    fn instr_RL_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let mut value = self.mmu.read(address);
        self.tick_dot(4);

        let new_carry = value & 0x80 == 0x80;
        value = (value << 1) | u8::from(self.registers.has_flag(Flag::CARRY));

        self.registers.set_flag(Flag::CARRY, new_carry);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);

        self.mmu.write(address, value);
        self.tick_dot(4);
        4
    }

    fn instr_RR_r8<const register: Reg>(&mut self) -> u32 {
        let mut value = self.get_r8::<register>();

        let new_carry = value & 0x01 == 0x01;
        value = (value >> 1) | (u8::from(self.registers.has_flag(Flag::CARRY)) << 7);

        self.registers.set_flag(Flag::CARRY, new_carry);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);

        self.set_r8::<register>(value);
        2
    }

    fn instr_RR_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let mut value = self.mmu.read(address);
        self.tick_dot(4);

        let new_carry = value & 0x01 == 0x01;
        value = (value >> 1) | (u8::from(self.registers.has_flag(Flag::CARRY)) << 7);

        self.registers.set_flag(Flag::CARRY, new_carry);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);

        self.mmu.write(address, value);
        self.tick_dot(4);
        4
    }

    fn instr_SLA_r8<const register: Reg>(&mut self) -> u32 {
        let mut value = self.get_r8::<register>();

        let new_carry = value & 0x80 == 0x80;
        value <<= 1;

        self.registers.set_flag(Flag::CARRY, new_carry);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);

        self.set_r8::<register>(value);
        2
    }

    fn instr_SLA_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let mut value = self.mmu.read(address);
        self.tick_dot(4);

        let new_carry = value & 0x80 == 0x80;
        value <<= 1;

        self.registers.set_flag(Flag::CARRY, new_carry);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);

        self.mmu.write(address, value);
        self.tick_dot(4);
        4
    }

    fn instr_SRA_r8<const register: Reg>(&mut self) -> u32 {
        let mut value = self.get_r8::<register>();

        let sign = value & 0x80 == 0x80;
        let new_carry = value & 0x01 == 0x01;
        value >>= 1;
        if sign {
            value |= 0x80;
        } else {
            value &= 0x7F;
        }

        self.registers.set_flag(Flag::CARRY, new_carry);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);

        self.set_r8::<register>(value);
        2
    }

    fn instr_SRA_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let mut value = self.mmu.read(address);
        self.tick_dot(4);

        let sign = value & 0x80 == 0x80;
        let new_carry = value & 0x01 == 0x01;
        value >>= 1;
        if sign {
            value |= 0x80;
        } else {
            value &= 0x7F;
        }

        self.registers.set_flag(Flag::CARRY, new_carry);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);

        self.mmu.write(address, value);
        self.tick_dot(4);
        4
    }

    fn instr_SWAP_r8<const register: Reg>(&mut self) -> u32 {
        let mut value = self.get_r8::<register>();

        let value = value.rotate_right(4);

        self.registers.set_flag(Flag::CARRY, false);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);

        self.set_r8::<register>(value);
        2
    }

    fn instr_SWAP_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let mut value = self.mmu.read(address);
        self.tick_dot(4);

        value = value.rotate_right(4);

        self.registers.set_flag(Flag::CARRY, false);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);

        self.mmu.write(address, value);
        self.tick_dot(4);
        4
    }

    fn instr_SRL_r8<const register: Reg>(&mut self) -> u32 {
        let mut value = self.get_r8::<register>();

        let new_carry = value & 0x01 == 0x01;
        value >>= 1;

        self.registers.set_flag(Flag::CARRY, new_carry);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);

        self.set_r8::<register>(value);
        2
    }

    fn instr_SRL_HL(&mut self) -> u32 {
        let address = self.registers.HL();
        let mut value = self.mmu.read(address);
        self.tick_dot(4);

        let new_carry = value & 0x01 == 0x01;
        value >>= 1;

        self.registers.set_flag(Flag::CARRY, new_carry);
        self.registers.set_flag(Flag::ZERO, value == 0);
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, false);

        self.mmu.write(address, value);
        self.tick_dot(4);
        4
    }

    fn instr_BIT_u3_r8<const bit: u8, const register: Reg>(&mut self) -> u32 {
        let value = self.get_r8::<register>();

        self.registers
            .set_flag(Flag::ZERO, value & (0x1 << bit) != (0x1 << bit));
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, true);
        2
    }

    fn instr_BIT_u3_HL<const bit: u8>(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.mmu.read(address);
        self.tick_dot(4);

        self.registers
            .set_flag(Flag::ZERO, value & (0x1 << bit) != (0x1 << bit));
        self.registers.set_flag(Flag::SUBTRACTION, false);
        self.registers.set_flag(Flag::HALF_CARRY, true);
        3
    }

    fn instr_RES_u3_r8<const bit: u8, const register: Reg>(&mut self) -> u32 {
        let mut value = self.get_r8::<register>();
        value &= !(0x1 << bit);
        self.set_r8::<register>(value);
        2
    }

    fn instr_RES_u3_HL<const bit: u8>(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.mmu.read(address) & !(0x1 << bit);
        self.tick_dot(4);
        self.mmu.write(address, value);
        self.tick_dot(4);
        4
    }

    fn instr_SET_u3_r8<const bit: u8, const register: Reg>(&mut self) -> u32 {
        let mut value = self.get_r8::<register>();
        value |= 0x1 << bit;
        self.set_r8::<register>(value);
        2
    }

    fn instr_SET_u3_HL<const bit: u8>(&mut self) -> u32 {
        let address = self.registers.HL();
        let value = self.mmu.read(address) | (0x1 << bit);
        self.tick_dot(4);
        self.mmu.write(address, value);
        self.tick_dot(4);
        4
    }
}
