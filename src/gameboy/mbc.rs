use crate::gameboy::mbc::mbc1::MBC1;
use crate::gameboy::mbc::null::NullMBC;
use crate::gameboy::mbc::rom_only::ROMOnly;

mod mbc1;
mod null;
mod rom_only;

pub struct CartridgeHeader {
    title: String,
    manufacturer_code: [u8; 4],
    cgb_flag: CGBFlag,
    new_licensee_code: u16,
    sgb_flag: SGBFlag,
    cartridge_type: u8,
    rom_size: usize,
    ram_size: usize,
    destination_code: DestinationCode,
    old_licensee_code: u8,
    version_number: u8,
    header_checksum: u8,
    global_checksum: u16,
}

enum CGBFlag {
    DMGCompatible,
    CGBOnly,
}

enum SGBFlag {
    Supported,
    Unsupported,
}

enum DestinationCode {
    Japan,
    Overseas,
}

pub trait MBC {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

pub fn create_MBC(rom: Vec<u8>) -> Box<dyn MBC> {
    if rom.is_empty() {
        return Box::new(NullMBC::new());
    }

    let header = parse_header(&rom);
    match header.cartridge_type {
        0x00 => Box::new(ROMOnly::new(&rom, false, header.ram_size, false)),
        0x01 => Box::new(MBC1::new(
            &rom,
            header.rom_size,
            false,
            header.ram_size,
            false,
        )),
        0x02 => Box::new(MBC1::new(
            &rom,
            header.rom_size,
            true,
            header.ram_size,
            false,
        )),
        0x03 => Box::new(MBC1::new(
            &rom,
            header.rom_size,
            true,
            header.ram_size,
            true,
        )),
        0x05 => todo!("Implement MBC2"),
        0x06 => todo!("Implement MBC2"),
        0x08 => Box::new(ROMOnly::new(&rom, true, header.ram_size, false)),
        0x09 => Box::new(ROMOnly::new(&rom, true, header.ram_size, true)),
        _ => panic!("Unknown cartridge type!"),
    }
}

fn parse_rom_size(data: u8) -> usize {
    match data {
        0x00 => 32768,
        0x01 => 2 * 32768,
        0x02 => 4 * 32768,
        0x03 => 8 * 32768,
        0x04 => 16 * 32768,
        0x05 => 32 * 32768,
        0x06 => 64 * 32768,
        0x07 => 128 * 32768,
        0x08 => 256 * 32768,
        _ => panic!("Unknown rom size: {}", data),
    }
}

fn parse_ram_size(data: u8) -> usize {
    match data {
        0x00 => 0,
        0x02 => 1 * 8192,
        0x03 => 4 * 8192,
        0x04 => 16 * 8192,
        0x05 => 8 * 8192,
        _ => panic!("Unknown ram size: {}", data),
    }
}

fn parse_header(rom: &Vec<u8>) -> CartridgeHeader {
    let title = match rom[0x143] {
        0x80 => String::from_utf8(Vec::from(&rom[0x134..0x13E]))
            .expect("Failed to parse title from cartridge header"),
        0xC0 => String::from_utf8(Vec::from(&rom[0x134..0x13E]))
            .expect("Failed to parse title from cartridge header"),
        _ => String::from_utf8(Vec::from(&rom[0x134..0x144]))
            .expect("Failed to parse title from cartridge header"),
    };

    CartridgeHeader {
        title,
        manufacturer_code: <[u8; 4]>::try_from(&rom[0x13F..0x143]).unwrap(),
        cgb_flag: match rom[0x143] {
            0x80 => CGBFlag::DMGCompatible,
            0xC0 => CGBFlag::CGBOnly,
            _ => CGBFlag::DMGCompatible,
        },
        new_licensee_code: ((rom[0x144] as u16) << 8) & (rom[0x145] as u16),
        sgb_flag: match rom[0x146] {
            0x03 => SGBFlag::Supported,
            _ => SGBFlag::Unsupported,
        },
        cartridge_type: rom[0x147],
        rom_size: parse_rom_size(rom[0x148]),
        ram_size: parse_ram_size(rom[0x149]),
        destination_code: match rom[0x14A] {
            0x00 => DestinationCode::Japan,
            0x01 => DestinationCode::Overseas,
            _ => DestinationCode::Overseas,
        },
        old_licensee_code: rom[0x14B],
        version_number: rom[0x14C],
        header_checksum: rom[0x14D],
        global_checksum: ((rom[0x14E] as u16) << 8) & (rom[0x14F] as u16),
    }
}
