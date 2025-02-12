use crate::gameboy::mbc::{create_MBC, MBC};
use std::fs;

pub struct MMU {
    // 256 bytes: 0x0000 -> 0x00FF
    // Bootstrap is loaded to $00-$FF until boot is completed, after which this is mapped back to
    // the cartridge ROM
    bootstrap: [u8; 256],
    bootstrap_enabled: bool,
    // 6144 bytes: 0x8000 -> 0x97FF
    // Exclusively for video purposes, holds the 8x8 pixels of 2-bit color tiles
    character_ram: [u8; 6144],
    // 1024 bytes: 0x9800 -> 0x9BFF
    // Background map 1
    background_map_1: [u8; 1024],
    // 1024 bytes: 0x9C00 -> 0x9FFF
    // Background map 2
    background_map_2: [u8; 1024],
    // 4096 bytes: 0xC000 -> 0xDFFF
    // The ram inside the Game Boy
    internal_ram: [u8; 4096],
    // 160 bytes: 0xFE00 -> 0xFE9F
    // The Object Attribute Memory holds the sprites
    object_attribute_memory: [u8; 160],
    // 127 bytes: 0xFF80 -> 0xFFFE
    // Extra ram space often used as a zero-page
    high_ram: [u8; 127],
    // Memory Bank Controller
    // Handles available ROM, RAM, and extras on cartridge
    mbc: Box<dyn MBC>,
    // Test ram
    test_ram: Option<[u8; 65536]>,
    test_ram_enabled: bool,
}

impl MMU {
    pub fn new() -> Self {
        MMU {
            bootstrap: *include_bytes!("../roms/bootix_dmg.bin"),
            bootstrap_enabled: false,
            character_ram: [0; 6144],
            background_map_1: [0; 1024],
            background_map_2: [0; 1024],
            internal_ram: [0; 4096],
            object_attribute_memory: [0; 160],
            high_ram: [0; 127],
            mbc: create_MBC(Vec::new()),
            test_ram: None,
            test_ram_enabled: false,
        }
    }

    pub fn load_rom(&mut self, rom_name: &str) {
        // Load rom and create mbc
        let rom = fs::read("./src/roms/".to_owned() + rom_name).expect("Failed to load rom");
        let mbc = create_MBC(rom);
        self.mbc = mbc;
    }

    pub fn read(&self, address: u16) -> u8 {
        if self.test_ram_enabled {
            return self.test_ram.unwrap()[address as usize];
        }

        match address {
            0x0000..=0x7FFF => self.mbc.read(address),
            _ => {
                panic!("Trying to read outside of MMU memory range")
            }
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        if self.test_ram_enabled {
            self.test_ram.as_mut().unwrap()[address as usize] = value;
        }
    }

    pub fn enable_test_memory(&mut self) {
        self.test_ram = Some([0; 65536]);
        self.test_ram_enabled = true;
    }
}
