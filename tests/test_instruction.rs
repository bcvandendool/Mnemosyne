use serde_json::Value;
use std::fs;

mod common;

macro_rules! test {
    ( $from:expr, $to:expr ) => {
        #[cfg(test)]
        mod tests {
            use serde_json::Value;
            use std::fs;
            use seq_macro::seq;
            use test_case::test_case;

            use crate::common;

            seq!(N in $from..=$to {
                #(#[test_case(N)])*
                fn test(index: usize) {
                    // invalid opcodes and the prefix do not have tests
                    if [0xCB, 0xD3, 0xDB, 0xDD, 0xE3, 0xE4, 0xEB, 0xEC, 0xED, 0xF4, 0xFC, 0xFD].contains(&(index as i32)) {
                        return;
                    }

                    let mut gameboy = common::create_gameboy();

                    let path = format!("./tests/sm83-main/v1/{:02x}.json", index);
                    let tests_json =
                        fs::read_to_string(path).expect("Failed to read test json");
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
            });
        }
    };
}

macro_rules! test_cb {
    ( $from:expr, $to:expr ) => {
        #[cfg(test)]
        mod tests_cb {
            use serde_json::Value;
            use std::fs;
            use seq_macro::seq;
            use test_case::test_case;

            use crate::common;

            seq!(N in $from..=$to {
                #(#[test_case(N)])*
                fn test(index: usize) {
                    let mut gameboy = common::create_gameboy();

                    let path = format!("./tests/sm83-main/v1/cb {:02x}.json", index);
                    let tests_json =
                        fs::read_to_string(path).expect("Failed to read test json");
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
            });
        }
    };
}

test!(0x00, 0xFF);
test_cb!(0x00, 0xFF);

#[test]
fn test_manual() {
    let mut gameboy = common::create_gameboy();

    let tests_json =
        fs::read_to_string("./tests/sm83-main/v1/cb 28.json").expect("Failed to read test json");
    let tests: Value = serde_json::from_str(&tests_json).expect("Failed to parse test json");

    for test in tests
        .as_array()
        .expect("Failed to parse array from test json")
    {
        println!("Test name: {}", test["name"]);
        common::init_gameboy(&mut gameboy, &test["initial"]);

        // TODO: check cycle state
        gameboy.tick();

        common::check_gameboy(&mut gameboy, &test["final"]);
        common::reset_gameboy(&mut gameboy);
    }
}
