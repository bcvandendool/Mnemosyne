use crate::gameboy::mbc::MBC;

pub struct ROMOnly {
    name: String,
    rom: Vec<u8>,
    ram: Vec<u8>,
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
                    panic!("Tried to read cartridge RAM which does not exist for this cartridge!")
                }
            }
            _ => {
                panic!("Tried to read from cartridge with invalid address!")
            }
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0xA000..=0xBFFF => {
                if self.has_ram {
                    self.ram[(address - 0xA000) as usize] = value;
                } else {
                    panic!("Tried to read cartridge RAM which does not exist for this cartridge!")
                }
            }
            _ => {
                println!(
                    "Tried to write to cartridge with invalid address: {}",
                    address
                )
            }
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

impl ROMOnly {
    pub fn new(
        name: String,
        rom: &[u8],
        has_ram: bool,
        ram_size: usize,
        has_battery: bool,
    ) -> Self {
        ROMOnly {
            name,
            rom: rom.to_vec(),
            ram: vec![0; ram_size],
            has_ram,
            has_battery,
        }
    }
}
