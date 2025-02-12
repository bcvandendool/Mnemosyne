use crate::gameboy::GameBoy;
use crate::SyncMessage;
use rand::Rng;
use std::sync::mpsc::Receiver;
use std::thread;
use std::thread::JoinHandle;
use vulkano::buffer::Subbuffer;

pub(crate) struct Emulator {
    rx: Receiver<SyncMessage>,
    upload_buffer: Subbuffer<[u8]>,
}

impl Emulator {
    pub(crate) fn new(rx: Receiver<SyncMessage>, upload_buffer: Subbuffer<[u8]>) -> Emulator {
        Emulator { rx, upload_buffer }
    }

    pub(crate) fn start(emu: Emulator) -> JoinHandle<()> {
        thread::spawn(move || emu.run())
    }

    fn run(&self) {
        let mut gameboy = GameBoy::new();
        gameboy.load_rom("far_far_away_demo.gb");

        loop {
            match self.rx.recv().unwrap() {
                SyncMessage::FrameStart => {
                    if rand::random_bool(0.01) {
                        let idx = rand::rng().random_range(0..160 * 144) * 4;
                        self.upload_buffer.write().unwrap()[idx] = 0xFF;
                        self.upload_buffer.write().unwrap()[idx + 1] = 0x00;
                        self.upload_buffer.write().unwrap()[idx + 2] = 0x00;
                    }
                }
                SyncMessage::StateSynchronized => {}
                SyncMessage::Exit => {
                    return;
                }
            }
        }
    }
}
