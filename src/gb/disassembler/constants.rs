use crate::gb::disassembler::{Address, Label, Symbol, SymbolType};
use crate::gb::registers::{ConditionCode, Reg};
use std::collections::HashMap;
use std::string::ToString;
use std::sync::LazyLock;

// Disassembly tables
pub const TABLE_R: [Reg; 8] = [
    Reg::B,
    Reg::C,
    Reg::D,
    Reg::E,
    Reg::H,
    Reg::L,
    Reg::HL,
    Reg::A,
];
pub const TABLE_RP: [Reg; 4] = [Reg::BC, Reg::DE, Reg::HL, Reg::SP];
pub const TABLE_RP2: [Reg; 4] = [Reg::BC, Reg::DE, Reg::HL, Reg::AF];
pub const TABLE_CC: [ConditionCode; 4] = [
    ConditionCode::NZ,
    ConditionCode::Z,
    ConditionCode::NC,
    ConditionCode::C,
];

// Initial set of sections from which to start disassembling
pub static INITIAL_SYMBOLS: LazyLock<[Symbol; 26]> = LazyLock::new(|| {
    [
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0000,
                },
                name: "RST_00".to_string(),
            },
            symbol_type: SymbolType::Code,
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0008,
                },
                name: "RST_08".to_string(),
            },
            symbol_type: SymbolType::Code,
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0010,
                },
                name: "RST_10".to_string(),
            },
            symbol_type: SymbolType::Code,
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0018,
                },
                name: "RST_18".to_string(),
            },
            symbol_type: SymbolType::Code,
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0020,
                },
                name: "RST_20".to_string(),
            },
            symbol_type: SymbolType::Code,
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0028,
                },
                name: "RST_28".to_string(),
            },
            symbol_type: SymbolType::Code,
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0030,
                },
                name: "RST_30".to_string(),
            },
            symbol_type: SymbolType::Code,
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0038,
                },
                name: "RST_38".to_string(),
            },
            symbol_type: SymbolType::Code,
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0040,
                },
                name: "VBlankInterrupt".to_string(),
            },
            symbol_type: SymbolType::Code,
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0048,
                },
                name: "LCDCInterrupt".to_string(),
            },
            symbol_type: SymbolType::Code,
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0050,
                },
                name: "TimerOverflowInterrupt".to_string(),
            },
            symbol_type: SymbolType::Code,
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0058,
                },
                name: "SerialTransferCompleteInterrupt".to_string(),
            },
            symbol_type: SymbolType::Code,
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0060,
                },
                name: "JoypadTransitionInterrupt".to_string(),
            },
            symbol_type: SymbolType::Code,
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0100,
                },
                name: "Boot".to_string(),
            },
            symbol_type: SymbolType::Code,
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0104,
                },
                name: "HeaderLogo".to_string(),
            },
            symbol_type: SymbolType::Data(0x30),
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0134,
                },
                name: "HeaderTitle".to_string(),
            },
            symbol_type: SymbolType::Text(0x10),
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0144,
                },
                name: "HeaderNewLicenseeCode".to_string(),
            },
            symbol_type: SymbolType::Text(0x2),
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0146,
                },
                name: "HeaderSGBFlag".to_string(),
            },
            symbol_type: SymbolType::Data(0x1),
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0147,
                },
                name: "HeaderCartridgeType".to_string(),
            },
            symbol_type: SymbolType::Data(0x1),
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0148,
                },
                name: "HeaderROMSize".to_string(),
            },
            symbol_type: SymbolType::Data(0x1),
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x0149,
                },
                name: "HeaderRAMSize".to_string(),
            },
            symbol_type: SymbolType::Data(1),
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x014a,
                },
                name: "HeaderDestinationCode".to_string(),
            },
            symbol_type: SymbolType::Data(0x1),
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x014b,
                },
                name: "HeaderOldLicenseeCode".to_string(),
            },
            symbol_type: SymbolType::Data(0x1),
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x014c,
                },
                name: "HeaderMaskROMVersion".to_string(),
            },
            symbol_type: SymbolType::Data(0x1),
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x014d,
                },
                name: "HeaderComplementCheck".to_string(),
            },
            symbol_type: SymbolType::Data(1),
        },
        Symbol {
            label: Label {
                address: Address {
                    bank: 0,
                    address: 0x014e,
                },
                name: "HeaderGlobalChecksum".to_string(),
            },
            symbol_type: SymbolType::Data(0x2),
        },
    ]
});

// Hardware registers map
pub static HARDWARE_REGISTERS: LazyLock<HashMap<u16, &str>> = LazyLock::new(|| {
    HashMap::from([
        (0xFF00, "rP1"),
        (0xFF01, "rSB"),
        (0xFF02, "rSC"),
        (0xFF04, "rDIV"),
        (0xFF05, "rTIMA"),
        (0xFF06, "rTMA"),
        (0xFF07, "rTAC"),
        (0xFF0F, "rIF"),
        (0xFF10, "rAUD1SWEEP"),
        (0xFF11, "rAUD1LEN"),
        (0xFF12, "rAUD1ENV"),
        (0xFF13, "rAUD1LOW"),
        (0xFF14, "rAUD1HIGH"),
        (0xFF16, "rAUD2LEN"),
        (0xFF17, "rAUD2ENV"),
        (0xFF18, "rAUD2LOW"),
        (0xFF19, "rAUD2HIGH"),
        (0xFF1A, "rAUD3ENA"),
        (0xFF1B, "rAUD3LEN"),
        (0xFF1C, "rAUD3LEVEL"),
        (0xFF1D, "rAUD3LOW"),
        (0xFF1E, "rAUD3HIGH"),
        (0xFF20, "rAUD4LEN"),
        (0xFF21, "rAUD4LEN"),
        (0xFF22, "rAUD4POLY"),
        (0xFF23, "rAUD4GO"),
        (0xFF24, "rAUDVOL"),
        (0xFF25, "rAUDTERM"),
        (0xFF26, "rAUDENA"),
        (0xFF40, "rLCDC"),
        (0xFF41, "rSTAT"),
        (0xFF42, "rSCY"),
        (0xFF43, "rSCX"),
        (0xFF44, "rLY"),
        (0xFF45, "rLYC"),
        (0xFF46, "rDMA"),
        (0xFF47, "rBGP"),
        (0xFF48, "rOBP0"),
        (0xFF49, "rOBP1"),
        (0xFF4A, "rWY"),
        (0xFF4B, "rWX"),
        (0xFF4D, "rSPD"),
        (0xFF4F, "rVBK"),
        (0xFF51, "rHDMA1"),
        (0xFF52, "rHDMA2"),
        (0xFF53, "rHDMA3"),
        (0xFF54, "rHDMA4"),
        (0xFF55, "rHDMA5"),
        (0xFF56, "rRP"),
        (0xFF68, "rBGPI"),
        (0xFF69, "rBGPD"),
        (0xFF6A, "rOBPI"),
        (0xFF6B, "rOBPD"),
        (0xFF6C, "rOPRI"),
        (0xFF70, "rSMBK"),
        (0xFF76, "rPCM12"),
        (0xFF77, "rPCM34"),
        (0xFFFF, "rIE"),
    ])
});
