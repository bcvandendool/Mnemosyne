use crate::config;
use crate::gb::mbc::MBC;
use intbits::Bits;
use std::fs::File;
use std::io::{Read, Write};

pub(crate) struct MBC3 {
    name: String,
    rom: Vec<u8>,
    rom_size: usize,
    rom_banks: usize,
    has_ram: bool,
    ram: Vec<u8>,
    ram_size: usize,
    ram_banks: usize,
    has_battery: bool,
    has_timer: bool,
    // registers
    reg_ram_enabled: bool,
    reg_rom_bank_number: u8,
    reg_ram_bank_number: u8,
}

impl MBC for MBC3 {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x000..=0x3FFF => self.rom[address.bits(0..14) as usize],
            0x4000..=0x7FFF => {
                let mapped_address =
                    ((self.reg_rom_bank_number as usize) << 14) | (address.bits(0..14) as usize);
                self.rom[mapped_address]
            }
            0xA000..=0xBFFF => {
                if self.has_ram && self.reg_ram_enabled && self.reg_ram_bank_number <= 0x03 {
                    let mapped_address = ((self.reg_ram_bank_number as usize) << 13)
                        | (address.bits(0..13) as usize);
                    self.ram[mapped_address]
                } else if self.has_timer
                    && self.reg_ram_bank_number >= 0x08
                    && self.reg_ram_bank_number <= 0xC
                {
                    // TODO: RTC registers
                    return 0xFF;
                } else {
                    return 0xFF;
                }
            }
            _ => 0xFF,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => {
                // RAM and timer enable
                self.reg_ram_enabled = value.bits(0..4) == 0xA;
                if !self.reg_ram_enabled {
                    self.save_ram();
                }
            }
            0x2000..=0x3FFF => {
                // ROM bank number
                self.reg_rom_bank_number = value.max(1);
            }
            0x4000..=0x5FFF => {
                // RAM bank number / RTC register select
                self.reg_ram_bank_number = value.bits(0..4);
            }
            0x6000..=0x7FFF => {
                // Latch clock data
            }
            0xA000..=0xBFFF => {
                // RAM / RTC register
                if self.has_ram && self.reg_ram_enabled && self.reg_ram_bank_number <= 0x03 {
                    let mapped_address = ((self.reg_ram_bank_number as usize) << 13)
                        | (address.bits(0..13) as usize);
                    self.ram[mapped_address] = value;
                } else if self.has_timer
                    && self.reg_ram_bank_number >= 0x08
                    && self.reg_ram_bank_number <= 0xC
                {
                    // TODO: RTC registers
                }
            }
            _ => {}
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn save_ram(&self) {
        if self.has_ram {
            let file: Option<File> = config::THREAD_LOCAL_CONFIG.with(|c| {
                let mut binding = c.borrow_mut();
                let save_path = &binding.load().gameboy_config.save_path;
                if !save_path.is_empty() {
                    Some(File::create(save_path).expect("Failed to create save file"))
                } else {
                    None
                }
            });

            if let Some(mut file) = file {
                file.write_all(&self.ram).unwrap();
                file.flush().unwrap();
            }
        }
    }

    fn load_ram(&mut self) {
        if self.has_ram {
            let file: Option<File> = config::THREAD_LOCAL_CONFIG.with(|c| {
                let mut binding = c.borrow_mut();
                let save_path = &binding.load().gameboy_config.save_path;
                if !save_path.is_empty() {
                    Some(File::open(save_path).expect("Failed to create save file"))
                } else {
                    None
                }
            });

            if let Some(mut file) = file {
                file.read_exact(&mut self.ram).unwrap();
            }
        }
    }
}

impl MBC3 {
    // TODO: implement timer
    pub(crate) fn new(
        name: String,
        rom: &[u8],
        rom_size: usize,
        has_ram: bool,
        ram_size: usize,
        has_battery: bool,
        has_timer: bool,
    ) -> Self {
        MBC3 {
            name,
            rom: rom.to_vec(),
            rom_size,
            rom_banks: rom_size / 16384,
            ram: vec![0; ram_size],
            ram_size,
            ram_banks: ram_size / 8096,
            has_battery,
            has_timer,
            has_ram,
            reg_ram_enabled: false,
            reg_rom_bank_number: 0x01,
            reg_ram_bank_number: 0x00,
        }
    }
}
