use crate::gameboy::mbc::MBC;
use intbits::Bits;

pub(crate) struct MBC2 {
    name: String,
    rom: Vec<u8>,
    rom_size: usize,
    rom_banks: usize,
    ram: Vec<u8>,
    has_battery: bool,
    // registers
    reg_ram_enabled: bool,
    reg_rom_bank_number: u8,
}

impl MBC for MBC2 {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                let mapped_address = address.bits(0..14) as usize;
                self.rom[mapped_address]
            }
            0x4000..=0x7FFF => {
                let mut mapped_address = ((self.reg_rom_bank_number.bits(0..4) as usize) << 14)
                    | (address.bits(0..14) as usize);
                mapped_address &= (1 << (self.rom_banks.ilog2() + 14)) - 1;
                self.rom[mapped_address]
            }
            0xA000..=0xBFFF => {
                if self.reg_ram_enabled {
                    let mapped_address = address.bits(0..9) as usize;
                    self.ram[mapped_address] | 0xF0
                } else {
                    0xFF
                }
            }
            _ => 0xFF,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x3FFF => {
                if address.bit(8) {
                    // Rom bank selection
                    self.reg_rom_bank_number = value.bits(0..4).max(1);
                } else {
                    // Ram enable control
                    self.reg_ram_enabled = value.bits(0..4) == 0xA;
                }
            }
            0xA000..=0xBFFF => {
                // Write to ram
                if self.reg_ram_enabled {
                    let mapped_address = address.bits(0..9) as usize;
                    self.ram[mapped_address] = value.bits(0..4);
                }
            }
            _ => {}
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

impl MBC2 {
    pub(crate) fn new(name: String, rom: &[u8], rom_size: usize, has_battery: bool) -> Self {
        MBC2 {
            name,
            rom: rom.to_vec(),
            rom_size,
            rom_banks: rom_size / 16384,
            ram: vec![0; 512],
            has_battery,
            reg_ram_enabled: false,
            reg_rom_bank_number: 1,
        }
    }
}
