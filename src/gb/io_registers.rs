#![allow(non_snake_case)]

use intbits::Bits;
use log::{log, Level};
use std::collections::HashMap;
use winit::keyboard::KeyCode;

pub struct IORegisters {
    // IO registers
    pub(crate) FF00_JOYP: u8,
    FF01_serial_transfer_data: u8,
    FF01_serial_transfer_buffer: Vec<char>,
    FF02_serial_transfer_control: u8,
    FF04_DIV_divider_register: u8,
    FF05_TIMA_timer_counter: u8,
    FF06_TMA_timer_modulo: u8,
    FF07_TAC_timer_control: u8,
    pub(crate) FF0F_IF_interrupt_flag: u8,
    pub(crate) FF50_boot_rom_enabled: bool,
    FFFF_IE_interrupt_enable: u8,
    // Internal state
    clock_counter: u16,
    TIMA_overflowed: bool,
    TIMA_counter: u8,
    pub(crate) inputs: HashMap<KeyCode, bool>,
    should_update_DIV_APU: bool,
    serial_timer: u16,
}

impl IORegisters {
    pub fn new() -> Self {
        IORegisters {
            // IO registers
            FF00_JOYP: 0xC0,
            FF01_serial_transfer_data: 0x00,
            FF01_serial_transfer_buffer: Vec::new(),
            FF02_serial_transfer_control: 0x7E,
            FF04_DIV_divider_register: 0x00,
            FF05_TIMA_timer_counter: 0x00,
            FF06_TMA_timer_modulo: 0x00,
            FF07_TAC_timer_control: 0xF8,
            FF0F_IF_interrupt_flag: 0xE0,
            FF50_boot_rom_enabled: true,
            FFFF_IE_interrupt_enable: 0x00,
            // Internal state
            clock_counter: 0xABCD,
            TIMA_overflowed: false,
            TIMA_counter: 0,
            inputs: HashMap::new(),
            should_update_DIV_APU: false,
            serial_timer: 0,
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {
        match address {
            0xFF00 => {
                let mut value = self.FF00_JOYP | 0xF;
                if self.FF00_JOYP & 0x10 == 0 {
                    // d-pad
                    if *self.inputs.get(&KeyCode::ArrowDown).unwrap_or(&false) {
                        value &= 0b11110111;
                    } else if *self.inputs.get(&KeyCode::ArrowUp).unwrap_or(&false) {
                        value &= 0b11111011;
                    }
                    if *self.inputs.get(&KeyCode::ArrowLeft).unwrap_or(&false) {
                        value &= 0b11111101;
                    } else if *self.inputs.get(&KeyCode::ArrowRight).unwrap_or(&false) {
                        value &= 0b11111110;
                    }
                }
                if self.FF00_JOYP & 0x20 == 0 {
                    // buttons
                    if *self.inputs.get(&KeyCode::KeyF).unwrap_or(&false) {
                        value &= 0b11110111;
                    }
                    if *self.inputs.get(&KeyCode::KeyD).unwrap_or(&false) {
                        value &= 0b11111011;
                    }
                    if *self.inputs.get(&KeyCode::KeyS).unwrap_or(&false) {
                        value &= 0b11111101;
                    }
                    if *self.inputs.get(&KeyCode::KeyA).unwrap_or(&false) {
                        value &= 0b11111110;
                    }
                }
                value
            }
            0xFF01 => self.FF01_serial_transfer_data,
            0xFF02 => self.FF02_serial_transfer_control,
            0xFF04 => self.clock_counter.bits(8..16) as u8,
            0xFF05 => self.FF05_TIMA_timer_counter,
            0xFF06 => self.FF06_TMA_timer_modulo,
            0xFF07 => self.FF07_TAC_timer_control | 0xF8,
            0xFF0F => self.FF0F_IF_interrupt_flag | 0xE0,
            0xFF50 => 0xFF,
            0xFFFF => self.FFFF_IE_interrupt_enable,
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0xFF00 => self.FF00_JOYP = value | 0xC0,
            0xFF01 => {
                self.FF01_serial_transfer_data = value;
                self.FF01_serial_transfer_buffer.push(value as char);
                log!(Level::Info, "Serial to write: {}", value as char);
            }
            0xFF02 => {
                self.FF02_serial_transfer_control = value | 0x7E;
                if self.FF02_serial_transfer_control.bit(7)
                    && self.FF02_serial_transfer_control.bit(0)
                {
                    self.serial_timer = 8;
                }
            }
            0xFF04 => {
                let clock_select = self.FF07_TAC_timer_control.bits(0..2);
                if (clock_select == 0b00 && self.clock_counter.bit(9))
                    || (clock_select == 0b01 && self.clock_counter.bit(3))
                    || (clock_select == 0b10 && self.clock_counter.bit(5))
                    || (clock_select == 0b11 && self.clock_counter.bit(7))
                {
                    let (value, overflowed) = self.FF05_TIMA_timer_counter.overflowing_add(1);
                    if overflowed {
                        self.FF0F_IF_interrupt_flag |= 0b100;
                        self.FF05_TIMA_timer_counter = self.FF06_TMA_timer_modulo;
                        // self.TIMA_overflowed = true;
                        // self.TIMA_counter = 0;
                    } else {
                        self.FF05_TIMA_timer_counter = value;
                    }
                }
                // DIV-APU
                if self.clock_counter.bit(11) {
                    self.should_update_DIV_APU = true;
                }
                self.clock_counter = 0
            }
            0xFF05 => self.FF05_TIMA_timer_counter = value,
            0xFF06 => self.FF06_TMA_timer_modulo = value,
            0xFF07 => {
                if self.FF07_TAC_timer_control.bits(0..2) != value.bits(0..2) {
                    self.FF07_TAC_timer_control = value;
                    let clock_select = self.FF07_TAC_timer_control.bits(0..2);
                    if (clock_select == 0b00 && self.clock_counter.bits(0..10) == 0)
                        || (clock_select == 0b01 && self.clock_counter.bits(0..4) == 0)
                        || (clock_select == 0b10 && self.clock_counter.bits(0..6) == 0)
                        || (clock_select == 0b11 && self.clock_counter.bits(0..8) == 0)
                    {
                        let (value, overflowed) = self.FF05_TIMA_timer_counter.overflowing_add(1);
                        if overflowed {
                            self.FF0F_IF_interrupt_flag |= 0b100;
                            self.FF05_TIMA_timer_counter = self.FF06_TMA_timer_modulo;
                            // self.TIMA_overflowed = true;
                            // self.TIMA_counter = 0;
                        } else {
                            self.FF05_TIMA_timer_counter = value;
                        }
                    }
                } else if self.FF07_TAC_timer_control.bit(2) && !value.bit(2) {
                    self.FF07_TAC_timer_control = value;
                    let clock_select = self.FF07_TAC_timer_control.bits(0..2);
                    if (clock_select == 0b00 && self.clock_counter.bit(9))
                        || (clock_select == 0b01 && self.clock_counter.bit(3))
                        || (clock_select == 0b10 && self.clock_counter.bit(5))
                        || (clock_select == 0b11 && self.clock_counter.bit(7))
                    {
                        let (value, overflowed) = self.FF05_TIMA_timer_counter.overflowing_add(1);
                        if overflowed {
                            self.FF0F_IF_interrupt_flag |= 0b100;
                            self.FF05_TIMA_timer_counter = self.FF06_TMA_timer_modulo;
                            // self.TIMA_overflowed = true;
                            // self.TIMA_counter = 0;
                        } else {
                            self.FF05_TIMA_timer_counter = value;
                        }
                    }
                } else {
                    self.FF07_TAC_timer_control = value
                }
            }
            0xFF0F => self.FF0F_IF_interrupt_flag = value,
            0xFF50 => {
                self.FF50_boot_rom_enabled = false;
                // Bypasses bootix DIV state issue
                // See https://github.com/Hacktix/Bootix/issues/2
                self.clock_counter = 0xABCD;
            }
            0xFFFF => self.FFFF_IE_interrupt_enable = value,
            _ => {} // TODO: implement all io registers
        }
    }

    pub fn serial_buffer(&self) -> &Vec<char> {
        &self.FF01_serial_transfer_buffer
    }

    pub fn update_timers(&mut self) -> bool {
        self.clock_counter = self.clock_counter.wrapping_add(1);

        if self.FF02_serial_transfer_control.bit(7) && self.FF02_serial_transfer_control.bit(0) {
            if self.serial_timer > 0 && self.clock_counter.bits(0..9) == 0 {
                self.serial_timer -= 1;
            }

            if self.serial_timer == 0 {
                self.FF02_serial_transfer_control.set_bit(7, false);
                self.FF0F_IF_interrupt_flag.set_bit(3, true);
                self.FF01_serial_transfer_data = 0xFF;
            }
        }

        let mut DIV_APU = self.should_update_DIV_APU;
        self.should_update_DIV_APU = false;
        if self.clock_counter.bits(0..13) == 0 {
            DIV_APU = true;
        }

        // if self.TIMA_overflowed {
        //     if self.TIMA_counter < 3 {
        //         self.TIMA_counter += 1;
        //     } else {
        //         self.TIMA_counter = 0;
        //         self.TIMA_overflowed = false;
        //         self.FF05_TIMA_timer_counter = self.FF06_TMA_timer_modulo;
        //         self.FF0F_IF_interrupt_flag |= 0b100;
        //     }
        // }

        if self.FF07_TAC_timer_control.bit(2) {
            let clock_select = self.FF07_TAC_timer_control.bits(0..2);

            if (clock_select == 0b00 && self.clock_counter.bits(0..10) == 0)
                || (clock_select == 0b01 && self.clock_counter.bits(0..4) == 0)
                || (clock_select == 0b10 && self.clock_counter.bits(0..6) == 0)
                || (clock_select == 0b11 && self.clock_counter.bits(0..8) == 0)
            {
                let (value, overflowed) = self.FF05_TIMA_timer_counter.overflowing_add(1);
                if overflowed {
                    self.FF0F_IF_interrupt_flag |= 0b100;
                    self.FF05_TIMA_timer_counter = self.FF06_TMA_timer_modulo;
                    // self.TIMA_overflowed = true;
                    // self.TIMA_counter = 0;
                } else {
                    self.FF05_TIMA_timer_counter = value;
                }
            }
        }
        DIV_APU
    }
}
