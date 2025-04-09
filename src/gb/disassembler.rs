#![allow(non_camel_case_types)]

mod constants;
mod instruction;

use crate::gb::disassembler::constants::INITIAL_SYMBOLS;
use crate::gb::registers::Reg;
use constants::{TABLE_CC, TABLE_R, TABLE_RP, TABLE_RP2};
use instruction::Instruction;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::{BufRead, LineWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Eq, Hash, PartialEq)]
pub(crate) struct Address {
    bank: u8,
    pub(crate) address: u16,
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub(crate) struct Label {
    address: Address,
    name: String,
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub(crate) enum SymbolType {
    Code,
    Data(usize),
    Text(usize),
    Image(usize),
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub(crate) struct Symbol {
    label: Label,
    symbol_type: SymbolType,
}

pub(crate) struct BasicBlock {
    start_address: Address,
    length: usize,
    instructions: Vec<(Address, Instruction)>,
    rom: bool,
}

pub(crate) struct Disassembler {
    rom: Vec<u8>,
    symbols: HashSet<Symbol>,
    addresses_to_process: HashSet<Address>,
    basic_blocks: Vec<BasicBlock>,
}

impl Disassembler {
    pub(crate) fn new(rom_path: &Path) -> Self {
        // Load rom
        let rom = fs::read(rom_path).expect("Failed to load rom");

        // Load symbols
        let mut sym_path = PathBuf::from(rom_path);
        sym_path.set_extension("sym");
        let symbols = if sym_path.exists() {
            Disassembler::load_sym_file(&sym_path)
        } else {
            INITIAL_SYMBOLS.to_vec()
        };

        Disassembler {
            rom,
            symbols: HashSet::from_iter(symbols.iter().cloned()),
            addresses_to_process: HashSet::from_iter(symbols.iter().filter_map(|symbol| {
                if symbol.symbol_type == SymbolType::Code {
                    Some(symbol.label.address.clone())
                } else {
                    None
                }
            })),
            basic_blocks: Vec::new(),
        }
    }

    pub(crate) fn disassemble(&mut self) {
        while !self.addresses_to_process.is_empty() {
            let address = self.addresses_to_process.iter().next().unwrap().clone();
            self.addresses_to_process.remove(&address);

            let basic_block = self.disassemble_basic_block(address.clone(), true);
            self.basic_blocks.push(basic_block);

            if !self
                .symbols
                .iter()
                .any(|symbol| symbol.label.address == address)
            {
                let name = format!("FN_{:02X}_{:04X}", address.bank, address.address).to_string();
                self.symbols.insert(Symbol {
                    label: Label { address, name },
                    symbol_type: SymbolType::Code,
                });
            }
        }
    }

    pub(crate) fn to_table(&self) -> Vec<(Option<Address>, String)> {
        let mut table: Vec<(Option<Address>, String)> = Vec::new();
        let mut index = 0;
        while index < self.rom.len() {
            // Check if symbol exists with this index as start address
            if let Some(symbol) = self
                .symbols
                .iter()
                .find(|symbol| symbol.label.address.address == index as u16)
            {
                table.push((None, format!("{}::", symbol.label.name)));
                if symbol.symbol_type == SymbolType::Code {
                    let basic_block = self
                        .basic_blocks
                        .iter()
                        .find(|basic_block| basic_block.start_address == symbol.label.address)
                        .unwrap();
                    for instr in &basic_block.instructions {
                        table.push((Some(instr.0.clone()), instr.1.to_string()));
                    }
                    index += basic_block.length;
                } else {
                    match symbol.symbol_type {
                        SymbolType::Data(size) | SymbolType::Image(size) => {
                            for i in 0..size {
                                table.push((
                                    Some(Address {
                                        address: (index + i) as u16,
                                        bank: 0,
                                    }),
                                    format!("db ${:02X}", self.rom[index + i]),
                                ));
                            }
                            index += size;
                        }
                        SymbolType::Text(size) => {
                            table.push((
                                Some(Address {
                                    address: index as u16,
                                    bank: 0,
                                }),
                                String::from_utf8(self.rom[index..=index + size].to_owned())
                                    .unwrap(),
                            ));
                            index += size;
                        }
                        _ => panic!("Impossible"),
                    }
                }
            } else {
                // Check what the next symbol is, and write until then as data
                let next_symbol = self
                    .symbols
                    .iter()
                    .filter(|symbol| symbol.label.address.address > index as u16)
                    .min_by_key(|symbol| symbol.label.address.address);

                match next_symbol {
                    Some(symbol) => {
                        for i in index..=symbol.label.address.address.into() {
                            table.push((
                                Some(Address {
                                    address: i as u16,
                                    bank: 0,
                                }),
                                format!("db ${:02X}", self.rom[i]),
                            ));
                        }
                        index = symbol.label.address.address.into();
                    }
                    None => {
                        // No symbols remaining, continue till end of rom
                        for i in index..self.rom.len() {
                            table.push((
                                Some(Address {
                                    address: i as u16,
                                    bank: 0,
                                }),
                                format!("db ${:02X}", self.rom[i]),
                            ));
                        }
                        index = self.rom.len();
                    }
                }
            }
        }
        table
    }

    pub(crate) fn load_sym_file(sym_path: &Path) -> Vec<Symbol> {
        let mut symbols: Vec<Symbol> = Vec::new();

        let lines: Vec<String> = fs::read_to_string(sym_path)
            .unwrap()
            .lines()
            .map(String::from)
            .collect();

        let mut index = 0;
        while index < lines.len() {
            // parse line
            let data: Vec<&str> = lines[index].split_whitespace().collect();
            let full_address: Vec<&str> = data[0].split(":").collect();
            let name = data[1];

            let label = Label {
                address: Address {
                    bank: u8::from_str_radix(full_address[0], 16).expect("Failed to parse bank"),
                    address: u16::from_str_radix(full_address[1], 16)
                        .expect("Failed to parse address"),
                },
                name: name.parse().unwrap(),
            };

            // check if next line gives more info about symbol
            if index + 1 < lines.len()
                && lines[index + 1].split_whitespace().next().unwrap() == data[0]
            {
                let info: Vec<&str> = lines[index + 1].split_whitespace().collect::<Vec<&str>>()[1]
                    .split(":")
                    .collect();
                let size = usize::from_str_radix(info[1], 16).expect("Failed to parse size");
                let symbol_type = match info[0] {
                    ".code" => SymbolType::Code,
                    ".data" => SymbolType::Data(size),
                    ".text" => SymbolType::Text(size),
                    ".image" => SymbolType::Image(size),
                    _ => {
                        panic!()
                    }
                };
                symbols.push(Symbol { label, symbol_type });
                index += 2;
            } else {
                symbols.push(Symbol {
                    label,
                    symbol_type: SymbolType::Code,
                });
                index += 1;
            }
        }
        symbols
    }
    pub(crate) fn save_sym_file(&self, sym_path: &Path) {
        let file = File::create(sym_path).expect("Failed to create sym file");
        let mut file = LineWriter::new(file);

        let mut symbols: Vec<&Symbol> = self.symbols.iter().collect();
        symbols.sort_by(|a, b| a.label.address.address.cmp(&b.label.address.address));

        for symbol in symbols {
            // Write first line with address and name
            file.write_all(
                format!(
                    "{:x}:{:04x} {}\n",
                    symbol.label.address.bank, symbol.label.address.address, symbol.label.name
                )
                .as_bytes(),
            )
            .expect("Failed to write to sym file");
            // Write second line with type and size
            if symbol.symbol_type != SymbolType::Code {
                file.write_all(
                    format!(
                        "{:x}:{:04x} ",
                        symbol.label.address.bank, symbol.label.address.address
                    )
                    .as_bytes(),
                )
                .expect("Failed to write to sym file");
                match symbol.symbol_type {
                    SymbolType::Data(size) => file
                        .write_all(format!(".data:{:x}\n", size).as_bytes())
                        .unwrap(),
                    SymbolType::Text(size) => file
                        .write_all(format!(".text:{:x}\n", size).as_bytes())
                        .unwrap(),
                    SymbolType::Image(size) => file
                        .write_all(format!(".image:{:x}\n", size).as_bytes())
                        .unwrap(),
                    _ => panic!("Impossible"),
                }
            }
        }
        file.flush().unwrap();
    }

    pub(crate) fn add_symbol(&mut self, address: Address) {
        // TODO
    }

    pub(crate) fn disassemble_basic_block(
        &mut self,
        //data: &[u8],
        address: Address,
        rom: bool,
    ) -> BasicBlock {
        let mut basic_block = BasicBlock {
            start_address: address.clone(),
            length: 0,
            instructions: vec![],
            rom,
        };

        let mut current_address = address;

        loop {
            let instruction =
                Disassembler::disassemble_instruction(&self.rom, current_address.address);
            basic_block
                .instructions
                .push((current_address.clone(), instruction.clone()));
            current_address.address += instruction.bytes();
            basic_block.length += instruction.bytes() as usize;

            match instruction {
                Instruction::JR_cc_n16(_, addr)
                | Instruction::JP_cc_n16(_, addr)
                | Instruction::CALL_n16(addr)
                | Instruction::CALL_cc_n16(_, addr) => {
                    // Jump and step
                    if !self
                        .symbols
                        .iter()
                        .any(|symbol| symbol.label.address == current_address)
                        && current_address.address < 0x7FFF
                    {
                        self.addresses_to_process.insert(current_address.clone());
                    }

                    if !self
                        .symbols
                        .iter()
                        .any(|symbol| symbol.label.address.address == addr)
                        && addr < 0x7FFF
                    {
                        self.addresses_to_process.insert(Address {
                            address: addr,
                            bank: current_address.bank,
                        });
                    }
                    break;
                }
                Instruction::JR_n16(addr) | Instruction::JP_n16(addr) => {
                    // Jump only
                    if !self
                        .symbols
                        .iter()
                        .any(|symbol| symbol.label.address.address == addr)
                        && addr < 0x7FFF
                    {
                        self.addresses_to_process.insert(Address {
                            address: addr,
                            bank: current_address.bank,
                        });
                    }
                    break;
                }
                Instruction::RET | Instruction::RETI => {
                    // Ends basic block
                    break;
                }
                Instruction::RET_cc(_) => {
                    // Step only
                    if !self
                        .symbols
                        .iter()
                        .any(|symbol| symbol.label.address == current_address)
                        && current_address.address < 0x7FFF
                    {
                        self.addresses_to_process.insert(current_address.clone());
                    }
                    break;
                }
                Instruction::RST_vec(addr) => {
                    // Jump only
                    if !self
                        .symbols
                        .iter()
                        .any(|symbol| symbol.label.address.address == addr as u16)
                    {
                        self.addresses_to_process.insert(Address {
                            address: addr as u16,
                            bank: current_address.bank,
                        });
                    }
                    break;
                }
                _ => {
                    if self
                        .symbols
                        .iter()
                        .any(|symbol| symbol.label.address == current_address)
                    {
                        // End basic block
                        break;
                    }
                }
            }
        }

        basic_block
    }

    pub(crate) fn disassemble_instruction(rom: &[u8], address: u16) -> Instruction {
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
                    0 => Instruction::NOP,
                    1 => Instruction::LD_n16_SP(Disassembler::get_nn(rom, address)),
                    2 => Instruction::STOP,
                    3 => Instruction::JR_n16(
                        (address as i16 + Disassembler::get_d(rom, address) as i16) as u16,
                    ),
                    4..=7 => Instruction::JR_cc_n16(
                        TABLE_CC[((opcode & 0b11000) >> 3) as usize].clone(),
                        (address as i16 + Disassembler::get_d(rom, address) as i16) as u16,
                    ),
                    _ => Instruction::INVALID(opcode),
                },
                1 => match q {
                    0 => Instruction::LD_r16_n16(
                        TABLE_RP[p].clone(),
                        Disassembler::get_nn(rom, address),
                    ),
                    1 => Instruction::ADD_HL_r16(TABLE_RP[p].clone()),
                    _ => Instruction::INVALID(opcode),
                },
                2 => match q {
                    0 => match p {
                        0 => Instruction::LD_r16_A(Reg::BC),
                        1 => Instruction::LD_r16_A(Reg::DE),
                        2 => Instruction::LD_HLI_A,
                        3 => Instruction::LD_HLD_A,
                        _ => Instruction::INVALID(opcode),
                    },
                    1 => match p {
                        0 => Instruction::LD_A_r16(Reg::BC),
                        1 => Instruction::LD_A_r16(Reg::DE),
                        2 => Instruction::LD_A_HLI,
                        3 => Instruction::LD_A_HLD,
                        _ => Instruction::INVALID(opcode),
                    },
                    _ => Instruction::INVALID(opcode),
                },
                3 => match q {
                    0 => Instruction::INC_r16(TABLE_RP[p].clone()),
                    1 => Instruction::DEC_r16(TABLE_RP[p].clone()),
                    _ => Instruction::INVALID(opcode),
                },
                4 => Instruction::INC_r8(TABLE_R[y].clone()),
                5 => Instruction::DEC_r8(TABLE_R[y].clone()),
                6 => Instruction::LD_r8_n8(TABLE_R[y].clone(), Disassembler::get_n(rom, address)),
                7 => match y {
                    0 => Instruction::RLCA,
                    1 => Instruction::RRCA,
                    2 => Instruction::RLA,
                    3 => Instruction::RRA,
                    4 => Instruction::DAA,
                    5 => Instruction::CPL,
                    6 => Instruction::SCF,
                    7 => Instruction::CCF,
                    _ => Instruction::INVALID(opcode),
                },
                _ => Instruction::INVALID(opcode),
            },
            1 => {
                if z == 6 && y == 6 {
                    Instruction::HALT
                } else if y == 6 {
                    Instruction::LD_HL_r8(TABLE_R[z].clone())
                } else if z == 6 {
                    Instruction::LD_r8_HL(TABLE_R[y].clone())
                } else {
                    Instruction::LD_r8_r8(TABLE_R[y].clone(), TABLE_R[z].clone())
                }
            }
            2 => {
                if z != 6 {
                    match y {
                        0 => Instruction::ADD_A_r8(TABLE_R[z].clone()),
                        1 => Instruction::ADC_A_r8(TABLE_R[z].clone()),
                        2 => Instruction::SUB_A_r8(TABLE_R[z].clone()),
                        3 => Instruction::SBC_A_r8(TABLE_R[z].clone()),
                        4 => Instruction::AND_A_r8(TABLE_R[z].clone()),
                        5 => Instruction::XOR_A_r8(TABLE_R[z].clone()),
                        6 => Instruction::OR_A_r8(TABLE_R[z].clone()),
                        7 => Instruction::CP_A_r8(TABLE_R[z].clone()),
                        _ => Instruction::INVALID(opcode),
                    }
                } else {
                    match y {
                        0 => Instruction::ADD_A_HL,
                        1 => Instruction::ADC_A_HL,
                        2 => Instruction::SUB_A_HL,
                        3 => Instruction::SBC_A_HL,
                        4 => Instruction::AND_A_HL,
                        5 => Instruction::XOR_A_HL,
                        6 => Instruction::OR_A_HL,
                        7 => Instruction::CP_A_HL,
                        _ => Instruction::INVALID(opcode),
                    }
                }
            }
            3 => match z {
                0 => match y {
                    0..=3 => Instruction::RET_cc(TABLE_CC[y].clone()),
                    4 => Instruction::LDH_A_n16(address + Disassembler::get_n(rom, address) as u16),
                    5 => Instruction::ADD_SP_e8(Disassembler::get_d(rom, address)),
                    6 => Instruction::LDH_A_n16(address + Disassembler::get_n(rom, address) as u16),
                    7 => Instruction::LD_HL_SP_e8(Disassembler::get_d(rom, address)),
                    _ => Instruction::INVALID(opcode),
                },
                1 => match q {
                    0 => Instruction::POP_r16(TABLE_RP2[p].clone()),
                    1 => match p {
                        0 => Instruction::RET,
                        1 => Instruction::RETI,
                        2 => Instruction::JP_HL,
                        3 => Instruction::LD_SP_HL,
                        _ => Instruction::INVALID(opcode),
                    },
                    _ => Instruction::INVALID(opcode),
                },
                2 => match y {
                    0..=3 => Instruction::JP_cc_n16(
                        TABLE_CC[y].clone(),
                        Disassembler::get_nn(rom, address),
                    ),
                    4 => Instruction::LDH_C_A,
                    5 => Instruction::LD_n16_A(Disassembler::get_nn(rom, address)),
                    6 => Instruction::LDH_A_C,
                    7 => Instruction::LD_A_n16(Disassembler::get_nn(rom, address)),
                    _ => Instruction::INVALID(opcode),
                },
                3 => match y {
                    0 => Instruction::JP_n16(Disassembler::get_nn(rom, address)),
                    1 => {
                        let opcode_bc = rom[address as usize + 1];
                        let x = ((opcode_bc & 0b11000000) >> 6) as usize;
                        let y = (opcode_bc & 0b00111000) >> 3;
                        let z = (opcode_bc & 0b00000111) as usize;
                        match x {
                            0 => match y {
                                0 => Instruction::RLC_r8(TABLE_R[z].clone()),
                                1 => Instruction::RRC_r8(TABLE_R[z].clone()),
                                2 => Instruction::RL_r8(TABLE_R[z].clone()),
                                3 => Instruction::RR_r8(TABLE_R[z].clone()),
                                4 => Instruction::SLA_r8(TABLE_R[z].clone()),
                                5 => Instruction::SRA_r8(TABLE_R[z].clone()),
                                6 => Instruction::SWAP_r8(TABLE_R[z].clone()),
                                7 => Instruction::SRL_r8(TABLE_R[z].clone()),
                                _ => Instruction::INVALID(opcode),
                            },
                            1 => Instruction::BIT_u3_r8(y, TABLE_R[z].clone()),
                            2 => Instruction::RES_u3_r8(y, TABLE_R[z].clone()),
                            3 => Instruction::SET_u3_r8(y, TABLE_R[z].clone()),
                            _ => Instruction::INVALID(opcode),
                        }
                    }
                    6 => Instruction::DI,
                    7 => Instruction::EI,
                    _ => Instruction::INVALID(opcode),
                },
                4 => match y {
                    0..3 => Instruction::CALL_cc_n16(
                        TABLE_CC[y].clone(),
                        Disassembler::get_nn(rom, address),
                    ),
                    _ => Instruction::INVALID(opcode),
                },
                5 => match q {
                    0 => Instruction::PUSH_r16(TABLE_RP2[p].clone()),
                    1 => Instruction::CALL_n16(Disassembler::get_nn(rom, address)),
                    _ => Instruction::INVALID(opcode),
                },
                6 => match y {
                    0 => Instruction::ADD_A_n8(Disassembler::get_n(rom, address)),
                    1 => Instruction::ADC_A_n8(Disassembler::get_n(rom, address)),
                    2 => Instruction::SUB_A_n8(Disassembler::get_n(rom, address)),
                    3 => Instruction::SBC_A_n8(Disassembler::get_n(rom, address)),
                    4 => Instruction::AND_A_n8(Disassembler::get_n(rom, address)),
                    5 => Instruction::XOR_A_n8(Disassembler::get_n(rom, address)),
                    6 => Instruction::OR_A_n8(Disassembler::get_n(rom, address)),
                    7 => Instruction::CP_A_n8(Disassembler::get_n(rom, address)),
                    _ => Instruction::INVALID(opcode),
                },
                7 => Instruction::RST_vec((y * 8) as u8),
                _ => Instruction::INVALID(opcode),
            },
            _ => Instruction::INVALID(opcode),
        }
    }

    fn get_d(rom: &[u8], address: u16) -> i8 {
        rom[(address + 1) as usize] as i8
    }

    fn get_n(rom: &[u8], address: u16) -> u8 {
        rom[(address + 1) as usize]
    }

    fn get_nn(rom: &[u8], address: u16) -> u16 {
        let value_lo = rom[(address + 1) as usize];
        let value_hi = rom[(address + 2) as usize];
        ((value_hi as u16) << 8) | (value_lo as u16)
    }
}
