use image::{GenericImageView, ImageReader};
use Mnemosyne::gb::GameBoy;
#[test]
fn test() {
    let mut gameboy = GameBoy::new();
    gameboy.load_rom("./tests/game-boy-test-roms/artifacts/dmg-acid2/dmg-acid2.gb");
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

    let img = ImageReader::open("./tests/game-boy-test-roms/artifacts/dmg-acid2/dmg-acid2-dmg.png")
        .unwrap()
        .decode()
        .unwrap();
    let test = img.pixels().flat_map(|a| a.2.0).collect::<Vec<u8>>();

    assert_eq!(output_img, test);
}
