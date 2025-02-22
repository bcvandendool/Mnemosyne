use crate::gameboy::registers::Registers;
use crate::gameboy::GameBoy;
use crate::ui::UIState;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;
use std::thread::JoinHandle;
use vulkano::buffer::Subbuffer;
use winit::event::{ElementState, KeyEvent};

pub(crate) enum SyncMessage {
    FrameStart(UIState),
    StateSynchronized(EmulatorState),
    Exit,
}

pub(crate) struct Emulator {
    rx: Receiver<SyncMessage>,
    tx: SyncSender<SyncMessage>,
    rx_controls: Receiver<KeyEvent>,
    upload_buffer: Subbuffer<[u8]>,
}

pub struct EmulatorState {
    pub(crate) registers: Registers,
    pub(crate) ram: Vec<u8>,
}

impl Emulator {
    pub(crate) fn new(
        rx: Receiver<SyncMessage>,
        tx: SyncSender<SyncMessage>,
        rx_controls: Receiver<KeyEvent>,
        upload_buffer: Subbuffer<[u8]>,
    ) -> Emulator {
        Emulator {
            rx,
            tx,
            rx_controls,
            upload_buffer,
        }
    }

    pub(crate) fn start(emu: Emulator) -> JoinHandle<()> {
        thread::spawn(move || emu.run())
    }

    fn run(&self) {
        let mut gameboy = GameBoy::new();
        gameboy.load_rom("../../tests/game-boy-test-roms/artifacts/mooneye-test-suite/emulator-only/mbc5/rom_512kb.gb");
        gameboy.skip_boot_rom();

        loop {
            match self.rx.recv().unwrap() {
                SyncMessage::FrameStart(state) => {
                    // Prep data for render thread
                    {
                        let mut writer = self.upload_buffer.write().unwrap();
                        let frame_buffer = gameboy.get_framebuffer();
                        for idx in 0..(160 * 144) {
                            let color: u8 = match frame_buffer[idx] {
                                0 => 0xFF,
                                1 => 0xAA,
                                2 => 0x55,
                                3 => 0x00,
                                _ => panic!("Received invalid color code"),
                            };
                            writer[idx * 4] = color;
                            writer[idx * 4 + 1] = color;
                            writer[idx * 4 + 2] = color;
                            writer[idx * 4 + 3] = 0xFF;
                        }
                    }

                    // Send state synchronized message
                    let emu_state = EmulatorState::new(
                        gameboy.dump_registers(),
                        gameboy.dump_ram(state.selected_memory),
                    );
                    self.tx.send(SyncMessage::StateSynchronized(emu_state)).ok();

                    // Do stuff per frame while previous frame is being rendered
                    if state.emulator_running {
                        // TODO: proper time handling
                        for _ in 0..1000 {
                            for _ in 0..4194304 / 1000 / 180 {
                                gameboy.tick();
                            }

                            // Check inputs
                            while let Ok(key_event) = self.rx_controls.try_recv() {
                                if key_event.state == ElementState::Pressed {
                                    gameboy.key_pressed(key_event.physical_key);
                                } else {
                                    gameboy.key_released(key_event.physical_key);
                                }
                            }
                        }
                    }

                    if state.emulator_should_step {
                        gameboy.tick();
                    }
                }
                SyncMessage::Exit => {
                    return;
                }
                _ => {
                    panic!("Received unexpected message on emulator thread");
                }
            }
        }
    }
}

impl EmulatorState {
    pub(crate) fn new(registers: Registers, ram: Vec<u8>) -> Self {
        EmulatorState { registers, ram }
    }
}
