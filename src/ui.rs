mod components;
mod views;

use crate::egui_renderer::CallbackFn;
use crate::emulator::{EmulatorControlMessage, EmulatorState};
use crate::gb::breakpoints::Breakpoints;
use crate::gb::disassembler::{Address, Disassembler};
use crate::gb::registers::Flag;
use crate::vulkan_renderer::EmulatorRenderer;
use egui::text::{LayoutJob, LayoutSection};
use egui::{menu, vec2, Align, Context, PaintCallback, Rgba, Sense, TextFormat, TextStyle};
use egui_extras::{Column, TableBuilder};
use std::ops::Deref;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::{SyntaxSet, SyntaxSetBuilder};
use winit::dpi::PhysicalSize;

#[derive(Clone, PartialEq)]
enum BottomPanels {
    Logger,
    Profiler,
}

#[derive(Clone, PartialEq)]
enum Views {
    Fullscreen,
    Debugger,
    GameList,
}

#[derive(Clone)]
pub struct UIState {
    pub(crate) emulator_running: bool,
    pub(crate) emulator_should_step: bool,
    pub(crate) breakpoints: Breakpoints,
    pub(crate) selected_memory: Memories,
    bottom_panel: BottomPanels,
    volume: f32,
    current_view: Views,
    game_list: Vec<String>,
    selected_game: String,
    search_string: String,
    tx_ui: Sender<EmulatorControlMessage>,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Memories {
    ExternalRAM,
    WRAM1,
    WRAM2,
    TileData,
    BackgroundMaps,
    OAM,
    HRAM,
    IO,
}

pub(crate) struct UIContext {
    ts: ThemeSet,
    ps: SyntaxSet,
    previous_pc: u16,
    disassembler: Disassembler,
    disassembly: Vec<(Option<Address>, String)>,
}

impl UIContext {
    pub(crate) fn new() -> Self {
        let ts = ThemeSet::load_defaults();
        let mut builder = SyntaxSetBuilder::new();
        builder
            .add_from_folder(Path::new("./"), false)
            .expect("Failed to load syntax files");
        let ps = builder.build();

        let mut disassembler = Disassembler::new(Path::new(
            "./tests/game-boy-test-roms/artifacts/mooneye-test-suite/acceptance/ppu/vblank_stat_intr-GS.gb",
        ));
        disassembler.disassemble();
        //disassembler.save_sym_file(Path::new("./src/roms/rex-run.sym"));

        let table = disassembler.to_table();

        UIContext {
            ts,
            ps,
            previous_pc: 0x00FF,
            disassembler,
            disassembly: table,
        }
    }
}

impl UIState {
    pub(crate) fn new(tx_ui: Sender<EmulatorControlMessage>) -> Self {
        UIState {
            emulator_running: false,
            emulator_should_step: false,
            breakpoints: Breakpoints::new(),
            selected_memory: Memories::WRAM1,
            bottom_panel: BottomPanels::Logger,
            volume: 50.0,
            current_view: Views::GameList,
            game_list: Vec::new(),
            selected_game: String::new(),
            search_string: "Search".to_string(),
            tx_ui,
        }
    }

    fn toggle_row_selection(&mut self, address: Option<Address>, row_response: &egui::Response) {
        if address.is_some() && row_response.clicked() {
            if self
                .breakpoints
                .breakpoints
                .contains(&address.clone().unwrap().address)
            {
                self.breakpoints
                    .breakpoints
                    .remove(&(address.unwrap().address));
            } else {
                self.breakpoints
                    .breakpoints
                    .insert(address.unwrap().address);
            }
        }
    }
}

pub(crate) fn create_ui(
    egui_context: &Context,
    size: PhysicalSize<u32>,
    ui_state: &mut UIState,
    emu_state: EmulatorState,
    ui_context: &mut UIContext,
    emulator_renderer: Arc<Mutex<dyn EmulatorRenderer>>,
) {
    puffin::profile_scope!("Create UI");

    match ui_state.current_view {
        Views::Debugger => {
            views::debugger::render(
                egui_context,
                ui_state,
                &emu_state,
                ui_context,
                emulator_renderer,
            );
        }
        Views::Fullscreen => {
            views::fullscreen::render(
                egui_context,
                ui_state,
                &emu_state,
                ui_context,
                emulator_renderer,
            );
        }
        Views::GameList => {
            views::game_list::render(
                egui_context,
                size,
                ui_state,
                &emu_state,
                ui_context,
                emulator_renderer,
            );
        }
    }
}

fn as_byte_range(whole: &str, range: &str) -> std::ops::Range<usize> {
    let whole_start = whole.as_ptr() as usize;
    let range_start = range.as_ptr() as usize;
    assert!(whole_start <= range_start);
    assert!(range_start + range.len() <= whole_start + whole.len());
    let offset = range_start - whole_start;
    offset..(offset + range.len())
}
