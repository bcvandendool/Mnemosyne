use crate::gameboy::apu::APU;
use crate::gameboy::io_registers::IORegisters;
use crate::gameboy::mbc::{create_MBC, MBC};
use crate::gameboy::ppu::{PPUMode, PPU};
use std::fs;

pub struct MMU {
    // 256 bytes: 0x0000 -> 0x00FF
    // Bootstrap is loaded to $00-$FF until boot is completed, after which this is mapped back to
    // the cartridge ROM
    boot_rom: [u8; 256],
    // 8096 bytes: 0xC000 -> 0xDFFF
    // The ram inside the Game Boy
    pub(crate) internal_ram: [u8; 8192],
    // 127 bytes: 0xFF80 -> 0xFFFE
    // Extra ram space often used as a zero-page
    pub(crate) high_ram: [u8; 127],
    // Memory Bank Controller
    // Handles available ROM, RAM, and extras on cartridge
    mbc: Box<dyn MBC>,
    // Test ram
    test_ram: Option<[u8; 65536]>,
    test_ram_enabled: bool,
    // IO registers
    pub(crate) io_registers: IORegisters,
    // Pixel Processing Unit
    pub(crate) ppu: PPU,
    // Audio Processing Unit
    apu: APU,
    // Direct Memory Access controller
    dot_counter: u16,
    source_address: u16,
    transfer_active: bool,
}

impl MMU {
    pub fn new() -> Self {
        MMU {
            boot_rom: *include_bytes!("../roms/bootix_dmg.bin"),
            internal_ram: [0; 8192],
            high_ram: [0; 127],
            mbc: create_MBC(Vec::new()),
            test_ram: None,
            test_ram_enabled: false,
            io_registers: IORegisters::new(),
            ppu: PPU::new(),
            apu: APU::new(),
            // DMA
            dot_counter: 0,
            source_address: 0x0000,
            transfer_active: false,
        }
    }

    pub fn load_rom(&mut self, rom_name: &str) {
        // Load rom and create mbc
        let rom = fs::read("./src/roms/".to_owned() + rom_name).expect("Failed to load rom");
        let mbc = create_MBC(rom);
        self.mbc = mbc;
    }

    pub fn tick(&mut self, cycles: u32) {
        for _ in 0..cycles {
            if self.transfer_active {
                if self.dot_counter % 4 == 0 {
                    // Transfer next byte
                    let src = self.source_address + self.dot_counter / 4;
                    let dst = 0xFE00 + self.dot_counter / 4;
                    let value = self.read(src);
                    self.write(dst, value);
                }

                if self.dot_counter == 640 {
                    self.transfer_active = false;
                    self.dot_counter = 0;
                } else {
                    self.dot_counter += 1;
                }
            }
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {
        if self.test_ram_enabled {
            return self.test_ram.unwrap()[address as usize];
        }

        // Check if boot rom is enabled
        if self.io_registers.FF50_boot_rom_enabled && address <= 0x00FF {
            return self.boot_rom[address as usize];
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
            0xFF46 => (self.source_address >> 8) as u8,
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
        if self.test_ram_enabled {
            return self.test_ram.as_mut().unwrap()[address as usize] = value;
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
                self.source_address = (value as u16) << 8;
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

    pub fn enable_test_memory(&mut self) {
        self.test_ram = Some([0; 65536]);
        self.test_ram_enabled = true;
    }

    pub(crate) fn handle_ppu_interrupts(&mut self) {
        if self.ppu.int_vblank {
            self.ppu.int_vblank = false;
            let value = self.io_registers.read(0xFF0F) | 1;
            self.io_registers.write(0xFF0F, value);
        }

        if self.ppu.reg_LY == self.ppu.reg_LYC && !self.ppu.lyc_ly_handled {
            // Set LYC == LY bit of reg_STAT
            self.ppu.reg_STAT |= 0b100;

            if self.ppu.reg_STAT & 0b1000000 > 0 {
                let value = self.read(0xFF0F) | 0b10;
                self.write(0xFF0F, value);
            }

            self.ppu.lyc_ly_handled = true;
        } else if self.ppu.reg_LY != self.ppu.reg_LYC && self.ppu.lyc_ly_handled {
            self.ppu.lyc_ly_handled = false;
        }

        if self.ppu.mode_transitioned {
            if self.ppu.ppu_mode == PPUMode::HorizontalBlank {
                self.ppu.reg_STAT &= !0b11;
                if self.ppu.reg_STAT & 0b1000 > 0 {
                    let value = self.read(0xFF0F) | 0b10;
                    self.write(0xFF0F, value);
                }
            } else if self.ppu.ppu_mode == PPUMode::VerticalBlank {
                self.ppu.reg_STAT &= !0b11;
                self.ppu.reg_STAT |= 0b01;
                if self.ppu.reg_STAT & 0b10000 > 0 {
                    let value = self.read(0xFF0F) | 0b10;
                    self.write(0xFF0F, value);
                }
            } else if self.ppu.ppu_mode == PPUMode::OAMScan {
                self.ppu.reg_STAT &= !0b11;
                self.ppu.reg_STAT |= 0b10;
                if self.ppu.reg_STAT & 0b100000 > 0 {
                    let value = self.read(0xFF0F) | 0b10;
                    self.write(0xFF0F, value);
                }
            } else {
                self.ppu.reg_STAT |= 0b11;
            }

            self.ppu.mode_transitioned = false;
        }
    }
}
