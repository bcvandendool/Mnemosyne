use Mnemosyne::gb::GameBoy;

mod test_blargg;
mod test_dmg_acid2;
mod test_mooneye_test_suite;
mod test_mooneye_test_suite_wilbertpol;

fn setup(rom: &str) -> GameBoy {
    let mut gameboy = GameBoy::new();
    gameboy.load_rom(rom);
    gameboy.skip_boot_rom();
    gameboy
}

fn run_mooneye_test(rom: &str) {
    let mut gameboy = GameBoy::new();
    gameboy.load_rom(rom);
    gameboy.skip_boot_rom();

    loop {
        let (breakpoint, _) = gameboy.tick();
        if breakpoint {
            break;
        }
    }

    let register = gameboy.dump_registers();
    assert_eq!(
        register.B, 3,
        "Check register B follows fibonacci sequence: 3"
    );
    assert_eq!(
        register.C, 5,
        "Check register B follows fibonacci sequence: 5"
    );
    assert_eq!(
        register.D, 8,
        "Check register B follows fibonacci sequence: 8"
    );
    assert_eq!(
        register.E, 13,
        "Check register B follows fibonacci sequence: 13"
    );
    assert_eq!(
        register.H, 21,
        "Check register B follows fibonacci sequence: 21"
    );
    assert_eq!(
        register.L, 34,
        "Check register B follows fibonacci sequence: 34"
    );
}
