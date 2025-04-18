use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;
use Mnemosyne::gb::GameBoy;

fn run_gameboy(rom_path: &str, duration: f32) {
    let mut gameboy = GameBoy::new();
    gameboy.load_rom(rom_path);
    let mut cycles = 0;
    while cycles < (4194304.0 * duration) as u64 {
        let (_, cycles_spent) = gameboy.tick();
        cycles += cycles_spent as u64;
    }
}

#[library_benchmark]
#[bench::fairylake("./tests/game-boy-test-roms/artifacts/scribbltests/fairylake/fairylake.gb")]
#[bench::far_far_away("./src/roms/far_far_away_demo.gb")]
fn bench_gameboy(rom_path: &str) {
    black_box(run_gameboy(rom_path, 10.0));
}

library_benchmark_group!(
    name = bench_gameboy_group;
    benchmarks = bench_gameboy
);

main!(library_benchmark_groups = bench_gameboy_group);
