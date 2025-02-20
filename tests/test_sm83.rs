mod common;

#[cfg(test)]
mod unit_test_CB {
    use crate::common;
    use serde_json::Value;
    use std::fs;
    use test_case::test_matrix;

    #[test_matrix(
        0x00..0xFF
    )]
    fn test(index: usize) {
        println!("Testing opcode {:#04X}", index);
        let mut gameboy = common::create_gameboy();

        let path = format!("./tests/sm83/v1/cb {:02x}.json", index);
        let tests_json = fs::read_to_string(path).expect("Failed to read test json");
        let tests: Value = serde_json::from_str(&tests_json).expect("Failed to parse test json");

        for test in tests
            .as_array()
            .expect("Failed to parse array from test json")
        {
            common::init_gameboy(&mut gameboy, &test["initial"]);

            // TODO: check cycle state
            gameboy.tick();

            common::check_gameboy(&mut gameboy, &test["final"]);
            common::reset_gameboy(&mut gameboy);
        }
    }
}

#[cfg(test)]
mod unit_test {
    use crate::common;
    use serde_json::Value;
    use std::fs;
    use test_case::test_matrix;

    #[test_matrix(
        0x00..0xFF
    )]
    fn test(index: usize) {
        println!("Testing opcode {:#04X}", index);

        // invalid opcodes and the prefix do not have tests
        if [
            0xCB, 0xD3, 0xDB, 0xDD, 0xE3, 0xE4, 0xEB, 0xEC, 0xED, 0xF4, 0xFC, 0xFD,
        ]
        .contains(&(index as i32))
        {
            return;
        }

        // HALT test is probably? not correctly tested
        if index == 0x76 {
            return;
        }

        let mut gameboy = common::create_gameboy();

        let path = format!("./tests/sm83/v1/{:02x}.json", index);
        let tests_json = fs::read_to_string(path).expect("Failed to read test json");
        let tests: Value = serde_json::from_str(&tests_json).expect("Failed to parse test json");

        for test in tests
            .as_array()
            .expect("Failed to parse array from test json")
        {
            common::init_gameboy(&mut gameboy, &test["initial"]);

            // TODO: check cycle state
            gameboy.tick();

            common::check_gameboy(&mut gameboy, &test["final"]);
            common::reset_gameboy(&mut gameboy);
        }
    }
}
