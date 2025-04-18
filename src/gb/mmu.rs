use crate::audio::AudioPlayer;
use crate::gb::apu::APU;
use crate::gb::io_registers::IORegisters;
use crate::gb::mbc::{create_MBC, MBC};
use crate::gb::ppu::PPU;
use intbits::Bits;
use rand::Rng;
use std::fs;

pub struct MMU {
    // 256 bytes: 0x0000 -> 0x00FF
    // Bootstrap is loaded to $00-$FF until boot is completed, after which this is mapped back to
    // the cartridge ROM
    boot_rom: [u8; 256],
    // 8096 bytes: 0xC000 -> 0xDFFF
    // The ram inside the Game Boy
    pub(crate) internal_ram: Vec<u8>,
    // 127 bytes: 0xFF80 -> 0xFFFE
    // Extra ram space often used as a zero-page
    pub(crate) high_ram: [u8; 127],
    // Memory Bank Controller
    // Handles available ROM, RAM, and extras on cartridge
    pub(crate) mbc: Box<dyn MBC>,
    // IO registers
    pub(crate) io_registers: IORegisters,
    // Pixel Processing Unit
    pub(crate) ppu: PPU,
    // Audio Processing Unit
    pub(crate) apu: APU,
    // Direct Memory Access controller
    dot_counter: u16,
    source_address: u16,
    transfer_active: bool,
    reg_FF46_DMA: u8,
}

impl MMU {
    pub fn new(audio_player: AudioPlayer) -> Self {
        let mut rng = rand::rng();
        MMU {
            boot_rom: *include_bytes!("../roms/bootix_dmg.bin"),
            internal_ram: (0..8192).map(|_| rng.random()).collect(),
            high_ram: [0; 127],
            mbc: create_MBC(Vec::new()),
            io_registers: IORegisters::new(),
            ppu: PPU::new(),
            apu: APU::new(audio_player),
            // DMA
            dot_counter: 0,
            source_address: 0xFF00,
            transfer_active: false,
            reg_FF46_DMA: 0xFF,
        }
    }

    pub fn load_rom(&mut self, rom_path: &str) {
        // Load rom and create mbc
        let rom = fs::read(rom_path).expect("Failed to load rom");
        let mbc = create_MBC(rom);
        self.mbc = mbc;
    }

    pub fn tick(&mut self) {
        if self.transfer_active {
            if self.dot_counter >= 4 && self.dot_counter % 4 == 0 {
                // Transfer next byte
                let src = self
                    .source_address
                    .wrapping_sub(1)
                    .wrapping_add(self.dot_counter / 4);
                let dst = 0xFE00 - 1 + self.dot_counter / 4;
                let value = self.read(src);
                self.ppu.write(dst, value);
            }

            if self.dot_counter == 644 {
                self.transfer_active = false;
                self.dot_counter = 0;
            } else {
                self.dot_counter += 1;
            }
        }
    }

    pub fn read(&mut self, mut address: u16) -> u8 {
        // Check if boot rom is enabled
        if self.io_registers.FF50_boot_rom_enabled && address <= 0x00FF {
            return self.boot_rom[address as usize];
        }

        if self.transfer_active
            && self.dot_counter >= 4
            && !((0xFF80..=0xFFFE).contains(&address) || address == 0xFF46)
        {
            if (0xFE..=0xFE).contains(&(address >> 8)) {
                return 0xFF;
            }

            // Potential OAM bus conflict
            if !(0x80..0x9F).contains(&(self.source_address >> 8))
                && !(0x80..0x9F).contains(&(address >> 8))
            {
                // External bus conflict
                address = self
                    .source_address
                    .wrapping_sub(1)
                    .wrapping_add(self.dot_counter / 4);
            } else if ((0x80..0x9F).contains(&(self.source_address >> 8)))
                && ((0x80..0x9F).contains(&(address >> 8)))
            {
                // VRAM bus conflict
                address = self
                    .source_address
                    .wrapping_sub(1)
                    .wrapping_add(self.dot_counter / 4);
            }
        }

        match address {
            0x0000..=0x7FFF => self.mbc.read(address),
            0x8000..=0x9FFF => self.ppu.read(address),
            0xA000..=0xBFFF => self.mbc.read(address),
            0xC000..=0xCFFF => self.internal_ram[(address - 0xC000) as usize],
            0xD000..=0xDFFF => self.internal_ram[(address - 0xC000) as usize],
            0xE000..=0xFDFF => self.internal_ram[(address - 0xE000) as usize],
            0xFE00..=0xFE9F => self.ppu.read(address),
            0xFEA0..=0xFEFF => 0xFF, // Prohibited
            0xFF00..=0xFF0F => self.io_registers.read(address),
            0xFF10..=0xFF3F => self.apu.read(address),
            0xFF40..=0xFF45 => self.ppu.read(address),
            0xFF46 => self.reg_FF46_DMA,
            0xFF47..=0xFF4B => self.ppu.read(address),
            0xFF4C..=0xFF4E => self.io_registers.read(address),
            0xFF4F => self.ppu.read(address),
            0xFF50 => self.io_registers.read(address),
            0xFF51..=0xFF55 => 0xFF,
            0xFF56..=0xFF67 => self.io_registers.read(address),
            0xFF68..=0xFF6C => self.ppu.read(address),
            0xFF6D..=0xFF6F => self.io_registers.read(address),
            0xFF70 => {
                // TODO: WRAM bank select
                0xFF
            }
            0xFF71..=0xFF75 => self.io_registers.read(address),
            0xFF76..=0xFF77 => self.apu.read(address),
            0xFF78..=0xFF7F => self.io_registers.read(address),
            0xFF80..=0xFFFE => self.high_ram[(address - 0xFF80) as usize],
            0xFFFF..=0xFFFF => self.io_registers.read(address),
            _ => {
                panic!("Trying to read outside of MMU memory range")
            }
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        // OAM DMA conflict
        if self.transfer_active
            && self.dot_counter >= 4
            && !((0xFF80..=0xFFFE).contains(&address) || address == 0xFF46)
        {
            return;
        }

        match address {
            0x0000..=0x7FFF => self.mbc.write(address, value),
            0x8000..=0x9FFF => self.ppu.write(address, value),
            0xA000..=0xBFFF => self.mbc.write(address, value),
            0xC000..=0xCFFF => self.internal_ram[(address - 0xC000) as usize] = value,
            0xD000..=0xDFFF => self.internal_ram[(address - 0xC000) as usize] = value,
            0xE000..=0xFDFF => self.internal_ram[(address - 0xE000) as usize] = value,
            0xFE00..=0xFE9F => self.ppu.write(address, value),
            0xFEA0..=0xFEFF => {} // Prohibited
            0xFF00..=0xFF0F => self.io_registers.write(address, value),
            0xFF10..=0xFF3F => self.apu.write(address, value),
            0xFF40..=0xFF45 => self.ppu.write(address, value),
            0xFF46 => {
                if value >= 0xFE {
                    self.source_address = ((value - 0x20) as u16) << 8;
                } else {
                    self.source_address = (value as u16) << 8;
                }
                self.reg_FF46_DMA = value;

                self.transfer_active = true;
            }
            0xFF47..=0xFF4B => self.ppu.write(address, value),
            0xFF4C..=0xFF4E => self.io_registers.write(address, value),
            0xFF4F => self.ppu.write(address, value),
            0xFF50 => self.io_registers.write(address, value),
            0xFF51..=0xFF55 => {}
            0xFF56..=0xFF67 => self.io_registers.write(address, value),
            0xFF68..=0xFF6C => self.ppu.write(address, value),
            0xFF6D..=0xFF6F => self.io_registers.write(address, value),
            0xFF70 => {
                // TODO: WRAM bank select
            }
            0xFF71..=0xFF75 => self.io_registers.write(address, value),
            0xFF76..=0xFF77 => self.apu.write(address, value),
            0xFF78..=0xFF7F => self.io_registers.write(address, value),
            0xFF80..=0xFFFE => self.high_ram[(address - 0xFF80) as usize] = value,
            0xFFFF..=0xFFFF => self.io_registers.write(address, value),
            _ => {
                panic!("Trying to read outside of MMU memory range")
            }
        }
    }

    pub(crate) fn handle_ppu_interrupts(&mut self) {
        if self.ppu.int_vblank {
            self.ppu.int_vblank = false;
            let value = self.io_registers.read(0xFF0F) | 1;
            self.io_registers.write(0xFF0F, value);
        }

        if self.ppu.int_stat {
            self.ppu.int_stat = false;
            let value = self.read(0xFF0F) | 0b10;
            self.write(0xFF0F, value);
        }
    }
}
