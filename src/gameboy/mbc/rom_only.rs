use crate::gameboy::mbc::MBC;

pub struct ROMOnly {
    rom: [u8; 32768],
    ram: [u8; 8192],
    has_ram: bool,
    has_battery: bool,
}

impl MBC for ROMOnly {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => self.rom[address as usize],
            0xA000..=0xBFFF => {
                if self.has_ram {
                    self.ram[(address - 0xA000) as usize]
                } else {
                    panic!("Tried to read cartridge ROM which does not exist!")
                }
            }
            _ => {
                panic!("Tried to read from cartridge with invalid address!")
            }
        }
    }

    fn write(&self, address: u16, value: u8) {
        todo!()
    }
}

impl ROMOnly {
    pub fn new(rom: &[u8], has_ram: bool, has_battery: bool) -> Self {
        ROMOnly {
            rom: *rom.first_chunk::<32768>().unwrap(),
            ram: [0; 8192],
            has_battery,
            has_ram,
        }
    }
}
