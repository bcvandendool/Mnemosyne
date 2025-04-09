use crate::gb::disassembler::constants::HARDWARE_REGISTERS;
use crate::gb::registers::{ConditionCode, Reg};
use std::fmt::{Display, Formatter};

#[derive(PartialEq, Clone)]
pub enum Instruction {
    // Load instructions
    LD_r8_r8(Reg, Reg),
    LD_r8_n8(Reg, u8),
    LD_r16_n16(Reg, u16),
    LD_HL_r8(Reg),
    LD_HL_n8(u8),
    LD_r8_HL(Reg),
    LD_r16_A(Reg),
    LD_n16_A(u16),
    LDH_n16_A(u16),
    LDH_C_A,
    LD_A_r16(Reg),
    LD_A_n16(u16),
    LDH_A_n16(u16),
    LDH_A_C,
    LD_HLI_A,
    LD_HLD_A,
    LD_A_HLI,
    LD_A_HLD,
    // 8-bit arithmetic instructions
    ADC_A_r8(Reg),
    ADC_A_HL,
    ADC_A_n8(u8),
    ADD_A_r8(Reg),
    ADD_A_HL,
    ADD_A_n8(u8),
    CP_A_r8(Reg),
    CP_A_HL,
    CP_A_n8(u8),
    DEC_r8(Reg),
    DEC_HL,
    INC_r8(Reg),
    INC_HL,
    SBC_A_r8(Reg),
    SBC_A_HL,
    SBC_A_n8(u8),
    SUB_A_r8(Reg),
    SUB_A_HL,
    SUB_A_n8(u8),
    // 16-bit arithmetic instructions
    ADD_HL_r16(Reg),
    DEC_r16(Reg),
    INC_r16(Reg),
    // Bitwise logic instructions
    AND_A_r8(Reg),
    AND_A_HL,
    AND_A_n8(u8),
    CPL,
    OR_A_r8(Reg),
    OR_A_HL,
    OR_A_n8(u8),
    XOR_A_r8(Reg),
    XOR_A_HL,
    XOR_A_n8(u8),
    // Bit flag instructions
    BIT_u3_r8(u8, Reg),
    BIT_u3_HL(u8),
    RES_u3_r8(u8, Reg),
    RES_u3_HL(u8),
    SET_u3_r8(u8, Reg),
    SET_u3_HL(u8),
    // Bit shift instructions
    RL_r8(Reg),
    RL_HL,
    RLA,
    RLC_r8(Reg),
    RLC_HL,
    RLCA,
    RR_r8(Reg),
    RR_HL,
    RRA,
    RRC_r8(Reg),
    RRC_HL,
    RRCA,
    SLA_r8(Reg),
    SLA_HL,
    SRA_r8(Reg),
    SRA_HL,
    SRL_r8(Reg),
    SRL_HL,
    SWAP_r8(Reg),
    SWAP_HL,
    // Jumps and subroutine instructions
    CALL_n16(u16),
    CALL_cc_n16(ConditionCode, u16),
    JP_HL,
    JP_n16(u16),
    JP_cc_n16(ConditionCode, u16),
    JR_n16(u16),
    JR_cc_n16(ConditionCode, u16),
    RET_cc(ConditionCode),
    RET,
    RETI,
    RST_vec(u8),
    // Carry flag instructions
    CCF,
    SCF,
    // Stack manipulation instructions
    ADD_HL_SP,
    ADD_SP_e8(i8),
    DEC_SP,
    INC_SP,
    LD_SP_n16(u16),
    LD_n16_SP(u16),
    LD_HL_SP_e8(i8),
    LD_SP_HL,
    POP_AF,
    POP_r16(Reg),
    PUSH_AF,
    PUSH_r16(Reg),
    // Interrupt-related instructions
    DI,
    EI,
    HALT,
    // Miscellaneous instructions
    DAA,
    NOP,
    STOP,
    INVALID(u8),
}

impl Instruction {
    pub(crate) fn bytes(&self) -> u16 {
        match self {
            Instruction::CALL_n16(_)
            | Instruction::CALL_cc_n16(_, _)
            | Instruction::JP_n16(_)
            | Instruction::JP_cc_n16(_, _)
            | Instruction::LD_r16_n16(_, _)
            | Instruction::LD_n16_A(_)
            | Instruction::LD_A_n16(_)
            | Instruction::LD_SP_n16(_)
            | Instruction::LD_n16_SP(_) => 3,
            Instruction::ADC_A_n8(_)
            | Instruction::ADD_A_n8(_)
            | Instruction::ADD_SP_e8(_)
            | Instruction::AND_A_n8(_)
            | Instruction::BIT_u3_r8(_, _)
            | Instruction::BIT_u3_HL(_)
            | Instruction::CP_A_n8(_)
            | Instruction::JR_n16(_)
            | Instruction::JR_cc_n16(_, _)
            | Instruction::LD_r8_n8(_, _)
            | Instruction::LD_HL_n8(_)
            | Instruction::LDH_n16_A(_)
            | Instruction::LDH_A_n16(_)
            | Instruction::LD_HL_SP_e8(_)
            | Instruction::OR_A_n8(_)
            | Instruction::RES_u3_r8(_, _)
            | Instruction::RES_u3_HL(_)
            | Instruction::RL_r8(_)
            | Instruction::RL_HL
            | Instruction::RLC_r8(_)
            | Instruction::RLC_HL
            | Instruction::RR_r8(_)
            | Instruction::RR_HL
            | Instruction::RRC_r8(_)
            | Instruction::RRC_HL
            | Instruction::SBC_A_n8(_)
            | Instruction::SET_u3_r8(_, _)
            | Instruction::SET_u3_HL(_)
            | Instruction::SLA_r8(_)
            | Instruction::SLA_HL
            | Instruction::SRA_r8(_)
            | Instruction::SRA_HL
            | Instruction::SRL_r8(_)
            | Instruction::SRL_HL
            | Instruction::SUB_A_n8(_)
            | Instruction::SWAP_r8(_)
            | Instruction::SWAP_HL
            | Instruction::XOR_A_n8(_) => 2,
            _ => 1,
        }
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            // Load instructions
            Instruction::LD_r8_r8(reg_a, reg_b) => {
                write!(f, "LD {}, {}", reg_a, reg_b)
            }
            Instruction::LD_r8_n8(reg, imm) => {
                write!(f, "LD {}, ${:02X}", reg, imm)
            }
            Instruction::LD_r16_n16(reg, imm) => {
                write!(f, "LD {}, ${:04X}", reg, imm)
            }
            Instruction::LD_HL_r8(reg) => {
                write!(f, "LD [HL], {}", reg)
            }
            Instruction::LD_HL_n8(imm) => {
                write!(f, "LD [HL], ${:02X}", imm)
            }
            Instruction::LD_r8_HL(reg) => {
                write!(f, "LD {}, [HL]", reg)
            }
            Instruction::LD_r16_A(reg) => {
                write!(f, "LD [{}], A", reg)
            }
            Instruction::LD_n16_A(imm) => {
                if HARDWARE_REGISTERS.contains_key(imm) {
                    write!(f, "LD [{:}], A", HARDWARE_REGISTERS[imm])
                } else {
                    write!(f, "LD [${:04X}], A", imm)
                }
            }
            Instruction::LDH_n16_A(imm) => {
                if HARDWARE_REGISTERS.contains_key(imm) {
                    write!(f, "LDH [{:}], A", HARDWARE_REGISTERS[imm])
                } else {
                    write!(f, "LDH [${:04X}], A", imm)
                }
            }
            Instruction::LDH_C_A => {
                write!(f, "LDH [C], A")
            }
            Instruction::LD_A_r16(reg) => {
                write!(f, "LD A, [{}]", reg)
            }
            Instruction::LD_A_n16(imm) => {
                if HARDWARE_REGISTERS.contains_key(imm) {
                    write!(f, "LD A, [{:}]", HARDWARE_REGISTERS[imm])
                } else {
                    write!(f, "LD A, [${:04X}]", imm)
                }
            }
            Instruction::LDH_A_n16(imm) => {
                if HARDWARE_REGISTERS.contains_key(imm) {
                    write!(f, "LDH A, [{:}]", HARDWARE_REGISTERS[imm])
                } else {
                    write!(f, "LDH A, [${:04X}]", imm)
                }
            }
            Instruction::LDH_A_C => {
                write!(f, "LDH A, [C]")
            }
            Instruction::LD_HLI_A => {
                write!(f, "LD [HLI], A")
            }
            Instruction::LD_HLD_A => {
                write!(f, "LD [HLD], A")
            }
            Instruction::LD_A_HLI => {
                write!(f, "LD A, [HLI]")
            }
            Instruction::LD_A_HLD => {
                write!(f, "LD A, [HLD]")
            }
            // 8-bit arithmetic instructions
            Instruction::ADC_A_r8(reg) => {
                write!(f, "ADC A, {}", reg)
            }
            Instruction::ADC_A_HL => {
                write!(f, "ADC A, [HL]")
            }
            Instruction::ADC_A_n8(imm) => {
                write!(f, "ADC A, ${:02X}", imm)
            }
            Instruction::ADD_A_r8(reg) => {
                write!(f, "ADD A, {}", reg)
            }
            Instruction::ADD_A_HL => {
                write!(f, "ADD A, [HL]")
            }
            Instruction::ADD_A_n8(imm) => {
                write!(f, "ADD A, ${:02X}", imm)
            }
            Instruction::CP_A_r8(reg) => {
                write!(f, "CP A, {}", reg)
            }
            Instruction::CP_A_HL => {
                write!(f, "CP A, [HL]")
            }
            Instruction::CP_A_n8(imm) => {
                write!(f, "CP A, ${:02X}", imm)
            }
            Instruction::DEC_r8(reg) => {
                write!(f, "DEC {}", reg)
            }
            Instruction::DEC_HL => {
                write!(f, "DEC [HL]")
            }
            Instruction::INC_r8(reg) => {
                write!(f, "INC {}", reg)
            }
            Instruction::INC_HL => {
                write!(f, "INC [HL]")
            }
            Instruction::SBC_A_r8(reg) => {
                write!(f, "SBC A, {}", reg)
            }
            Instruction::SBC_A_HL => {
                write!(f, "SBC A, [HL]")
            }
            Instruction::SBC_A_n8(imm) => {
                write!(f, "SBC A, ${:02X}", imm)
            }
            Instruction::SUB_A_r8(reg) => {
                write!(f, "SUB A, {}", reg)
            }
            Instruction::SUB_A_HL => {
                write!(f, "SUB A, [HL]")
            }
            Instruction::SUB_A_n8(imm) => {
                write!(f, "SUB A, ${:02X}", imm)
            }
            // 16-bit arithmetic instructions
            Instruction::ADD_HL_r16(reg) => {
                write!(f, "ADD HL, {}", reg)
            }
            Instruction::DEC_r16(reg) => {
                write!(f, "DEC {}", reg)
            }
            Instruction::INC_r16(reg) => {
                write!(f, "INC {}", reg)
            }
            // Bitwise logic instructions
            Instruction::AND_A_r8(reg) => {
                write!(f, "AND A, {}", reg)
            }
            Instruction::AND_A_HL => {
                write!(f, "AND A, [HL]")
            }
            Instruction::AND_A_n8(imm) => {
                write!(f, "AND A, ${:02X}", imm)
            }
            Instruction::CPL => {
                write!(f, "CPL")
            }
            Instruction::OR_A_r8(reg) => {
                write!(f, "OR A, {}", reg)
            }
            Instruction::OR_A_HL => {
                write!(f, "OR A, [HL]")
            }
            Instruction::OR_A_n8(imm) => {
                write!(f, "OR A, ${:02X}", imm)
            }
            Instruction::XOR_A_r8(reg) => {
                write!(f, "XOR A, {}", reg)
            }
            Instruction::XOR_A_HL => {
                write!(f, "XOR A, [HL]")
            }
            Instruction::XOR_A_n8(imm) => {
                write!(f, "XOR A, ${:02X}", imm)
            }
            // Bit flag instructions
            Instruction::BIT_u3_r8(bit, reg) => {
                write!(f, "BIT {}, {}", bit, reg)
            }
            Instruction::BIT_u3_HL(bit) => {
                write!(f, "BIT {}, [HL]", bit)
            }
            Instruction::RES_u3_r8(bit, reg) => {
                write!(f, "RES {}, {}", bit, reg)
            }
            Instruction::RES_u3_HL(bit) => {
                write!(f, "RES {}, [HL]", bit)
            }
            Instruction::SET_u3_r8(bit, reg) => {
                write!(f, "SET {}, {}", bit, reg)
            }
            Instruction::SET_u3_HL(bit) => {
                write!(f, "SET {}, [HL]", bit)
            }
            // Bit shift instructions
            Instruction::RL_r8(reg) => {
                write!(f, "RL {}", reg)
            }
            Instruction::RL_HL => {
                write!(f, "RL [HL]")
            }
            Instruction::RLA => {
                write!(f, "RLA")
            }
            Instruction::RLC_r8(reg) => {
                write!(f, "RLC {}", reg)
            }
            Instruction::RLC_HL => {
                write!(f, "RLC [HL]")
            }
            Instruction::RLCA => {
                write!(f, "RLCA")
            }
            Instruction::RR_r8(reg) => {
                write!(f, "RR {}", reg)
            }
            Instruction::RR_HL => {
                write!(f, "RR [HL]")
            }
            Instruction::RRA => {
                write!(f, "RRA")
            }
            Instruction::RRC_r8(reg) => {
                write!(f, "RRC {}", reg)
            }
            Instruction::RRC_HL => {
                write!(f, "RRC [HL]")
            }
            Instruction::RRCA => {
                write!(f, "RRCA")
            }
            Instruction::SLA_r8(reg) => {
                write!(f, "SLA {}", reg)
            }
            Instruction::SLA_HL => {
                write!(f, "SLA [HL]")
            }
            Instruction::SRA_r8(reg) => {
                write!(f, "SRA {}", reg)
            }
            Instruction::SRA_HL => {
                write!(f, "SRA [HL]")
            }
            Instruction::SRL_r8(reg) => {
                write!(f, "SRL {}", reg)
            }
            Instruction::SRL_HL => {
                write!(f, "SRL [HL]")
            }
            Instruction::SWAP_r8(reg) => {
                write!(f, "SWAP {}", reg)
            }
            Instruction::SWAP_HL => {
                write!(f, "SWAP [HL]")
            }
            // Jumps and subroutine instructions
            Instruction::CALL_n16(imm) => {
                write!(f, "CALL ${:04X}", imm)
            }
            Instruction::CALL_cc_n16(cc, imm) => {
                write!(f, "CALL {}, ${:04X}", cc, imm)
            }
            Instruction::JP_HL => {
                write!(f, "JP [HL]")
            }
            Instruction::JP_n16(imm) => {
                write!(f, "JP ${:04X}", imm)
            }
            Instruction::JP_cc_n16(cc, imm) => {
                write!(f, "JP {}, ${:04X}", cc, imm)
            }
            Instruction::JR_n16(imm) => {
                write!(f, "JR ${:04X}", imm)
            }
            Instruction::JR_cc_n16(cc, imm) => {
                write!(f, "JR {}, ${:04X}", cc, imm)
            }
            Instruction::RET_cc(cc) => {
                write!(f, "RET {}", cc)
            }
            Instruction::RET => {
                write!(f, "RET")
            }
            Instruction::RETI => {
                write!(f, "RETI")
            }
            Instruction::RST_vec(imm) => {
                write!(f, "RST ${:02X}", imm)
            }
            // Carry flag instructions
            Instruction::CCF => {
                write!(f, "CCF")
            }
            Instruction::SCF => {
                write!(f, "SCF")
            }
            // Stack manipulation instructions
            Instruction::ADD_HL_SP => {
                write!(f, "ADD HL, SP")
            }
            Instruction::ADD_SP_e8(imm) => {
                write!(f, "ADD SP, {}", imm)
            }
            Instruction::DEC_SP => {
                write!(f, "DEC SP")
            }
            Instruction::INC_SP => {
                write!(f, "INC SP")
            }
            Instruction::LD_SP_n16(imm) => {
                write!(f, "LD SP, ${:04X}", imm)
            }
            Instruction::LD_n16_SP(imm) => {
                if HARDWARE_REGISTERS.contains_key(imm) {
                    write!(f, "LD [{:}], SP", HARDWARE_REGISTERS[imm])
                } else {
                    write!(f, "LD [${:04X}], SP", imm)
                }
            }
            Instruction::LD_HL_SP_e8(imm) => {
                write!(f, "LD HL, SP + {}", imm)
            }
            Instruction::LD_SP_HL => {
                write!(f, "LD SP, HL")
            }
            Instruction::POP_AF => {
                write!(f, "POP AF")
            }
            Instruction::POP_r16(reg) => {
                write!(f, "POP {}", reg)
            }
            Instruction::PUSH_AF => {
                write!(f, "PUSH AF")
            }
            Instruction::PUSH_r16(reg) => {
                write!(f, "PUSH {}", reg)
            }
            // Interrupt-related instructions
            Instruction::DI => {
                write!(f, "DI")
            }
            Instruction::EI => {
                write!(f, "EI")
            }
            Instruction::HALT => {
                write!(f, "HALT")
            }
            // Miscellaneous instructions
            Instruction::DAA => {
                write!(f, "DAA")
            }
            Instruction::NOP => {
                write!(f, "NOP")
            }
            Instruction::STOP => {
                write!(f, "STOP")
            }
            Instruction::INVALID(data) => {
                write!(f, "db ${:02X}", data)
            }
        }
    }
}
