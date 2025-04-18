use crate::audio::AudioPlayer;
use crate::gb::breakpoints::Breakpoints;
use crate::gb::cpu::CPU;
use crate::gb::mmu::MMU;
use crate::gb::registers::Registers;
use crate::ui::Memories;
use intbits::Bits;
use winit::keyboard::{KeyCode, PhysicalKey};

mod apu;
pub(crate) mod breakpoints;
pub mod cpu;
pub(crate) mod disassembler;
mod io_registers;
mod mbc;
pub mod mmu;
mod ppu;
pub mod registers;
pub mod renderer;

pub struct GameBoy {
    pub(crate) cpu: CPU,
}

impl GameBoy {
    pub fn new() -> Self {
        let registers = Registers::new();
        let audio_player = AudioPlayer::new();
        let mmu = MMU::new(audio_player);
        let cpu = CPU::new(registers, mmu);
        GameBoy { cpu }
    }

    pub fn load_rom(&mut self, rom_name: &str) {
        self.cpu.mmu.load_rom(rom_name);
    }

    pub fn tick(&mut self) -> (bool, u32) {
        let (hit_breakpoint, cycles) = self.cpu.process_instruction();
        (hit_breakpoint, cycles)
    }

    pub(crate) fn set_breakpoints(&mut self, breakpoints: Breakpoints) {
        self.cpu.breakpoints = breakpoints;
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
            Memories::TileData => self.cpu.mmu.ppu.tile_data.to_vec(),
            Memories::BackgroundMaps => {
                let mut mem = self.cpu.mmu.ppu.background_map_1.to_vec();
                mem.extend(self.cpu.mmu.ppu.background_map_2.iter());
                mem
            }
            Memories::OAM => self.cpu.mmu.ppu.object_attribute_memory.to_vec(),
            _ => {
                todo!("Implement remaining memories")
            }
        }
    }

    pub fn skip_boot_rom(&mut self) {
        // Setup registers
        self.cpu.registers.A = 0x01;
        self.cpu.registers.F = 0xB0;
        self.cpu.registers.B = 0x00;
        self.cpu.registers.C = 0x13;
        self.cpu.registers.D = 0x00;
        self.cpu.registers.E = 0xD8;
        self.cpu.registers.H = 0x01;
        self.cpu.registers.L = 0x4D;
        self.cpu.registers.PC = 0x0100;
        self.cpu.registers.SP = 0xFFFE;

        // Setup hardware registers
        self.cpu.mmu.write(0xFF00, 0xCF); // P1
        self.cpu.mmu.write(0xFF02, 0x7E); // SC
        self.cpu.mmu.write(0xFF07, 0xF8); // TAC
        self.cpu.mmu.write(0xFF0F, 0xE1); // IF
        self.cpu.mmu.write(0xFF10, 0x80); // NR10
        self.cpu.mmu.write(0xFF11, 0xBF); // NR11
        self.cpu.mmu.write(0xFF12, 0xF3); // NR12
        self.cpu.mmu.write(0xFF13, 0xFF); // NR13
        self.cpu.mmu.write(0xFF14, 0xBF); // NR14
        self.cpu.mmu.write(0xFF16, 0x3F); // NR21
        self.cpu.mmu.write(0xFF17, 0x00); // NR22
        self.cpu.mmu.write(0xFF18, 0xFF); // NR23
        self.cpu.mmu.write(0xFF19, 0xBF); // NR24
        self.cpu.mmu.write(0xFF1A, 0x7F); // NR30
        self.cpu.mmu.write(0xFF1B, 0xFF); // NR31
        self.cpu.mmu.write(0xFF1C, 0x9F); // NR32
        self.cpu.mmu.write(0xFF1D, 0xFF); // NR33
        self.cpu.mmu.write(0xFF1E, 0xBF); // NR34
        self.cpu.mmu.write(0xFF20, 0xFF); // NR41
        self.cpu.mmu.write(0xFF21, 0x00); // NR42
        self.cpu.mmu.write(0xFF22, 0x00); // NR43
        self.cpu.mmu.write(0xFF23, 0xBF); // NR44
        self.cpu.mmu.write(0xFF24, 0x77); // NR50
        self.cpu.mmu.write(0xFF25, 0xF3); // NR51
        self.cpu.mmu.write(0xFF26, 0xF1); // NR52
        self.cpu.mmu.write(0xFF40, 0x91); // LCDC
        self.cpu.mmu.write(0xFF41, 0x80); // STAT
        self.cpu.mmu.write(0xFF42, 0x00); // SCY
        self.cpu.mmu.write(0xFF43, 0x00); // SCX
        self.cpu.mmu.write(0xFF44, 0x00); // LY
        self.cpu.mmu.write(0xFF45, 0x00); // LYC
        self.cpu.mmu.write(0xFF47, 0xFC); // BGP
        self.cpu.mmu.write(0xFF4A, 0x00); // WY
        self.cpu.mmu.write(0xFF4B, 0x00); // WX
        self.cpu.mmu.write(0xFFFF, 0x00); // IE

        // Setup APU state
        self.cpu.mmu.apu.skip_bootrom();

        // Disable boot rom
        self.cpu.mmu.write(0xFF50, 0x00);
    }

    pub fn serial_buffer(&self) -> Vec<char> {
        self.cpu.mmu.io_registers.serial_buffer().clone()
    }

    pub fn get_framebuffer(&self) -> Vec<u8> {
        self.cpu.mmu.ppu.frame_buffer_vblanked.clone()
    }

    pub fn key_pressed(&mut self, physical_key: PhysicalKey) {
        // TODO: move function to io_registers to allow internals to remain private
        if let PhysicalKey::Code(key_code) = physical_key {
            self.cpu.mmu.io_registers.inputs.insert(key_code, true);
            if self.cpu.mmu.io_registers.FF00_JOYP & 0x10 == 0 {
                let dpad_keys = [
                    KeyCode::ArrowUp,
                    KeyCode::ArrowDown,
                    KeyCode::ArrowLeft,
                    KeyCode::ArrowRight,
                ];
                if dpad_keys.contains(&key_code) {
                    self.cpu
                        .mmu
                        .io_registers
                        .FF0F_IF_interrupt_flag
                        .set_bit(4, true);
                }
            }

            if self.cpu.mmu.io_registers.FF00_JOYP & 0x20 == 0 {
                let dpad_keys = [KeyCode::KeyF, KeyCode::KeyD, KeyCode::KeyS, KeyCode::KeyA];
                if dpad_keys.contains(&key_code) {
                    self.cpu
                        .mmu
                        .io_registers
                        .FF0F_IF_interrupt_flag
                        .set_bit(4, true);
                }
            }
        }
    }

    pub fn key_released(&mut self, physical_key: PhysicalKey) {
        if let PhysicalKey::Code(key_code) = physical_key {
            self.cpu.mmu.io_registers.inputs.insert(key_code, false);
        }
    }
}
