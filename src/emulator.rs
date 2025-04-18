use crate::gb::registers::Registers;
use crate::gb::GameBoy;
use crate::ui::UIState;
use crate::vulkan_renderer::EmulatorRenderer;
use puffin::{internal_profile_reporter, ThreadProfiler};
use std::path::Path;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
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
    rx_ui: Receiver<EmulatorControlMessage>,
    emulator_renderer: Arc<Mutex<dyn EmulatorRenderer>>,
    runtime_state: RuntimeState,
}

pub enum EmulatorState {
    GameBoy(GameBoyState),
}

pub struct GameBoyState {
    pub(crate) registers: Registers,
    pub(crate) ram: Vec<u8>,
    pub(crate) hit_breakpoint: bool,
    pub(crate) frame_buffer: Vec<u8>,
}

pub enum EmulatorControlMessage {
    // Standard controls
    Start,
    Stop,
    Pause,
    FastForward(u8),
    FastRewind(u8),
    // Load / save
    Load(String),
    LoadState,
    SaveState,
    // Debugging
    DebugMode(bool),
    StepOver,
    StepInto,
    StepOut,
    Breakpoints,
    Watchpoints,
}

#[derive(PartialEq)]
pub enum RuntimeState {
    Stopped,
    Paused,
    Running,
    Stepping,
}

impl Emulator {
    pub(crate) fn new(
        rx: Receiver<SyncMessage>,
        tx: SyncSender<SyncMessage>,
        rx_controls: Receiver<KeyEvent>,
        rx_ui: Receiver<EmulatorControlMessage>,
        emulator_renderer: Arc<Mutex<dyn EmulatorRenderer>>,
    ) -> Emulator {
        Emulator {
            rx,
            tx,
            rx_controls,
            rx_ui,
            emulator_renderer,
            runtime_state: RuntimeState::Stopped,
        }
    }

    pub(crate) fn start(mut emu: Emulator) -> JoinHandle<()> {
        thread::Builder::new()
            .name("Emulator".to_owned())
            .spawn(move || emu.run())
            .expect("Failed to start emulation thread!")
    }

    fn run(&mut self) {
        let mut gameboy = GameBoy::new();

        let mut hit_breakpoint: bool = false;

        let mut previous_time = fastant::Instant::now();

        loop {
            match self.rx.recv().unwrap() {
                SyncMessage::FrameStart(state) => {
                    {
                        puffin::profile_scope!("sync to render thread");
                        let emu_state = EmulatorState::new(
                            gameboy.dump_registers(),
                            gameboy.dump_ram(state.selected_memory),
                            hit_breakpoint,
                            gameboy.get_framebuffer(),
                        );
                        let mut renderer = self
                            .emulator_renderer
                            .lock()
                            .expect("Failed to acquire render lock");
                        renderer.sync_render_world(&emu_state);
                        self.tx.send(SyncMessage::StateSynchronized(emu_state)).ok();
                    }

                    while let Ok(message) = self.rx_ui.try_recv() {
                        match message {
                            EmulatorControlMessage::Start => {
                                self.runtime_state = RuntimeState::Running;
                            }
                            EmulatorControlMessage::Load(path) => {
                                gameboy = GameBoy::new();
                                gameboy.load_rom(&path);
                                self.runtime_state = RuntimeState::Stopped;
                                // TODO: skip bootrom or not based on settings
                                gameboy.skip_boot_rom();
                            }
                            EmulatorControlMessage::Pause => {
                                self.runtime_state = RuntimeState::Paused;
                            }
                            EmulatorControlMessage::Stop => {
                                self.runtime_state = RuntimeState::Stopped;
                                gameboy = GameBoy::new();
                            }
                            EmulatorControlMessage::StepInto
                            | EmulatorControlMessage::StepOut
                            | EmulatorControlMessage::StepOver => {
                                self.runtime_state = RuntimeState::Stepping;
                            }
                            _ => {}
                        }
                    }

                    if self.runtime_state != RuntimeState::Running {
                        previous_time = fastant::Instant::now();
                    }
                    //
                    // if hit_breakpoint {
                    //     hit_breakpoint = false;
                    //     continue;
                    // }
                    //
                    // gb.set_breakpoints(state.breakpoints);
                    //
                    // Do stuff per frame while previous frame is being rendered
                    if self.runtime_state == RuntimeState::Running {
                        puffin::profile_scope!("emulate");

                        let elapsed = previous_time.elapsed().as_secs_f64().min(0.1);
                        previous_time = fastant::Instant::now();
                        let mut cycles = 0;

                        while cycles < (elapsed * (4194304.0 / 4.0)) as u64 {
                            let (hit_breakpoint_now, cycles_spent) = gameboy.tick();
                            cycles += cycles_spent as u64;

                            if hit_breakpoint_now {
                                hit_breakpoint = hit_breakpoint_now;
                                break;
                            }

                            while let Ok(key_event) = self.rx_controls.try_recv() {
                                if key_event.state == ElementState::Pressed {
                                    gameboy.key_pressed(key_event.physical_key);
                                } else {
                                    gameboy.key_released(key_event.physical_key);
                                }
                            }
                        }

                        {
                            if puffin::are_scopes_on() {
                                static SCOPE_ID: std::sync::OnceLock<puffin::ScopeId> =
                                    std::sync::OnceLock::new();
                                let scope_id = SCOPE_ID.get_or_init(|| {
                                    puffin::ThreadProfiler::call(|tp| {
                                        tp.register_named_scope(
                                            "ppu",
                                            puffin::clean_function_name(
                                                puffin::current_function_name!(),
                                            ),
                                            puffin::short_file_name(file!()),
                                            line!(),
                                        )
                                    })
                                });
                                let start_stream_offset = ThreadProfiler::call(|tp| {
                                    tp.begin_scope_with_offset(
                                        *scope_id,
                                        "".as_ref(),
                                        -(gameboy.cpu.time_ppu.as_nanos() as i64),
                                    )
                                });
                                ThreadProfiler::call(|tp| tp.end_scope(start_stream_offset));
                            }
                            if puffin::are_scopes_on() {
                                static SCOPE_ID: std::sync::OnceLock<puffin::ScopeId> =
                                    std::sync::OnceLock::new();
                                let scope_id2 = SCOPE_ID.get_or_init(|| {
                                    puffin::ThreadProfiler::call(|tp| {
                                        tp.register_named_scope(
                                            "io",
                                            puffin::clean_function_name(
                                                puffin::current_function_name!(),
                                            ),
                                            puffin::short_file_name(file!()),
                                            line!(),
                                        )
                                    })
                                });
                                let start_stream_offset2 = ThreadProfiler::call(|tp| {
                                    tp.begin_scope_with_offset(
                                        *scope_id2,
                                        "".as_ref(),
                                        -(gameboy.cpu.time_ppu.as_nanos() as i64
                                            + gameboy.cpu.time_io.as_nanos() as i64),
                                    )
                                });
                                ThreadProfiler::call(|tp| {
                                    tp.end_scope_with_offset(
                                        start_stream_offset2,
                                        -(gameboy.cpu.time_ppu.as_nanos() as i64),
                                    )
                                });
                                gameboy.cpu.time_ppu = Duration::new(0, 0);
                                gameboy.cpu.time_io = Duration::new(0, 0);
                            }
                        }
                    }

                    if self.runtime_state == RuntimeState::Stepping {
                        puffin::profile_scope!("emulate tick");
                        gameboy.tick();
                        self.runtime_state = RuntimeState::Paused;
                    }
                }
                SyncMessage::Exit => {
                    gameboy.cpu.mmu.mbc.save_ram();
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
    pub(crate) fn new(
        registers: Registers,
        ram: Vec<u8>,
        hit_breakpoint: bool,
        frame_buffer: Vec<u8>,
    ) -> Self {
        EmulatorState::GameBoy(GameBoyState {
            registers,
            ram,
            hit_breakpoint,
            frame_buffer,
        })
    }
}
