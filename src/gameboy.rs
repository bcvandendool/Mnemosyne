use crate::gameboy::cpu::CPU;
use crate::gameboy::mmu::MMU;
use crate::gameboy::registers::Reg;
use crate::gameboy::registers::Registers;

pub mod cpu;
mod mbc;
pub mod mmu;
pub mod registers;

pub struct GameBoy {
    cpu: CPU,
}

impl GameBoy {
    pub fn new() -> Self {
        let registers = Registers::new();
        let mmu = MMU::new();
        let cpu = CPU::new(registers, mmu);
        GameBoy { cpu }
    }

    pub fn load_rom(&mut self, rom_name: &str) {
        self.cpu.mmu.load_rom(rom_name);
    }

    pub fn tick(&mut self) {
        self.cpu.process_instruction();
    }

    pub fn enable_test_memory(&mut self) {
        self.cpu.mmu.enable_test_memory();
    }

    pub fn set_initial_register(&mut self, register: Reg, value: u64) {
        match register {
            Reg::PC => self.cpu.registers.PC = value as u16,
            Reg::SP => self.cpu.registers.SP = value as u16,
            Reg::A => self.cpu.registers.A = value as u8,
            Reg::B => self.cpu.registers.B = value as u8,
            Reg::C => self.cpu.registers.C = value as u8,
            Reg::D => self.cpu.registers.D = value as u8,
            Reg::E => self.cpu.registers.E = value as u8,
            Reg::F => self.cpu.registers.F = value as u8,
            Reg::H => self.cpu.registers.H = value as u8,
            Reg::L => self.cpu.registers.L = value as u8,
            _ => {
                panic!("Tried to set initial value for invalid register!")
            }
        }
    }

    pub fn set_initial_memory(&mut self, address: u64, value: u64) {
        self.cpu.mmu.write(address as u16, value as u8);
    }

    pub fn get_final_register(&mut self, register: Reg) -> u64 {
        match register {
            Reg::PC => self.cpu.registers.PC as u64,
            Reg::SP => self.cpu.registers.SP as u64,
            Reg::A => self.cpu.registers.A as u64,
            Reg::B => self.cpu.registers.B as u64,
            Reg::C => self.cpu.registers.C as u64,
            Reg::D => self.cpu.registers.D as u64,
            Reg::E => self.cpu.registers.E as u64,
            Reg::F => self.cpu.registers.F as u64,
            Reg::H => self.cpu.registers.H as u64,
            Reg::L => self.cpu.registers.L as u64,
            _ => {
                panic!("Tried to set initial value for invalid register!")
            }
        }
    }

    pub fn get_final_memory(&mut self, address: u64) -> u64 {
        self.cpu.mmu.read(address as u16) as u64
    }
}
