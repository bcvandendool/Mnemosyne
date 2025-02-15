use crate::gameboy::cpu::CPU;
use crate::gameboy::mmu::MMU;
use crate::gameboy::registers::Reg;
use crate::gameboy::registers::Registers;
use crate::ui::Memories;

pub mod cpu;
pub(crate) mod disassembler;
mod io_registers;
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
        let cycles = self.cpu.process_instruction();
        self.cpu.mmu.io_registers.update_timers(cycles);
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

    pub fn dump_registers(&mut self) -> Registers {
        self.cpu.registers.clone()
    }

    pub(crate) fn dump_ram(&mut self, memory_to_dump: Memories) -> Vec<u8> {
        match memory_to_dump {
            Memories::WRAM1 => self.cpu.mmu.internal_ram[0..4096].to_vec(),
            Memories::WRAM2 => self.cpu.mmu.internal_ram[4096..].to_vec(),
            Memories::HRAM => {
                let mut mem = self.cpu.mmu.high_ram.to_vec();
                mem.push(self.cpu.mmu.read(0xFFFF));
                mem
            }
            _ => {
                todo!("Implement remaining memories")
            }
        }
    }

    pub fn skip_boot_rom(&mut self) {
        self.cpu.registers.A = 0x01;
        self.cpu.registers.F = 0x00;
        self.cpu.registers.B = 0xFF;
        self.cpu.registers.C = 0x13;
        self.cpu.registers.D = 0x00;
        self.cpu.registers.E = 0xC1;
        self.cpu.registers.H = 0x84;
        self.cpu.registers.L = 0x03;
        self.cpu.registers.PC = 0x0100;
        self.cpu.registers.SP = 0xFFFE;
        // Disabled boot rom
        self.cpu.mmu.write(0xFF50, 0x01);
    }

    pub fn serial_buffer(&self) -> Vec<char> {
        self.cpu.mmu.io_registers.serial_buffer().clone()
    }
}
