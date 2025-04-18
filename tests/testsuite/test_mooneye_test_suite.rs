use image::{GenericImageView, ImageReader};
use test_case::test_matrix;
use Mnemosyne::gb::GameBoy;

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
#[test_matrix(["mem_oam", "reg_f", "unused_hwio-GS"])]
fn acceptance_bits(test: &str) {
    run_mooneye_test(
        &("./tests/game-boy-test-roms/artifacts/mooneye-test-suite/acceptance/bits/".to_owned()
            + test
            + ".gb"),
    );
}

#[test_matrix(["daa"])]
fn acceptance_instr(test: &str) {
    run_mooneye_test(
        &("./tests/game-boy-test-roms/artifacts/mooneye-test-suite/acceptance/instr/".to_owned()
            + test
            + ".gb"),
    );
}

#[test_matrix(["ie_push"])]
fn acceptance_interrupts(test: &str) {
    run_mooneye_test(
        &("./tests/game-boy-test-roms/artifacts/mooneye-test-suite/acceptance/interrupts/"
            .to_owned()
            + test
            + ".gb"),
    );
}

#[test_matrix(["basic", "reg_read", "sources-GS"])]
fn acceptance_oam_dma(test: &str) {
    run_mooneye_test(
        &("./tests/game-boy-test-roms/artifacts/mooneye-test-suite/acceptance/oam_dma/".to_owned()
            + test
            + ".gb"),
    );
}

#[test_matrix(["hblank_ly_scx_timing-GS", "intr_1_2_timing-GS", "intr_2_0_timing", "intr_2_mode0_timing", "intr_2_mode0_timing_sprites", "intr_2_mode3_timing", "intr_2_oam_ok_timing", "lcdon_timing-GS", "lcdon_write_timing-GS", "stat_irq_blocking", "stat_lyc_onoff", "vblank_stat_intr-GS"])]
fn acceptance_ppu(test: &str) {
    run_mooneye_test(
        &("./tests/game-boy-test-roms/artifacts/mooneye-test-suite/acceptance/ppu/".to_owned()
            + test
            + ".gb"),
    );
}

#[test_matrix(["boot_sclk_align-dmgABCmgb"])]
fn acceptance_serial(test: &str) {
    run_mooneye_test(
        &("./tests/game-boy-test-roms/artifacts/mooneye-test-suite/acceptance/serial/".to_owned()
            + test
            + ".gb"),
    );
}

#[test_matrix(["div_write", "rapid_toggle", "tim00", "tim00_div_trigger", "tim01", "tim01_div_trigger", "tim10", "tim10_div_trigger", "tim11", "tim11_div_trigger", "tima_reload", "tima_write_reloading", "tma_write_reloading"])]
fn acceptance_timer(test: &str) {
    run_mooneye_test(
        &("./tests/game-boy-test-roms/artifacts/mooneye-test-suite/acceptance/timer/".to_owned()
            + test
            + ".gb"),
    );
}

#[test_matrix(["add_sp_e_timing", "boot_div-dmgABCmgb", "boot_hwio-dmgABCmgb", "boot_regs-dmgABC", "call_cc_timing", "call_cc_timing2", "call_timing", "call_timing2", "di_timing-GS", "div_timing", "ei_sequence", "ei_timing", "halt_ime0_ei", "halt_ime0_nointr_timing", "halt_ime1_timing", "halt_ime1_timing2-GS", "if_ie_registers", "intr_timing", "jp_cc_timing", "jp_timing", "ld_hl_sp_e_timing", "oam_dma_restart", "oam_dma_start", "oam_dma_timing", "pop_timing", "push_timing", "rapid_di_ei", "ret_cc_timing", "ret_timing", "reti_intr_timing", "reti_timing", "rst_timing"])]
fn acceptance(test: &str) {
    run_mooneye_test(
        &("./tests/game-boy-test-roms/artifacts/mooneye-test-suite/acceptance/".to_owned()
            + test
            + ".gb"),
    );
}

#[test_matrix(["bits_bank1", "bits_bank2", "bits_mode", "bits_ramg", "multicart_rom_8Mb", "ram_64kb", "ram_256kb", "rom_1Mb", "rom_2Mb", "rom_4Mb", "rom_8Mb", "rom_16Mb", "rom_512kb"])]
fn emulator_only_mbc1(test: &str) {
    run_mooneye_test(
        &("./tests/game-boy-test-roms/artifacts/mooneye-test-suite/emulator-only/mbc1/".to_owned()
            + test
            + ".gb"),
    );
}

#[test_matrix(["bits_ramg", "bits_romb", "bits_unused", "ram", "rom_1Mb", "rom_2Mb", "rom_512kb"])]
fn emulator_only_mbc2(test: &str) {
    run_mooneye_test(
        &("./tests/game-boy-test-roms/artifacts/mooneye-test-suite/emulator-only/mbc2/".to_owned()
            + test
            + ".gb"),
    );
}

#[test_matrix(["rom_1Mb", "rom_2Mb", "rom_4Mb", "rom_8Mb", "rom_16Mb", "rom_32Mb", "rom_64Mb", "rom_512kb"])]
fn emulator_only_mbc5(test: &str) {
    run_mooneye_test(
        &("./tests/game-boy-test-roms/artifacts/mooneye-test-suite/emulator-only/mbc5/".to_owned()
            + test
            + ".gb"),
    );
}

#[test]
fn manual_only() {
    let mut gameboy = GameBoy::new();
    gameboy.load_rom(
        "./tests/game-boy-test-roms/artifacts/mooneye-test-suite/manual-only/sprite_priority.gb",
    );
    gameboy.skip_boot_rom();

    loop {
        let (breakpoint, _) = gameboy.tick();
        if breakpoint {
            break;
        }
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
            "./tests/game-boy-test-roms/artifacts/mooneye-test-suite/manual-only/sprite_priority-dmg.png",
        )
        .unwrap()
        .decode()
        .unwrap();
    let test = img.pixels().flat_map(|a| a.2.0).collect::<Vec<u8>>();

    assert_eq!(output_img, test);
}
