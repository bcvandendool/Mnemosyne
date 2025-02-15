use crate::gameboy::registers::Registers;
use crate::gameboy::GameBoy;
use crate::ui::UIState;
use rand::Rng;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;
use std::thread::JoinHandle;
use vulkano::buffer::Subbuffer;

pub(crate) enum SyncMessage {
    FrameStart(UIState),
    StateSynchronized(EmulatorState),
    Exit,
}

pub(crate) struct Emulator {
    rx: Receiver<SyncMessage>,
    tx: SyncSender<SyncMessage>,
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
        upload_buffer: Subbuffer<[u8]>,
    ) -> Emulator {
        Emulator {
            rx,
            tx,
            upload_buffer,
        }
    }

    pub(crate) fn start(emu: Emulator) -> JoinHandle<()> {
        thread::spawn(move || emu.run())
    }

    fn run(&self) {
        let mut gameboy = GameBoy::new();
        gameboy.load_rom("../../tests/gb-test-roms/halt_bug.gb");
        gameboy.skip_boot_rom();

        loop {
            match self.rx.recv().unwrap() {
                SyncMessage::FrameStart(state) => {
                    // Prep data for render thread
                    if rand::random_bool(0.01) {
                        let idx = rand::rng().random_range(0..160 * 144) * 4;
                        self.upload_buffer.write().unwrap()[idx] = 0xFF;
                        self.upload_buffer.write().unwrap()[idx + 1] = 0x00;
                        self.upload_buffer.write().unwrap()[idx + 2] = 0x00;
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
                        for _ in 0..4194304 / 60 {
                            gameboy.tick();
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
