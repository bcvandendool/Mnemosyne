mod egui_renderer;
mod emulator;
mod gameboy;
mod ui;
mod vulkan_renderer;

use crate::egui_renderer::EguiRenderer;
use crate::emulator::{Emulator, SyncMessage};
use crate::vulkan_renderer::VulkanRenderer;
use std::any::Any;
use std::error::Error;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread::JoinHandle;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

struct App {
    renderer: VulkanRenderer,
    egui_renderer: EguiRenderer,
    join_handle: Option<JoinHandle<()>>,
    rx: Receiver<SyncMessage>,
    tx: SyncSender<SyncMessage>,
}

fn main() -> Result<(), impl Error> {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(&event_loop);

    event_loop.run_app(&mut app)
}

impl App {
    fn new(event_loop: &EventLoop<()>) -> Self {
        let renderer = VulkanRenderer::new(event_loop);
        let egui_renderer = EguiRenderer::new(
            &renderer.context,
            renderer.command_buffer_allocator.clone(),
            renderer.descriptor_set_allocator.clone(),
        );

        let (tx_main, rx_emulator) = mpsc::sync_channel::<SyncMessage>(0);
        let (tx_emulator, rx_main) = mpsc::sync_channel::<SyncMessage>(0);

        let emulator = Emulator::new(rx_emulator, tx_emulator, renderer.upload_buffer.clone());
        let join_handle = Emulator::start(emulator);

        App {
            renderer,
            egui_renderer,
            join_handle: Some(join_handle),
            rx: rx_main,
            tx: tx_main,
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
                self.tx
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
                self.tx
                    .send(SyncMessage::FrameStart(self.egui_renderer.ui_state.clone()))
                    .ok();
                let emu_state = match self.rx.recv().ok().unwrap() {
                    SyncMessage::StateSynchronized(emu_state) => emu_state,
                    _ => panic!("Unexpected message received on main thread"),
                };
                self.renderer.redraw(&mut self.egui_renderer, emu_state);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.renderer.request_redraw();
    }
}
