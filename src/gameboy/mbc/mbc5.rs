use crate::gameboy::mbc::MBC;
use intbits::Bits;

pub(crate) struct MBC5 {
    rom: Vec<u8>,
    rom_size: usize,
    rom_banks: usize,
    has_ram: bool,
    ram: Vec<u8>,
    ram_size: usize,
    ram_banks: usize,
    has_battery: bool,
    has_rumble: bool,
    // registers
    reg_ram_enabled: bool,
    reg_rom_bank_number: u16,
    reg_ram_bank_number: u8,
}

impl MBC for MBC5 {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom[address as usize],
            0x4000..=0x7FFF => {
                let mut mapped_address =
                    ((self.reg_rom_bank_number as usize) << 14) | (address.bits(0..14) as usize);
                mapped_address &= (1 << (self.rom_banks.ilog2() + 14)) - 1;
                self.rom[mapped_address]
            }
            0xA000..=0xBFFF => {
                if self.has_ram && self.reg_ram_enabled {
                    let mapped_address = ((self.reg_ram_bank_number as usize) << 13)
                        | (address.bits(0..13) as usize);
                    self.ram[mapped_address]
                } else {
                    0xFF
                }
            }
            _ => 0xFF,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => {
                // RAM enable
                self.reg_ram_enabled = value.bits(0..4) == 0xA;
            }
            0x2000..=0x2FFF => {
                // 8 LSB ROM bank number
                self.reg_rom_bank_number.set_bits(0..8, value as u16);
            }
            0x3000..=0x3FFF => {
                // bit 9 of ROM bank number
                self.reg_rom_bank_number.set_bit(8, value.bit(0));
            }
            0x4000..=0x5FFF => {
                // RAM bank number
                self.reg_ram_bank_number = value.bits(0..4);
                if self.has_rumble && value.bit(3) {
                    // TODO: rumble
                }
            }
            0xA000..=0xBFFF => {
                if self.has_ram && self.reg_ram_enabled {
                    let mapped_address = ((self.reg_ram_bank_number as usize) << 13)
                        | (address.bits(0..13) as usize);
                    self.ram[mapped_address] = value;
                }
            }
            _ => {}
        }
    }
}

impl MBC5 {
    pub(crate) fn new(
        rom: &[u8],
        rom_size: usize,
        has_ram: bool,
        ram_size: usize,
        has_battery: bool,
        has_rumble: bool,
    ) -> Self {
        MBC5 {
            rom: rom.to_vec(),
            rom_size,
            rom_banks: rom_size / 16384,
            has_ram,
            ram: vec![0; ram_size],
            ram_size,
            ram_banks: ram_size / 8096,
            has_battery,
            has_rumble,
            reg_ram_enabled: false,
            reg_rom_bank_number: 0x01,
            reg_ram_bank_number: 0x00,
        }
    }
}
