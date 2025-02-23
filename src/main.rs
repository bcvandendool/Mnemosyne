mod egui_renderer;
mod emulator;
mod gameboy;
mod ui;
mod vulkan_renderer;

use crate::egui_renderer::EguiRenderer;
use crate::emulator::{Emulator, SyncMessage};
use crate::vulkan_renderer::VulkanRenderer;
use flexi_logger::{Age, Cleanup, Criterion, FileSpec, LoggerHandle, Naming};
use std::any::Any;
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender, SyncSender};
use std::thread::JoinHandle;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowId;

struct App {
    renderer: VulkanRenderer,
    egui_renderer: EguiRenderer,
    join_handle: Option<JoinHandle<()>>,
    rx_sync: Receiver<SyncMessage>,
    tx_sync: SyncSender<SyncMessage>,
    tx_controls: Sender<KeyEvent>,
    logger_handle: LoggerHandle,
}

fn main() -> Result<(), impl Error> {
    // Setup logging
    let egui_logger = Box::new(egui_logger::builder().build());
    let (flexi_logger, logger_handle) = flexi_logger::Logger::try_with_str("debug")
        .expect("Failed to create flexi_logger")
        .log_to_file(FileSpec::default().directory(PathBuf::from("./log")))
        .rotate(
            Criterion::Age(Age::Day),
            Naming::Timestamps,
            Cleanup::KeepLogFiles(7),
        )
        .build()
        .expect("Failed to build flexi_logger");
    multi_log::MultiLogger::init(vec![egui_logger, flexi_logger], log::Level::Debug)
        .expect("Failed to init multi_logger");

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(&event_loop, logger_handle);

    event_loop.run_app(&mut app)
}

impl App {
    fn new(event_loop: &EventLoop<()>, logger_handle: LoggerHandle) -> Self {
        let renderer = VulkanRenderer::new(event_loop);
        let egui_renderer = EguiRenderer::new(
            &renderer.context,
            renderer.command_buffer_allocator.clone(),
            renderer.descriptor_set_allocator.clone(),
        );

        let (tx_main, rx_emulator) = mpsc::sync_channel::<SyncMessage>(0);
        let (tx_emulator, rx_main) = mpsc::sync_channel::<SyncMessage>(0);
        let (tx_controls, rx_controls) = mpsc::channel::<KeyEvent>();

        let emulator = Emulator::new(
            rx_emulator,
            tx_emulator,
            rx_controls,
            renderer.upload_buffer.clone(),
        );
        let join_handle = Emulator::start(emulator);

        App {
            renderer,
            egui_renderer,
            join_handle: Some(join_handle),
            rx_sync: rx_main,
            tx_sync: tx_main,
            tx_controls,
            logger_handle,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.renderer.create_render_context(event_loop);
        self.egui_renderer
            .create_render_context(&self.renderer.context, &self.renderer.windows);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        self.egui_renderer
            .handle_window_event(&self.renderer.windows, &event);
        match event {
            WindowEvent::CloseRequested => {
                self.tx_sync
                    .send(SyncMessage::Exit)
                    .expect("Failed to send Exit message to emulator thread");
                self.join_handle
                    .take()
                    .unwrap()
                    .join()
                    .expect("Unable to join on emulator thread");
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                self.renderer.resize();
            }
            WindowEvent::RedrawRequested => {
                self.tx_sync
                    .send(SyncMessage::FrameStart(self.egui_renderer.ui_state.clone()))
                    .ok();
                puffin::GlobalProfiler::lock().new_frame();
                let emu_state = match self.rx_sync.recv().ok().unwrap() {
                    SyncMessage::StateSynchronized(emu_state) => emu_state,
                    _ => panic!("Unexpected message received on main thread"),
                };
                self.renderer.redraw(&mut self.egui_renderer, emu_state);
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                let valid_keycodes = [
                    PhysicalKey::Code(KeyCode::ArrowUp),
                    PhysicalKey::Code(KeyCode::ArrowDown),
                    PhysicalKey::Code(KeyCode::ArrowLeft),
                    PhysicalKey::Code(KeyCode::ArrowRight),
                    PhysicalKey::Code(KeyCode::KeyA),
                    PhysicalKey::Code(KeyCode::KeyS),
                    PhysicalKey::Code(KeyCode::KeyD),
                    PhysicalKey::Code(KeyCode::KeyF),
                ];
                if valid_keycodes.contains(&key_event.physical_key) && !key_event.repeat {
                    self.tx_controls
                        .send(key_event)
                        .expect("Failed to send input keycode to emulator thread");
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.renderer.request_redraw();
    }
}
