pub(crate) struct DMA {
    dot_counter: u16,
    source_address: u16,
    transfer_active: bool,
}

impl DMA {
    pub(crate) fn new() -> Self {
        DMA {
            dot_counter: 0,
            source_address: 0x0000,
            transfer_active: false,
        }
    }

    pub(crate) fn tick(&mut self, cycles: u32) {
        for _ in 0..cycles {
            if self.transfer_active {
                if self.dot_counter % 4 == 0 {
                    // Transfer next byte
                    let src = self.source_address + self.dot_counter / 4;
                    let dst = 0xFE00 + self.dot_counter / 4;
                }

                if self.dot_counter == 640 {
                    self.transfer_active = false;
                    self.dot_counter = 0;
                } else {
                    self.dot_counter += 1;
                }
            }
        }
    }

    pub(crate) fn read(&self, address: u16) -> u8 {
        0xFF
    }

    pub(crate) fn write(&mut self, address: u16, value: u8) {
        if address == 0xFF46 {
            self.source_address = (value as u16) << 8;
            self.transfer_active = true;
        }
    }
}
