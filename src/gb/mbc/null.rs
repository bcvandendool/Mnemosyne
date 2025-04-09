use crate::gb::mbc::MBC;

pub struct NullMBC {}

impl MBC for NullMBC {
    fn read(&self, address: u16) -> u8 {
        panic!()
    }
    fn write(&mut self, address: u16, value: u8) {
        panic!()
    }
    fn name(&self) -> String {
        String::from("NULL")
    }
    fn save_ram(&self) {}
    fn load_ram(&mut self) {}
}

impl NullMBC {
    pub fn new() -> Self {
        NullMBC {}
    }
}
