use crate::gameboy::mbc::MBC;

pub struct NullMBC {}

impl MBC for NullMBC {
    fn read(&self, address: u16) -> u8 {
        todo!()
    }

    fn write(&self, address: u16, value: u8) {
        todo!()
    }
}

impl NullMBC {
    pub fn new() -> Self {
        NullMBC {}
    }
}
