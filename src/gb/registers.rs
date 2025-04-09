#![allow(non_snake_case)]

use std::fmt::{Display, Formatter};

#[derive(Clone)]
pub struct Registers {
    // Normal register
    pub A: u8,
    pub B: u8,
    pub C: u8,
    pub D: u8,
    pub E: u8,
    pub H: u8,
    pub L: u8,
    // Flag register
    pub F: u8,
    // Stack Pointer
    pub SP: u16,
    // Program Counter
    pub PC: u16,
    pub IME: bool,
    // Instruction register
    pub IR: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Reg {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    F,
    SP,
    PC,
    AF,
    BC,
    DE,
    HL,
}

impl Display for Reg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub enum Flag {
    ZERO = 0x80,
    SUBTRACTION = 0x40,
    HALF_CARRY = 0x20,
    CARRY = 0x10,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ConditionCode {
    Z,
    NZ,
    C,
    NC,
}

impl Display for ConditionCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            A: 0,
            B: 0,
            C: 0,
            D: 0,
            E: 0,
            F: 0,
            H: 0,
            L: 0,
            SP: 0,
            PC: 0,
            IME: false,
            IR: 0x00,
        }
    }

    pub fn AF(&self) -> u16 {
        ((self.A as u16) << 8) | (self.F as u16)
    }

    pub fn set_AF(&mut self, value: u16) {
        self.A = (value >> 8) as u8;
        self.F = value as u8;
    }

    pub fn BC(&self) -> u16 {
        ((self.B as u16) << 8) | (self.C as u16)
    }

    pub fn set_BC(&mut self, value: u16) {
        self.B = (value >> 8) as u8;
        self.C = value as u8;
    }

    pub fn DE(&self) -> u16 {
        ((self.D as u16) << 8) | (self.E as u16)
    }

    pub fn set_DE(&mut self, value: u16) {
        self.D = (value >> 8) as u8;
        self.E = value as u8;
    }

    pub fn HL(&self) -> u16 {
        ((self.H as u16) << 8) | (self.L as u16)
    }

    pub fn set_HL(&mut self, value: u16) {
        self.H = (value >> 8) as u8;
        self.L = value as u8;
    }

    pub fn set_flag(&mut self, flag: Flag, value: bool) {
        if value {
            self.F |= flag as u8;
        } else {
            self.F &= !(flag as u8);
        }
    }

    pub fn has_flag(&self, flag: Flag) -> bool {
        let value = flag as u8;
        self.F & value == value
    }
}
