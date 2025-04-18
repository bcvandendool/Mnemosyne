use image::{GenericImageView, ImageReader};
use Mnemosyne::gb::GameBoy;

pub(crate) fn setup(rom: &str) -> GameBoy {
    let mut gameboy = GameBoy::new();
    gameboy.load_rom(rom);
    gameboy.skip_boot_rom();
    gameboy
}

mod cpu_instrs {
    use crate::setup;

    #[test]
    fn test_01() {
        let mut gameboy = setup(
            "./tests/game-boy-test-roms/artifacts/blargg/cpu_instrs/individual/01-special.gb",
        );

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
        let mut gameboy = setup(
            "./tests/game-boy-test-roms/artifacts/blargg/cpu_instrs/individual/02-interrupts.gb",
        );

        for _ in 0..200000 {
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
        let mut gameboy = setup(
            "./tests/game-boy-test-roms/artifacts/blargg/cpu_instrs/individual/03-op sp,hl.gb",
        );

        for _ in 0..1250000 {
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
        let mut gameboy = setup(
            "./tests/game-boy-test-roms/artifacts/blargg/cpu_instrs/individual/04-op r,imm.gb",
        );

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
        let mut gameboy =
            setup("./tests/game-boy-test-roms/artifacts/blargg/cpu_instrs/individual/05-op rp.gb");

        for _ in 0..2000000 {
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
        let mut gameboy =
            setup("./tests/game-boy-test-roms/artifacts/blargg/cpu_instrs/individual/06-ld r,r.gb");

        for _ in 0..300000 {
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
        let mut gameboy = setup(
            "./tests/game-boy-test-roms/artifacts/blargg/cpu_instrs/individual/07-jr,jp,call,ret,rst.gb",
        );

        for _ in 0..400000 {
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
        let mut gameboy = setup(
            "./tests/game-boy-test-roms/artifacts/blargg/cpu_instrs/individual/08-misc instrs.gb",
        );

        for _ in 0..260000 {
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
        let mut gameboy =
            setup("./tests/game-boy-test-roms/artifacts/blargg/cpu_instrs/individual/09-op r,r.gb");

        for _ in 0..5000000 {
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
        let mut gameboy = setup(
            "./tests/game-boy-test-roms/artifacts/blargg/cpu_instrs/individual/10-bit ops.gb",
        );

        for _ in 0..7000000 {
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
        let mut gameboy = setup(
            "./tests/game-boy-test-roms/artifacts/blargg/cpu_instrs/individual/11-op a,(hl).gb",
        );

        for _ in 0..8000000 {
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

#[test]
fn test_oam_bug() {
    let mut gameboy = setup("./tests/game-boy-test-roms/artifacts/blargg/oam_bug/oam_bug.gb");

    let mut cycles = 0;
    while cycles < (21.0 * 4194304.0 / 4.0) as u64 {
        let (hit_breakpoint_now, cycles_spent) = gameboy.tick();
        cycles += cycles_spent as u64;
    }
    let mut output_img = vec![0; 160 * 144 * 4];
    let frame_buffer = gameboy.get_framebuffer();

    for idx in 0..(160 * 144) {
        let color: u8 = match frame_buffer[idx] {
            0 => 0xFF,
            1 => 0xAA,
            2 => 0x55,
            3 => 0x00,
            _ => panic!("Received invalid color code"),
        };
        output_img[idx * 4] = color;
        output_img[idx * 4 + 1] = color;
        output_img[idx * 4 + 2] = color;
        output_img[idx * 4 + 3] = 0xFF;
    }

    let img =
        ImageReader::open("./tests/game-boy-test-roms/artifacts/blargg/oam_bug/oam_bug-dmg.png")
            .unwrap()
            .decode()
            .unwrap();
    let test = img.pixels().flat_map(|a| a.2.0).collect::<Vec<u8>>();

    assert_eq!(output_img, test);
}

#[test]
fn test_dmg_sound() {
    let mut gameboy = setup("./tests/game-boy-test-roms/artifacts/blargg/dmg_sound/dmg_sound.gb");

    let mut cycles = 0;
    while cycles < (37.0 * 4194304.0 / 4.0) as u64 {
        let (hit_breakpoint_now, cycles_spent) = gameboy.tick();
        cycles += cycles_spent as u64;
    }
    let mut output_img = vec![0; 160 * 144 * 4];
    let frame_buffer = gameboy.get_framebuffer();

    for idx in 0..(160 * 144) {
        let color: u8 = match frame_buffer[idx] {
            0 => 0xFF,
            1 => 0xAA,
            2 => 0x55,
            3 => 0x00,
            _ => panic!("Received invalid color code"),
        };
        output_img[idx * 4] = color;
        output_img[idx * 4 + 1] = color;
        output_img[idx * 4 + 2] = color;
        output_img[idx * 4 + 3] = 0xFF;
    }

    let img = ImageReader::open(
        "./tests/game-boy-test-roms/artifacts/blargg/dmg_sound/dmg_sound-dmg.png",
    )
    .unwrap()
    .decode()
    .unwrap();
    let test = img.pixels().flat_map(|a| a.2.0).collect::<Vec<u8>>();

    assert_eq!(output_img, test);
}

mod mem_timing {
    use crate::setup;
    use image::{GenericImageView, ImageReader};

    #[test]
    fn test_v1() {
        let mut gameboy =
            setup("./tests/game-boy-test-roms/artifacts/blargg/mem_timing/mem_timing.gb");

        for _ in 0..700000 {
            gameboy.tick();
        }
        let mut output_img = vec![0; 160 * 144 * 4];
        let frame_buffer = gameboy.get_framebuffer();

        for idx in 0..(160 * 144) {
            let color: u8 = match frame_buffer[idx] {
                0 => 0xFF,
                1 => 0xAA,
                2 => 0x55,
                3 => 0x00,
                _ => panic!("Received invalid color code"),
            };
            output_img[idx * 4] = color;
            output_img[idx * 4 + 1] = color;
            output_img[idx * 4 + 2] = color;
            output_img[idx * 4 + 3] = 0xFF;
        }

        let img = ImageReader::open(
            "./tests/game-boy-test-roms/artifacts/blargg/mem_timing/mem_timing-dmg-cgb.png",
        )
        .unwrap()
        .decode()
        .unwrap();
        let test = img.pixels().flat_map(|a| a.2.0).collect::<Vec<u8>>();

        assert_eq!(output_img, test);
    }

    #[test]
    fn test_v2() {
        let mut gameboy =
            setup("./tests/game-boy-test-roms/artifacts/blargg/mem_timing-2/mem_timing.gb");

        for _ in 0..1250000 {
            gameboy.tick();
        }
        let mut output_img = vec![0; 160 * 144 * 4];
        let frame_buffer = gameboy.get_framebuffer();

        for idx in 0..(160 * 144) {
            let color: u8 = match frame_buffer[idx] {
                0 => 0xFF,
                1 => 0xAA,
                2 => 0x55,
                3 => 0x00,
                _ => panic!("Received invalid color code"),
            };
            output_img[idx * 4] = color;
            output_img[idx * 4 + 1] = color;
            output_img[idx * 4 + 2] = color;
            output_img[idx * 4 + 3] = 0xFF;
        }

        let img = ImageReader::open(
            "./tests/game-boy-test-roms/artifacts/blargg/mem_timing-2/mem_timing-dmg-cgb.png",
        )
        .unwrap()
        .decode()
        .unwrap();
        let test = img.pixels().flat_map(|a| a.2.0).collect::<Vec<u8>>();

        assert_eq!(output_img, test);
    }
}

#[test]
fn test_instr_timing() {
    let mut gameboy =
        setup("./tests/game-boy-test-roms/artifacts/blargg/instr_timing/instr_timing.gb");

    for _ in 0..300000 {
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
fn test_halt_bug() {
    let mut gameboy = setup("./tests/game-boy-test-roms/artifacts/blargg/halt_bug.gb");

    let mut cycles = 0;
    while cycles < (2.0 * 4194304.0 / 4.0) as u64 {
        let (hit_breakpoint_now, cycles_spent) = gameboy.tick();
        cycles += cycles_spent as u64;
    }
    let mut output_img = vec![0; 160 * 144 * 4];
    let frame_buffer = gameboy.get_framebuffer();

    for idx in 0..(160 * 144) {
        let color: u8 = match frame_buffer[idx] {
            0 => 0xFF,
            1 => 0xAA,
            2 => 0x55,
            3 => 0x00,
            _ => panic!("Received invalid color code"),
        };
        output_img[idx * 4] = color;
        output_img[idx * 4 + 1] = color;
        output_img[idx * 4 + 2] = color;
        output_img[idx * 4 + 3] = 0xFF;
    }

    let img = ImageReader::open("./tests/game-boy-test-roms/artifacts/blargg/halt_bug-dmg-cgb.png")
        .unwrap()
        .decode()
        .unwrap();
    let test = img.pixels().flat_map(|a| a.2.0).collect::<Vec<u8>>();

    assert_eq!(output_img, test);
}
