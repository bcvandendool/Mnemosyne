use crate::gameboy::mbc::MBC;

pub struct MBC1 {
    rom: Vec<u8>,
    rom_size: usize,
    rom_banks: usize,
    ram: Vec<u8>,
    ram_size: usize,
    ram_banks: usize,
    has_ram: bool,
    has_battery: bool,
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
                    ((self.reg_ram_bank_number as usize) << 19) | ((address & 0x3FFF) as usize)
                } else {
                    (address & 0x3FFF) as usize
                };
                mapped_address &= (1 << (self.rom_banks.ilog2() + 14)) - 1;
                self.rom[mapped_address]
            }
            0x4000..=0x7FFF => {
                let mut mapped_address = ((self.reg_ram_bank_number as usize) << 19)
                    | (((self.reg_rom_bank_number & 0x1F) as usize) << 14)
                    | ((address & 0x3FFF) as usize);
                mapped_address &= (1 << (self.rom_banks.ilog2() + 14)) - 1;
                self.rom[mapped_address]
            }
            0xA000..=0xBFFF => {
                if self.has_ram && self.reg_ram_enabled {
                    let mut mapped_address = if self.reg_banking_mode {
                        ((self.reg_ram_bank_number as usize) << 13) | ((address & 0x1FFF) as usize)
                    } else {
                        (address & 0x1FFF) as usize
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
                self.reg_ram_enabled = value & 0x0F == 0x0A;
                // TODO: save on disable
            }
            0x2000..=0x3FFF => {
                // ROM bank number
                self.reg_rom_bank_number = (value & 0x1F).max(1);
            }
            0x4000..=0x5FFF => {
                // RAM bank number - or - upper bits of ROM bank number
                self.reg_ram_bank_number = value & 0x3;
            }
            0x6000..=0x7FFF => {
                // Banking mode select
                self.reg_banking_mode = value & 0x01 == 0x01;
            }
            0xA000..=0xBFFF => {
                if self.has_ram && self.reg_ram_enabled {
                    let mut mapped_address = if self.reg_banking_mode {
                        ((self.reg_ram_bank_number as usize) << 13) | ((address & 0x1FFF) as usize)
                    } else {
                        (address & 0x1FFF) as usize
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
}

impl MBC1 {
    pub(crate) fn new(
        rom: &[u8],
        rom_size: usize,
        has_ram: bool,
        ram_size: usize,
        has_battery: bool,
    ) -> Self {
        // TODO: handle MBC1M cartridges
        MBC1 {
            rom: rom.to_vec(),
            rom_size,
            rom_banks: rom_size / 16384,
            ram: vec![0; ram_size],
            ram_size,
            ram_banks: ram_size / 8096,
            has_ram,
            has_battery,
            reg_ram_enabled: false,
            reg_rom_bank_number: 0x01,
            reg_ram_bank_number: 0x00,
            reg_banking_mode: false,
        }
    }
}
