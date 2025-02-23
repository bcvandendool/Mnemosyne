use crate::gameboy::mbc::MBC;
use intbits::Bits;
use log::{log, Level};

pub struct MBC1 {
    name: String,
    rom: Vec<u8>,
    rom_size: usize,
    rom_banks: usize,
    ram: Vec<u8>,
    ram_size: usize,
    ram_banks: usize,
    has_ram: bool,
    has_battery: bool,
    is_MBC1M: bool,
    // registers
    reg_ram_enabled: bool,
    reg_rom_bank_number: u8,
    reg_ram_bank_number: u8,
    reg_banking_mode: bool,
}

impl MBC for MBC1 {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                let mut mapped_address = if self.reg_banking_mode {
                    if self.is_MBC1M {
                        ((self.reg_ram_bank_number.bits(0..2) as usize) << 18)
                            | (address.bits(0..14) as usize)
                    } else {
                        ((self.reg_ram_bank_number as usize) << 19) | (address.bits(0..14) as usize)
                    }
                } else {
                    address.bits(0..14) as usize
                };
                mapped_address &= (1 << (self.rom_banks.ilog2() + 14)) - 1;
                self.rom[mapped_address]
            }
            0x4000..=0x7FFF => {
                let mut mapped_address = if self.is_MBC1M {
                    ((self.reg_ram_bank_number.bits(0..2) as usize) << 18)
                        | ((self.reg_rom_bank_number.bits(0..4) as usize) << 14)
                        | (address.bits(0..14) as usize)
                } else {
                    ((self.reg_ram_bank_number as usize) << 19)
                        | ((self.reg_rom_bank_number.bits(0..5) as usize) << 14)
                        | (address.bits(0..14) as usize)
                };
                mapped_address &= (1 << (self.rom_banks.ilog2() + 14)) - 1;
                self.rom[mapped_address]
            }
            0xA000..=0xBFFF => {
                if self.has_ram && self.reg_ram_enabled {
                    let mut mapped_address = if self.reg_banking_mode {
                        ((self.reg_ram_bank_number as usize) << 13) | (address.bits(0..13) as usize)
                    } else {
                        address.bits(0..13) as usize
                    };
                    mapped_address &= (1 << (self.ram_banks.ilog2() + 13)) - 1;
                    self.ram[mapped_address]
                } else {
                    0xFF
                }
            }
            _ => {
                panic!("Tried to read from cartridge with invalid address!")
            }
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => {
                // RAM enable
                self.reg_ram_enabled = value.bits(0..4) == 0x0A;
                // TODO: save on disable
            }
            0x2000..=0x3FFF => {
                // ROM bank number
                self.reg_rom_bank_number = value.bits(0..5).max(1);
            }
            0x4000..=0x5FFF => {
                // RAM bank number - or - upper bits of ROM bank number
                self.reg_ram_bank_number = value.bits(0..2);
            }
            0x6000..=0x7FFF => {
                // Banking mode select
                self.reg_banking_mode = value.bit(0);
            }
            0xA000..=0xBFFF => {
                if self.has_ram && self.reg_ram_enabled {
                    let mut mapped_address = if self.reg_banking_mode {
                        ((self.reg_ram_bank_number as usize) << 13) | (address.bits(0..13) as usize)
                    } else {
                        address.bits(0..13) as usize
                    };
                    mapped_address &= (1 << (self.ram_banks.ilog2() + 13)) - 1;
                    self.ram[mapped_address] = value;
                }
            }
            _ => {
                panic!(
                    "Tried to write to cartridge with invalid address: {:#06X}",
                    address
                )
            }
        }
    }
    fn name(&self) -> String {
        self.name.clone()
    }
}

impl MBC1 {
    pub(crate) fn new(
        name: String,
        rom: &[u8],
        rom_size: usize,
        has_ram: bool,
        ram_size: usize,
        has_battery: bool,
    ) -> Self {
        // Check if MBC1M type cartridge
        let nintendo_logo = &rom[0x0104..=0x0133];
        let mapped_address = (0x10 << 14) | (0x0104.bits(0..14) as usize);
        if rom.len() > mapped_address + 0x2F {
            let nintendo_logo_check = &rom[mapped_address..=mapped_address + 0x2F];
            if nintendo_logo == nintendo_logo_check {
                log!(Level::Info, "MBC1M type cartridge");
                return MBC1 {
                    name,
                    rom: rom.to_vec(),
                    rom_size,
                    rom_banks: rom_size / 16384,
                    ram: vec![0; ram_size],
                    ram_size,
                    ram_banks: ram_size / 8096,
                    has_ram,
                    has_battery,
                    is_MBC1M: true,
                    reg_ram_enabled: false,
                    reg_rom_bank_number: 0x01,
                    reg_ram_bank_number: 0x00,
                    reg_banking_mode: false,
                };
            }
        }

        // Otherwise a normal MBC1 cartridge
        MBC1 {
            name,
            rom: rom.to_vec(),
            rom_size,
            rom_banks: rom_size / 16384,
            ram: vec![0; ram_size],
            ram_size,
            ram_banks: ram_size / 8096,
            has_ram,
            has_battery,
            is_MBC1M: false,
            reg_ram_enabled: false,
            reg_rom_bank_number: 0x01,
            reg_ram_bank_number: 0x00,
            reg_banking_mode: false,
        }
    }
}
