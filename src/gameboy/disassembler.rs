pub(crate) struct Disassembler {
    table_r: Vec<String>,
    table_rp: Vec<String>,
    table_rp2: Vec<String>,
    table_cc: Vec<String>,
    table_alu: Vec<String>,
    table_rot: Vec<String>,
    // TODO: cache
}
impl Disassembler {
    pub(crate) fn new() -> Self {
        Disassembler {
            table_r: vec![
                String::from("B"),
                String::from("C"),
                String::from("D"),
                String::from("E"),
                String::from("H"),
                String::from("L"),
                String::from("[HL]"),
                String::from("A"),
            ],
            table_rp: vec![
                String::from("BC"),
                String::from("DE"),
                String::from("HL"),
                String::from("SP"),
            ],
            table_rp2: vec![
                String::from("BC"),
                String::from("DE"),
                String::from("HL"),
                String::from("AF"),
            ],
            table_cc: vec![
                String::from("NZ"),
                String::from("Z"),
                String::from("NC"),
                String::from("C"),
            ],
            table_alu: vec![
                String::from("ADD A,"),
                String::from("ADC A,"),
                String::from("SUB A,"),
                String::from("SBC A,"),
                String::from("AND A,"),
                String::from("XOR A,"),
                String::from("OR A,"),
                String::from("CP A,"),
            ],
            table_rot: vec![
                String::from("RLC"),
                String::from("RRC"),
                String::from("RL"),
                String::from("RR"),
                String::from("SLA"),
                String::from("SRA"),
                String::from("SWAP"),
                String::from("SRL"),
            ],
        }
    }

    pub(crate) fn disassemble_section(
        &self,
        rom: &Vec<u8>,
        start_address: u16,
        num_bytes: u16,
    ) -> Vec<(u16, String)> {
        let mut address = start_address;
        let mut result: Vec<(u16, String)> = Vec::new();

        while address <= start_address + num_bytes {
            let (mut opcode, offset) = self.disassemble(rom, address);

            opcode.push_str(" ; ");
            for i in 0..offset {
                opcode.push_str(format!("{:#04X} ", rom[address as usize + i as usize]).as_str());
            }

            result.push((address, opcode));
            address += offset;
        }
        result
    }

    pub(crate) fn disassemble(&self, rom: &Vec<u8>, address: u16) -> (String, u16) {
        let opcode = rom[address as usize];

        // Decoding based on https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
        let x = ((opcode & 0b11000000) >> 6) as usize;
        let y = ((opcode & 0b00111000) >> 3) as usize;
        let z = (opcode & 0b00000111) as usize;
        let p = y >> 1;
        let q = y % 2;

        match x {
            0 => match z {
                0 => match y {
                    0 => (String::from("NOP"), 1),
                    1 => (format!("LD {:#06X}, SP", self.get_nn(rom, address)), 3),
                    2 => (String::from("STOP"), 1),
                    3 => (format!("JR {}", self.get_d(rom, address)), 2),
                    4..=7 => (
                        format!(
                            "JR {}, {}",
                            self.table_cc[((opcode & 0b11000) >> 3) as usize],
                            self.get_d(rom, address)
                        ),
                        2,
                    ),
                    _ => (format!("{} ; DATA", opcode), 1),
                },
                1 => match q {
                    0 => (
                        format!(
                            "LD {}, {:#06X}",
                            self.table_rp[p],
                            self.get_nn(rom, address)
                        ),
                        3,
                    ),
                    1 => (format!("ADD HL, {}", self.table_rp[p]), 1),
                    _ => (format!("{} ; DATA", opcode), 1),
                },
                2 => match q {
                    0 => match p {
                        0 => (String::from("LD (BC), A"), 1),
                        1 => (String::from("LD (DE), A"), 1),
                        2 => (String::from("LD (HL+), A"), 1),
                        3 => (String::from("LD (HL-), A"), 1),
                        _ => (format!("{} ; DATA", opcode), 1),
                    },
                    1 => match p {
                        0 => (String::from("LD A, (BC)"), 1),
                        1 => (String::from("LD A, (DE)"), 1),
                        2 => (String::from("LD A, (HL+)"), 1),
                        3 => (String::from("LD A, (HL-)"), 1),
                        _ => (format!("{} ; DATA", opcode), 1),
                    },
                    _ => (format!("{} ; DATA", opcode), 1),
                },
                3 => match q {
                    0 => (format!("INC {}", self.table_rp[p]), 1),
                    1 => (format!("DEC {}", self.table_rp[p]), 1),
                    _ => (format!("{} ; DATA", opcode), 1),
                },
                4 => (format!("INC {}", self.table_r[y]), 1),
                5 => (format!("DEC {}", self.table_r[y]), 1),
                6 => (
                    format!("LD {}, {:#04X}", self.table_r[y], self.get_n(rom, address)),
                    2,
                ),
                7 => match y {
                    0 => (String::from("RLCA"), 1),
                    1 => (String::from("RRCA"), 1),
                    2 => (String::from("RLA"), 1),
                    3 => (String::from("RRA"), 1),
                    4 => (String::from("DAA"), 1),
                    5 => (String::from("CPL"), 1),
                    6 => (String::from("SCF"), 1),
                    7 => (String::from("CCF"), 1),
                    _ => (format!("{} ; DATA", opcode), 1),
                },
                _ => (format!("{} ; DATA", opcode), 1),
            },
            1 => {
                if z == 6 && y == 6 {
                    (String::from("HALT"), 1)
                } else {
                    (format!("LD {}, {}", self.table_r[y], self.table_r[z]), 1)
                }
            }
            2 => (format!("{} {}", self.table_alu[y], self.table_r[z]), 1),

            3 => match z {
                0 => match y {
                    0..=3 => (format!("RET {}", self.table_cc[y]), 1),
                    4 => (
                        format!("LD (0xFF00 + {:#04X}), A", self.get_n(rom, address)),
                        2,
                    ),
                    5 => (format!("ADD SP, {}", self.get_d(rom, address)), 2),
                    6 => (
                        format!("LD A, (0xFF00 + {:#04X})", self.get_n(rom, address)),
                        2,
                    ),
                    7 => (format!("LD HL, SP + {}", self.get_d(rom, address)), 2),
                    _ => (format!("{} ; DATA", opcode), 1),
                },
                1 => match q {
                    0 => (format!("POP {}", self.table_rp2[p]), 1),
                    1 => match p {
                        0 => (String::from("RET"), 1),
                        1 => (String::from("RETI"), 1),
                        2 => (String::from("JP HL"), 1),
                        3 => (String::from("LD SP, HL"), 1),
                        _ => (format!("{} ; DATA", opcode), 1),
                    },
                    _ => (format!("{} ; DATA", opcode), 1),
                },
                2 => match y {
                    0..=3 => (
                        format!(
                            "JP {}, {:#06X}",
                            self.table_cc[y],
                            self.get_nn(rom, address)
                        ),
                        3,
                    ),
                    4 => (String::from("LD (0xFF00 + C), A"), 1),
                    5 => (format!("LD [{:#06X}], A", self.get_nn(rom, address)), 3),
                    6 => (String::from("LD A, (0xFF00 + C)"), 1),
                    7 => (format!("LD A, [{:#06X}]", self.get_nn(rom, address)), 3),
                    _ => (format!("{} ; DATA", opcode), 1),
                },
                3 => match y {
                    0 => (format!("JP {:#06X}", self.get_nn(rom, address)), 3),
                    1 => {
                        let opcode_bc = rom[address as usize + 1];
                        let x = ((opcode_bc & 0b11000000) >> 6) as usize;
                        let y = ((opcode_bc & 0b00111000) >> 3) as usize;
                        let z = (opcode_bc & 0b00000111) as usize;
                        match x {
                            0 => (format!("{} {}", self.table_rot[y], self.table_r[z]), 2),
                            1 => (format!("BIT {}, {}", y, self.table_r[z]), 2),
                            2 => (format!("RES {}, {}", y, self.table_r[z]), 2),
                            3 => (format!("SET {}, {}", y, self.table_r[z]), 2),
                            _ => (format!("{} ; DATA", opcode), 2),
                        }
                    }
                    6 => (String::from("DI"), 1),
                    7 => (String::from("EI"), 1),
                    _ => (format!("{} ; DATA", opcode), 1),
                },
                4 => match y {
                    0..3 => (
                        format!(
                            "CALL {}, {:#06X}",
                            self.table_cc[y],
                            self.get_nn(rom, address)
                        ),
                        3,
                    ),
                    _ => (format!("{} ; DATA", opcode), 1),
                },
                5 => match q {
                    0 => (format!("PUSH {}", self.table_rp2[p]), 1),
                    1 => (format!("CALL, {:#06X}", self.get_nn(rom, address)), 3),
                    _ => (format!("{} ; DATA", opcode), 1),
                },
                6 => (
                    format!("{} {:#04X}", self.table_alu[y], self.get_n(rom, address)),
                    2,
                ),
                7 => (format!("RST {:#04X}", y * 8), 1),
                _ => (format!("{} ; DATA", opcode), 1),
            },
            _ => (format!("{} ; DATA", opcode), 1),
        }
    }

    fn get_d(&self, rom: &Vec<u8>, address: u16) -> i8 {
        rom[(address + 1) as usize] as i8
    }

    fn get_n(&self, rom: &Vec<u8>, address: u16) -> u8 {
        rom[(address + 1) as usize]
    }

    fn get_nn(&self, rom: &Vec<u8>, address: u16) -> u16 {
        let value_lo = rom[(address + 1) as usize];
        let value_hi = rom[(address + 2) as usize];
        ((value_hi as u16) << 8) | (value_lo as u16)
    }
}
