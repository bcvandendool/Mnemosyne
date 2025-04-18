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
#[test_matrix(["hblank_ly_scx_timing_nops", "hblank_ly_scx_timing_variant_nops", "intr_0_timing", "intr_1_timing",
    "intr_2_mode0_scx1_timing_nops", "intr_2_mode0_scx2_timing_nops", "intr_2_mode0_scx3_timing_nops", "intr_2_mode0_scx4_timing_nops",
    "intr_2_mode0_scx5_timing_nops", "intr_2_mode0_scx6_timing_nops", "intr_2_mode0_scx7_timing_nops", "intr_2_mode0_scx8_timing_nops",
    "intr2_mode0_timing_sprites_nops", "intr2_mode0_timing_sprites_scx1_nops", "intr2_mode0_timing_sprites_scx2_nops",
    "intr2_mode0_timing_sprites_scx4_nops", "intr2_mode0_timing_sprites_scx8_nops", "intr_2_timing", "lcdon_mode_timing",
    "ly00_01_mode0_2", "ly00_mode0_2-GS", "ly00_mode1_0-GS", "ly00_mode2_3", "ly00_mode3_0", "ly143_144_145", "ly143_144_152_153",
    "ly143_144_mode0_1", "ly143_144_mode3_0", "ly_lyc-GS", "ly_lyc_0-GS", "ly_lyc_0_write-GS", "ly_lyc_144-GS", "ly_lyc_153-GS",
    "ly_lyc_153_write-GS", "ly_lyc_write-GS", "ly_new_frame-GS", "stat_write_if-GS", "vblank_if_timing"])]
fn acceptance_bits(test: &str) {
    run_mooneye_test(
        &("./tests/game-boy-test-roms/artifacts/mooneye-test-suite-wilbertpol/acceptance/gpu/"
            .to_owned()
            + test
            + ".gb"),
    );
}

#[test]
fn acceptance_timer() {
    run_mooneye_test(
        "./tests/game-boy-test-roms/artifacts/mooneye-test-suite-wilbertpol/acceptance/timer/timer_if.gb",
    );
}
