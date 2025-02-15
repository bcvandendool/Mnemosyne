use Mnemosyne::gameboy::GameBoy;

mod common;

fn setup(rom: &str) -> GameBoy {
    let mut gameboy = GameBoy::new();
    gameboy.load_rom(rom);
    gameboy.skip_boot_rom();
    gameboy
}

mod cpu_instrs {
    use crate::setup;

    #[test]
    fn test_01() {
        let mut gameboy = setup("../../tests/gb-test-roms/cpu_instrs/individual/01-special.gb");

        for _ in 0..1500000 {
            gameboy.tick();
        }
        let serial_data = gameboy.serial_buffer();
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            '0', '1', '-', 's', 'p', 'e', 'c', 'i', 'a', 'l', '\n', '\n', '\n', 'P', 'a', 's', 's',
            'e', 'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_02() {
        let mut gameboy = setup("../../tests/gb-test-roms/cpu_instrs/individual/02-interrupts.gb");

        for _ in 0..1500000 {
            gameboy.tick();
        }
        let serial_data = gameboy.serial_buffer();
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            '0', '2', '-', 'i', 'n', 't', 'e', 'r', 'r', 'u', 'p', 't', 's', '\n', '\n', '\n', 'P',
            'a', 's', 's', 'e', 'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_03() {
        let mut gameboy = setup("../../tests/gb-test-roms/cpu_instrs/individual/03-op sp,hl.gb");

        for _ in 0..1500000 {
            gameboy.tick();
        }
        let serial_data = gameboy.serial_buffer();
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            '0', '3', '-', 'o', 'p', ' ', 's', 'p', ',', 'h', 'l', '\n', '\n', '\n', 'P', 'a', 's',
            's', 'e', 'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_04() {
        let mut gameboy = setup("../../tests/gb-test-roms/cpu_instrs/individual/04-op r,imm.gb");

        for _ in 0..1500000 {
            gameboy.tick();
        }
        let serial_data = gameboy.serial_buffer();
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            '0', '4', '-', 'o', 'p', ' ', 'r', ',', 'i', 'm', 'm', '\n', '\n', '\n', 'P', 'a', 's',
            's', 'e', 'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_05() {
        let mut gameboy = setup("../../tests/gb-test-roms/cpu_instrs/individual/05-op rp.gb");

        for _ in 0..3000000 {
            gameboy.tick();
        }
        let serial_data = gameboy.serial_buffer();
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            '0', '5', '-', 'o', 'p', ' ', 'r', 'p', '\n', '\n', '\n', 'P', 'a', 's', 's', 'e', 'd',
            '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_06() {
        let mut gameboy = setup("../../tests/gb-test-roms/cpu_instrs/individual/06-ld r,r.gb");

        for _ in 0..1500000 {
            gameboy.tick();
        }
        let serial_data = gameboy.serial_buffer();
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            '0', '6', '-', 'l', 'd', ' ', 'r', ',', 'r', '\n', '\n', '\n', 'P', 'a', 's', 's', 'e',
            'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_07() {
        let mut gameboy =
            setup("../../tests/gb-test-roms/cpu_instrs/individual/07-jr,jp,call,ret,rst.gb");

        for _ in 0..1500000 {
            gameboy.tick();
        }
        let serial_data = gameboy.serial_buffer();
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            '0', '7', '-', 'j', 'r', ',', 'j', 'p', ',', 'c', 'a', 'l', 'l', ',', 'r', 'e', 't',
            ',', 'r', 's', 't', '\n', '\n', '\n', 'P', 'a', 's', 's', 'e', 'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_08() {
        let mut gameboy = setup("../../tests/gb-test-roms/cpu_instrs/individual/08-misc instrs.gb");

        for _ in 0..1500000 {
            gameboy.tick();
        }
        let serial_data = gameboy.serial_buffer();
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            '0', '8', '-', 'm', 'i', 's', 'c', ' ', 'i', 'n', 's', 't', 'r', 's', '\n', '\n', '\n',
            'P', 'a', 's', 's', 'e', 'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_09() {
        let mut gameboy = setup("../../tests/gb-test-roms/cpu_instrs/individual/09-op r,r.gb");

        for _ in 0..6000000 {
            gameboy.tick();
        }
        let serial_data = gameboy.serial_buffer();
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            '0', '9', '-', 'o', 'p', ' ', 'r', ',', 'r', '\n', '\n', '\n', 'P', 'a', 's', 's', 'e',
            'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_10() {
        let mut gameboy = setup("../../tests/gb-test-roms/cpu_instrs/individual/10-bit ops.gb");

        for _ in 0..12000000 {
            gameboy.tick();
        }
        let serial_data = gameboy.serial_buffer();
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            '1', '0', '-', 'b', 'i', 't', ' ', 'o', 'p', 's', '\n', '\n', '\n', 'P', 'a', 's', 's',
            'e', 'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_11() {
        let mut gameboy = setup("../../tests/gb-test-roms/cpu_instrs/individual/11-op a,(hl).gb");

        for _ in 0..12000000 {
            gameboy.tick();
        }
        let serial_data = gameboy.serial_buffer();
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            '1', '1', '-', 'o', 'p', ' ', 'a', ',', '(', 'h', 'l', ')', '\n', '\n', '\n', 'P', 'a',
            's', 's', 'e', 'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }
}

mod mem_timing {
    use crate::setup;

    #[test]
    fn test_01() {
        let mut gameboy = setup("../../tests/gb-test-roms/mem_timing/individual/01-read_timing.gb");

        for _ in 0..12000000 {
            gameboy.tick();
        }
        let serial_data = gameboy.serial_buffer();
        println!("{:?}", serial_data);
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            'i', 'n', 's', 't', 'r', '_', 't', 'i', 'm', 'i', 'n', 'g', '\n', '\n', '\n', 'P', 'a',
            's', 's', 'e', 'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_02() {
        let mut gameboy =
            setup("../../tests/gb-test-roms/mem_timing/individual/02-write_timing.gb");

        for _ in 0..12000000 {
            gameboy.tick();
        }
        let serial_data = gameboy.serial_buffer();
        println!("{:?}", serial_data);
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            'i', 'n', 's', 't', 'r', '_', 't', 'i', 'm', 'i', 'n', 'g', '\n', '\n', '\n', 'P', 'a',
            's', 's', 'e', 'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_03() {
        let mut gameboy =
            setup("../../tests/gb-test-roms/mem_timing/individual/03-modify_timing.gb");

        for _ in 0..12000000 {
            gameboy.tick();
        }
        let serial_data = gameboy.serial_buffer();
        println!("{:?}", serial_data);
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            'i', 'n', 's', 't', 'r', '_', 't', 'i', 'm', 'i', 'n', 'g', '\n', '\n', '\n', 'P', 'a',
            's', 's', 'e', 'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_01_v2() {
        let mut gameboy =
            setup("../../tests/gb-test-roms/mem_timing-2/rom_singles/01-read_timing.gb");

        for _ in 0..12000000 {
            gameboy.tick();
        }
        // TODO: implement in a different way, does not seem to generate serial output
        let serial_data = gameboy.serial_buffer();
        println!("{:?}", serial_data);
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            'i', 'n', 's', 't', 'r', '_', 't', 'i', 'm', 'i', 'n', 'g', '\n', '\n', '\n', 'P', 'a',
            's', 's', 'e', 'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_02_v2() {
        let mut gameboy =
            setup("../../tests/gb-test-roms/mem_timing-2/rom_singles/02-write_timing.gb");

        for _ in 0..12000000 {
            gameboy.tick();
        }
        // TODO: implement in a different way, does not seem to generate serial output
        let serial_data = gameboy.serial_buffer();
        println!("{:?}", serial_data);
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            'i', 'n', 's', 't', 'r', '_', 't', 'i', 'm', 'i', 'n', 'g', '\n', '\n', '\n', 'P', 'a',
            's', 's', 'e', 'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }

    #[test]
    fn test_03_v2() {
        let mut gameboy =
            setup("../../tests/gb-test-roms/mem_timing-2/rom_singles/03-modify_timing.gb");

        for _ in 0..12000000 {
            gameboy.tick();
        }
        // TODO: implement in a different way, does not seem to generate serial output
        let serial_data = gameboy.serial_buffer();
        println!("{:?}", serial_data);
        println!("{:}", serial_data.iter().collect::<String>());
        let expected_data = [
            'i', 'n', 's', 't', 'r', '_', 't', 'i', 'm', 'i', 'n', 'g', '\n', '\n', '\n', 'P', 'a',
            's', 's', 'e', 'd', '\n',
        ];
        assert!(serial_data.eq(&expected_data));
    }
}

#[test]
fn test_instr_timing() {
    let mut gameboy = setup("../../tests/gb-test-roms/instr_timing/instr_timing.gb");

    for _ in 0..12000000 {
        gameboy.tick();
    }
    let serial_data = gameboy.serial_buffer();
    println!("{:}", serial_data.iter().collect::<String>());
    let expected_data = [
        'i', 'n', 's', 't', 'r', '_', 't', 'i', 'm', 'i', 'n', 'g', '\n', '\n', '\n', 'P', 'a',
        's', 's', 'e', 'd', '\n',
    ];
    assert!(serial_data.eq(&expected_data));
}

#[test]
fn test_interrupt_time() {
    let mut gameboy = setup("../../tests/gb-test-roms/interrupt_time/interrupt_time.gb");

    for _ in 0..12000000 {
        gameboy.tick();
    }
    // TODO: implement in a different way, does not seem to generate serial output
    let serial_data = gameboy.serial_buffer();
    println!("{:?}", serial_data);
    println!("{:}", serial_data.iter().collect::<String>());
    let expected_data = [
        'i', 'n', 's', 't', 'r', '_', 't', 'i', 'm', 'i', 'n', 'g', '\n', '\n', '\n', 'P', 'a',
        's', 's', 'e', 'd', '\n',
    ];
    assert!(serial_data.eq(&expected_data));
}

#[test]
fn test_halt_bug() {
    let mut gameboy = setup("../../tests/gb-test-roms/halt_bug.gb");

    for _ in 0..12000000 {
        gameboy.tick();
    }
    // TODO: implement in a different way, does not seem to generate serial output
    let serial_data = gameboy.serial_buffer();
    println!("{:}", serial_data.iter().collect::<String>());
    let expected_data = [
        '1', '1', '-', 'o', 'p', ' ', 'a', ',', '(', 'h', 'l', ')', '\n', '\n', '\n', 'P', 'a',
        's', 's', 'e', 'd', '\n',
    ];
    assert!(serial_data.eq(&expected_data));
}
