use crate::gameboy::mbc::MBC;

pub struct NullMBC {}

impl MBC for NullMBC {
    fn read(&self, address: u16) -> u8 {
        panic!()
    }

    fn write(&mut self, address: u16, value: u8) {
        panic!()
    }
}

impl NullMBC {
    pub fn new() -> Self {
        NullMBC {}
    }
}
