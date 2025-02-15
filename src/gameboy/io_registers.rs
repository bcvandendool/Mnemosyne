#![allow(non_snake_case)]
pub struct IORegisters {
    // IO registers
    FF01_serial_transfer_data: u8,
    FF01_serial_transfer_buffer: Vec<char>,
    FF02_serial_transfer_control: u8,
    FF04_DIV_divider_register: u8,
    FF05_TIMA_timer_counter: u8,
    FF06_TMA_timer_modulo: u8,
    FF07_TAC_timer_control: u8,
    FF0F_IF_interrupt_flag: u8,
    FF50_boot_rom_enabled: u8,
    FFFF_IE_interrupt_enable: u8,
    // Internal state
    clock_counter: u16,
    clock_frequencies: [u16; 4],
}

impl IORegisters {
    pub fn new() -> Self {
        IORegisters {
            // IO registers
            FF01_serial_transfer_data: 0x00,
            FF01_serial_transfer_buffer: Vec::new(),
            FF02_serial_transfer_control: 0x00,
            FF04_DIV_divider_register: 0x00,
            FF05_TIMA_timer_counter: 0x00,
            FF06_TMA_timer_modulo: 0x00,
            FF07_TAC_timer_control: 0x00,
            FF0F_IF_interrupt_flag: 0x00,
            FF50_boot_rom_enabled: 0x00,
            FFFF_IE_interrupt_enable: 0x00,
            // Internal state
            clock_counter: 0x00,
            clock_frequencies: [256, 4, 16, 64],
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {
        match address {
            0xFF01 => self.FF01_serial_transfer_data,
            0xFF02 => self.FF02_serial_transfer_control,
            0xFF04 => self.FF04_DIV_divider_register,
            0xFF05 => self.FF05_TIMA_timer_counter,
            0xFF06 => self.FF06_TMA_timer_modulo,
            0xFF07 => self.FF07_TAC_timer_control,
            0xFF0F => self.FF0F_IF_interrupt_flag,
            0xFF50 => self.FF50_boot_rom_enabled,
            0xFFFF => self.FFFF_IE_interrupt_enable,
            _ => 0, // TODO: implement all io registers
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0xFF01 => {
                self.FF01_serial_transfer_data = value;
                self.FF01_serial_transfer_buffer.push(value as char);
            }
            0xFF02 => self.FF02_serial_transfer_control = value,
            0xFF04 => self.FF04_DIV_divider_register = value,
            0xFF05 => self.FF05_TIMA_timer_counter = value,
            0xFF06 => self.FF06_TMA_timer_modulo = value,
            0xFF07 => self.FF07_TAC_timer_control = value,
            0xFF0F => self.FF0F_IF_interrupt_flag = value,
            0xFF50 => self.FF50_boot_rom_enabled = value,
            0xFFFF => self.FFFF_IE_interrupt_enable = value,
            _ => {} // TODO: implement all io registers
        }
    }

    pub fn serial_buffer(&self) -> &Vec<char> {
        &self.FF01_serial_transfer_buffer
    }

    pub fn update_timers(&mut self, cycles: u32) {
        // Increment DIV
        self.FF04_DIV_divider_register = self
            .FF04_DIV_divider_register
            .wrapping_add((cycles * 4) as u8);

        // Handle TIMA
        if self.FF07_TAC_timer_control & 0b100 > 0 {
            let clock_select = self.FF07_TAC_timer_control & 0b11;
            let clock_frequency = self.clock_frequencies[clock_select as usize];

            for _ in 0..cycles {
                self.clock_counter += 1;

                if self.clock_counter == clock_frequency {
                    self.clock_counter = 0;

                    let (value, overflowed) = self.FF05_TIMA_timer_counter.overflowing_add(1);
                    if overflowed {
                        self.FF0F_IF_interrupt_flag |= 0b100;
                        self.FF05_TIMA_timer_counter = self.FF06_TMA_timer_modulo;
                    } else {
                        self.FF05_TIMA_timer_counter = value;
                    }
                }
            }
        }
    }
}
