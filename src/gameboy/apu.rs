pub(crate) struct APU {}

impl APU {
    pub(crate) fn new() -> APU {
        APU {}
    }

    pub(crate) fn tick(&self, cycles: u32) {}

    pub(crate) fn read(&self, address: u16) -> u8 {
        0xFF
    }

    pub(crate) fn write(&mut self, address: u16, value: u8) {}
}
