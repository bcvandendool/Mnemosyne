use crate::gameboy::io_registers::IORegisters;
use crate::gameboy::mbc::{create_MBC, MBC};
use std::fs;

pub struct MMU {
    // 256 bytes: 0x0000 -> 0x00FF
    // Bootstrap is loaded to $00-$FF until boot is completed, after which this is mapped back to
    // the cartridge ROM
    boot_rom: [u8; 256],
    // 6144 bytes: 0x8000 -> 0x97FF
    // Exclusively for video purposes, holds the 8x8 pixels of 2-bit color tiles
    character_ram: [u8; 6144],
    // 1024 bytes: 0x9800 -> 0x9BFF
    // Background map 1
    background_map_1: [u8; 1024],
    // 1024 bytes: 0x9C00 -> 0x9FFF
    // Background map 2
    background_map_2: [u8; 1024],
    // 8096 bytes: 0xC000 -> 0xDFFF
    // The ram inside the Game Boy
    pub(crate) internal_ram: [u8; 8192],
    // 160 bytes: 0xFE00 -> 0xFE9F
    // The Object Attribute Memory holds the sprites
    object_attribute_memory: [u8; 160],
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
}

impl MMU {
    pub fn new() -> Self {
        MMU {
            boot_rom: *include_bytes!("../roms/bootix_dmg.bin"),
            character_ram: [0; 6144],
            background_map_1: [0; 1024],
            background_map_2: [0; 1024],
            internal_ram: [0; 8192],
            object_attribute_memory: [0; 160],
            high_ram: [0; 127],
            mbc: create_MBC(Vec::new()),
            test_ram: None,
            test_ram_enabled: false,
            io_registers: IORegisters::new(),
        }
    }

    pub fn load_rom(&mut self, rom_name: &str) {
        // Load rom and create mbc
        let rom = fs::read("./src/roms/".to_owned() + rom_name).expect("Failed to load rom");
        let mbc = create_MBC(rom);
        self.mbc = mbc;
    }

    pub fn read(&mut self, address: u16) -> u8 {
        if self.test_ram_enabled {
            return self.test_ram.unwrap()[address as usize];
        }

        // Check if boot rom is enabled
        if self.io_registers.read(0xFF50) == 0x00 && address <= 0x00FF {
            return self.boot_rom[address as usize];
        }

        match address {
            0x0000..=0x7FFF => self.mbc.read(address),
            0x8000..=0x97FF => self.character_ram[(address - 0x8000) as usize],
            0x9800..=0x9BFF => self.background_map_1[(address - 0x9800) as usize],
            0x9C00..=0x9FFF => self.background_map_2[(address - 0x9C00) as usize],
            0xA000..=0xBFFF => self.mbc.read(address),
            0xC000..=0xCFFF => self.internal_ram[(address - 0xC000) as usize],
            0xD000..=0xDFFF => self.internal_ram[(address - 0xC000) as usize],
            0xE000..=0xFDFF => self.internal_ram[(address - 0xC000) as usize],
            0xFE00..=0xFE9F => self.object_attribute_memory[(address - 0xFE00) as usize],
            0xFF00..=0xFF7F => self.io_registers.read(address),
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
            0x8000..=0x97FF => self.character_ram[(address - 0x8000) as usize] = value,
            0x9800..=0x9BFF => self.background_map_1[(address - 0x9800) as usize] = value,
            0x9C00..=0x9FFF => self.background_map_2[(address - 0x9C00) as usize] = value,
            0xA000..=0xBFFF => self.mbc.write(address, value),
            0xC000..=0xCFFF => self.internal_ram[(address - 0xC000) as usize] = value,
            0xD000..=0xDFFF => self.internal_ram[(address - 0xC000) as usize] = value,
            0xE000..=0xFDFF => self.internal_ram[(address - 0xC000) as usize] = value,
            0xFE00..=0xFE9F => self.object_attribute_memory[(address - 0xFE00) as usize] = value,
            0xFF00..=0xFF7F => self.io_registers.write(address, value),
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
}
