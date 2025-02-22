use Mnemosyne::gameboy::GameBoy;
use Mnemosyne::gameboy::registers::Reg;
use serde_json::Value;

pub fn create_gameboy() -> GameBoy {
    let mut gameboy = GameBoy::new();
    gameboy.enable_test_memory();
    gameboy
}

pub fn init_gameboy(gameboy: &mut GameBoy, initial_state: &Value) {
    gameboy.set_initial_register(Reg::PC, initial_state["pc"].as_u64().unwrap());
    gameboy.set_initial_register(Reg::SP, initial_state["sp"].as_u64().unwrap());
    gameboy.set_initial_register(Reg::A, initial_state["a"].as_u64().unwrap());
    gameboy.set_initial_register(Reg::B, initial_state["b"].as_u64().unwrap());
    gameboy.set_initial_register(Reg::C, initial_state["c"].as_u64().unwrap());
    gameboy.set_initial_register(Reg::D, initial_state["d"].as_u64().unwrap());
    gameboy.set_initial_register(Reg::E, initial_state["e"].as_u64().unwrap());
    gameboy.set_initial_register(Reg::F, initial_state["f"].as_u64().unwrap());
    gameboy.set_initial_register(Reg::H, initial_state["h"].as_u64().unwrap());
    gameboy.set_initial_register(Reg::L, initial_state["l"].as_u64().unwrap());

    for ram in initial_state["ram"].as_array().unwrap() {
        let address = ram.as_array().unwrap()[0].as_u64().unwrap();
        let value = ram.as_array().unwrap()[1].as_u64().unwrap();
        gameboy.set_initial_memory(address, value);
    }
}

pub fn check_gameboy(gameboy: &mut GameBoy, final_state: &Value) {
    assert_eq!(
        gameboy.get_final_register(Reg::PC),
        final_state["pc"].as_u64().unwrap(),
        "Check register PC is valid"
    );
    assert_eq!(
        gameboy.get_final_register(Reg::SP),
        final_state["sp"].as_u64().unwrap(),
        "Check register SP is valid"
    );
    assert_eq!(
        gameboy.get_final_register(Reg::A),
        final_state["a"].as_u64().unwrap(),
        "Check register A is valid"
    );
    assert_eq!(
        gameboy.get_final_register(Reg::B),
        final_state["b"].as_u64().unwrap(),
        "Check register B is valid"
    );
    assert_eq!(
        gameboy.get_final_register(Reg::C),
        final_state["c"].as_u64().unwrap(),
        "Check register C is valid"
    );
    assert_eq!(
        gameboy.get_final_register(Reg::D),
        final_state["d"].as_u64().unwrap(),
        "Check register D is valid"
    );
    assert_eq!(
        gameboy.get_final_register(Reg::E),
        final_state["e"].as_u64().unwrap(),
        "Check register E is valid"
    );
    assert_eq!(
        gameboy.get_final_register(Reg::F),
        final_state["f"].as_u64().unwrap(),
        "Check register F is valid"
    );
    assert_eq!(
        gameboy.get_final_register(Reg::H),
        final_state["h"].as_u64().unwrap(),
        "Check register H is valid"
    );
    assert_eq!(
        gameboy.get_final_register(Reg::L),
        final_state["l"].as_u64().unwrap(),
        "Check register L is valid"
    );

    for ram in final_state["ram"].as_array().unwrap() {
        let address = ram.as_array().unwrap()[0].as_u64().unwrap();
        let value = gameboy.get_final_memory(address);
        assert_eq!(
            value,
            ram.as_array().unwrap()[1].as_u64().unwrap(),
            "Check memory at address {} is valid",
            ram.as_array().unwrap()[0].as_u64().unwrap()
        );
    }
}

pub fn reset_gameboy(gameboy: &mut GameBoy) {
    gameboy.set_initial_register(Reg::PC, 0);
    gameboy.set_initial_register(Reg::SP, 0);
    gameboy.set_initial_register(Reg::A, 0);
    gameboy.set_initial_register(Reg::B, 0);
    gameboy.set_initial_register(Reg::C, 0);
    gameboy.set_initial_register(Reg::D, 0);
    gameboy.set_initial_register(Reg::E, 0);
    gameboy.set_initial_register(Reg::F, 0);
    gameboy.set_initial_register(Reg::H, 0);
    gameboy.set_initial_register(Reg::L, 0);

    for i in 0..65536 {
        gameboy.set_initial_memory(i as u64, 0);
    }
}
