mod registers;

use arbitrary_int::{u2, u3};
use bitbybit::bitfield;
use egui::ahash::HashSetExt;
use intbits::Bits;
use registers::*;
use std::cmp::PartialEq;
use std::collections::VecDeque;

#[derive(PartialEq)]
pub(crate) enum PPUMode {
    HorizontalBlank = 0,
    VerticalBlank = 1,
    OAMScan = 2,
    DrawingPixels = 3,
}

enum FetcherState {
    InitialBgFetch {
        dots_remaining: u8,
    },
    RenderingTile {
        dots_remaining: u8,
        screen_x: u8,
        fetcher_x: u8,
        rendering_background: bool,
        sprite_fetch_delayed: bool,
    },
    InitialWindowFetch {
        dots_remaining: u8,
        screen_x: u8,
    },
    SpriteFetch {
        dots_remaining: u8,
        render_dots_remaining: u8,
        render_screen_x: u8,
        render_fetcher_x: u8,
        render_rendering_background: bool,
        render_sprite_fetch_delayed: bool,
    },
}

enum ActiveFetcher {
    Background,
    Sprite,
    Window,
}

#[derive(Eq, Hash, PartialEq)]
struct Sprite {
    y: u8,
    x: u8,
    tile_index: u8,
    attributes: OAMAttributes,
    oam_index: u8,
}

#[bitfield(u8)]
#[derive(Eq, Hash, PartialEq)]
struct OAMAttributes {
    #[bit(7, rw)]
    priority: bool,

    #[bit(6, rw)]
    y_flip: bool,

    #[bit(5, rw)]
    x_flip: bool,

    #[bit(4, rw)]
    dmg_palette: bool,

    #[bit(3, rw)]
    bank: bool,

    #[bits(0..=2, rw)]
    cgm_palette: u3,
}

impl Sprite {
    fn new(data: &[u8], index: usize, oam_index: u8) -> Self {
        Sprite {
            y: data[index],
            x: data[index + 1],
            tile_index: data[index + 2],
            attributes: OAMAttributes::new_with_raw_value(data[index + 3]),
            oam_index,
        }
    }
}

struct PixelInfo {
    color: u8,
    palette: u8,
    sprite_priority: bool,
    background_priority: bool,
}

pub(crate) struct PPU {
    // State
    pub(crate) ppu_mode: PPUMode,
    dot_counter: u16,
    oam_buffer: Vec<Sprite>,
    pixel_fifo: VecDeque<PixelInfo>,
    sprite_fifo: VecDeque<PixelInfo>,
    fetcher_state: FetcherState,
    current_fetcher: ActiveFetcher,
    frame_buffer: [u8; 160 * 144],
    pub(crate) frame_buffer_vblanked: Vec<u8>,
    window_y: u8,
    // Memory
    pub(crate) tile_data: [u8; 6144],
    pub(crate) background_map_1: [u8; 1024],
    pub(crate) background_map_2: [u8; 1024],
    pub(crate) object_attribute_memory: [u8; 160],
    // Registers
    reg_LCDC: LCDC,            // LCD Control
    pub(crate) reg_STAT: STAT, // LCD status
    reg_SCY: u8,               // Viewport Y position
    reg_SCX: u8,               // Viewport X position
    pub(crate) reg_LY: u8,     // LCD Y coordinate
    pub(crate) reg_LYC: u8,    // LY compare
    reg_BGP: u8,               // BG palette data
    reg_OBP0: u8,              // OBJ palette 0 data
    reg_OBP1: u8,              // OBJ palette 1 data
    reg_WY: u8,                // Window Y position
    reg_WX: u8,                // Window X position
    // Interrupts
    pub(crate) int_vblank: bool,
    pub(crate) int_stat: bool,
    test_counter: u64,
    first_line: bool,
    new_line: bool,
    stat_delay: u8,
    first_frame: bool,
}

impl PPU {
    pub(crate) fn new() -> Self {
        PPU {
            // State
            ppu_mode: PPUMode::OAMScan,
            dot_counter: 0,
            oam_buffer: Vec::new(),
            pixel_fifo: VecDeque::new(),
            sprite_fifo: VecDeque::new(),
            fetcher_state: FetcherState::InitialBgFetch { dots_remaining: 4 },
            current_fetcher: ActiveFetcher::Background,
            frame_buffer: [0; 160 * 144],
            frame_buffer_vblanked: vec![0; 160 * 144],
            window_y: 0,
            // Memory
            tile_data: [0; 6144],
            background_map_1: [0; 1024],
            background_map_2: [0; 1024],
            object_attribute_memory: [0; 160],
            // Registers
            reg_LCDC: LCDC::ZERO,
            reg_STAT: STAT::ZERO,
            reg_SCY: 0x00,
            reg_SCX: 0x00,
            reg_LY: 0x00,
            reg_LYC: 0x00,
            reg_BGP: 0x00,
            reg_OBP0: 0x00,
            reg_OBP1: 0x00,
            reg_WY: 0x00,
            reg_WX: 0x00,
            // Interrupts
            int_vblank: false,
            int_stat: false,
            test_counter: 0,
            first_line: false,
            new_line: false,
            stat_delay: 0,
            first_frame: false,
        }
    }

    pub(crate) fn tick(&mut self) {
        self.test_counter += 1;
        if !self.reg_LCDC.lcd_ppu_enable() {
            return;
        }

        if self.stat_delay > 0 {
            self.stat_delay -= 1;
            if self.stat_delay == 0 {
                self.update_reg_STAT();
            }
        }

        match self.ppu_mode {
            PPUMode::HorizontalBlank => {
                if self.dot_counter == 455 {
                    self.first_line = false;
                    if self.reg_LY == 143 {
                        self.ppu_mode = PPUMode::VerticalBlank;
                        self.stat_delay = 3;
                        self.update_reg_STAT();
                        self.int_vblank = true;
                        self.frame_buffer_vblanked = self.frame_buffer.to_vec();
                    } else {
                        self.ppu_mode = PPUMode::OAMScan;
                        self.stat_delay = 3;
                        self.update_reg_STAT();
                        self.oam_buffer.clear();
                    }
                    self.dot_counter = 0;
                    self.reg_LY += 1;
                    if self.ppu_mode == PPUMode::OAMScan {
                        self.new_line = true;
                    }
                    self.update_reg_STAT();
                    return;
                }
            }
            PPUMode::VerticalBlank => {
                if self.reg_LY == 153 && self.dot_counter == 4 {
                    self.reg_LY = 0;
                    self.update_reg_STAT();
                }

                if self.dot_counter == 455 {
                    self.dot_counter = 0;
                    // Check if final line, LY is already set to 0 due to "scanline 153 quirk"
                    if self.reg_LY == 0 {
                        self.window_y = 0;
                        self.ppu_mode = PPUMode::OAMScan;
                        self.first_frame = false;
                        self.stat_delay = 3;
                        self.new_line = true;
                        self.update_reg_STAT();
                        self.oam_buffer.clear();
                        return;
                    } else {
                        self.reg_LY += 1;
                        self.update_reg_STAT();
                        return;
                    }
                }
            }
            PPUMode::OAMScan => {
                if self.dot_counter == 3 {
                    self.new_line = false;
                    self.update_reg_STAT();
                }

                if self.dot_counter % 2 == 0 {
                    // Check if sprite should be added to buffer
                    let sprite_idx = self.dot_counter / 2;
                    let mut sprite =
                        Sprite::new(&self.object_attribute_memory, (sprite_idx * 4) as usize, 0);

                    if self.reg_LY + 16 >= sprite.y
                        && self.reg_LY + 16
                            < sprite.y + if self.reg_LCDC.obj_size() { 16 } else { 8 }
                        && self.oam_buffer.len() < 10
                    {
                        sprite.oam_index = self.oam_buffer.len() as u8;
                        self.oam_buffer.push(sprite);
                    }
                }
                if self.dot_counter == 79 {
                    self.ppu_mode = PPUMode::DrawingPixels;
                    self.stat_delay = 3;
                    self.update_reg_STAT();
                    let delay: u8 = match self.reg_SCX % 8 {
                        0 => 0,
                        1..=4 => 1,
                        5..=8 => 2,
                        _ => unreachable!(),
                    };
                    self.fetcher_state = FetcherState::InitialBgFetch {
                        dots_remaining: 4 + delay,
                    };
                    self.oam_buffer
                        .sort_by(|a, b| a.x.cmp(&b.x).then(a.oam_index.cmp(&b.oam_index)));
                }
            }
            PPUMode::DrawingPixels => {
                match self.fetcher_state {
                    FetcherState::InitialBgFetch { dots_remaining } => {
                        if dots_remaining == 1 {
                            self.pixel_fifo.clear();
                            self.fetch_bg_tile(0);
                            self.fetcher_state = FetcherState::RenderingTile {
                                dots_remaining: 8,
                                screen_x: 0_u8.wrapping_sub(self.reg_SCX % 8),
                                fetcher_x: 0,
                                rendering_background: true,
                                sprite_fetch_delayed: false,
                            }
                        } else {
                            self.fetcher_state = FetcherState::InitialBgFetch {
                                dots_remaining: dots_remaining - 1,
                            }
                        }
                    }
                    FetcherState::InitialWindowFetch {
                        dots_remaining,
                        screen_x,
                    } => {
                        if dots_remaining == 1 {
                            self.fetcher_state = FetcherState::RenderingTile {
                                dots_remaining: 8,
                                screen_x,
                                fetcher_x: 1,
                                rendering_background: false,
                                sprite_fetch_delayed: false,
                            }
                        } else {
                            self.fetcher_state = FetcherState::InitialWindowFetch {
                                dots_remaining: dots_remaining - 1,
                                screen_x,
                            }
                        }
                    }
                    FetcherState::SpriteFetch {
                        dots_remaining,
                        render_dots_remaining,
                        render_screen_x,
                        render_fetcher_x,
                        render_rendering_background,
                        render_sprite_fetch_delayed,
                    } => {
                        if dots_remaining == 1 {
                            self.fetcher_state = FetcherState::RenderingTile {
                                dots_remaining: render_dots_remaining,
                                screen_x: render_screen_x,
                                fetcher_x: render_fetcher_x,
                                rendering_background: render_rendering_background,
                                sprite_fetch_delayed: render_sprite_fetch_delayed,
                            };
                        } else {
                            self.fetcher_state = FetcherState::SpriteFetch {
                                dots_remaining: dots_remaining - 1,
                                render_dots_remaining,
                                render_screen_x,
                                render_fetcher_x,
                                render_rendering_background,
                                render_sprite_fetch_delayed,
                            };
                        }
                    }
                    FetcherState::RenderingTile {
                        mut dots_remaining,
                        mut screen_x,
                        mut fetcher_x,
                        rendering_background,
                        mut sprite_fetch_delayed,
                    } => {
                        // Check for sprites
                        if self.oam_buffer.iter().any(|sprite| sprite.x == screen_x) {
                            let sprite = self.oam_buffer.remove(
                                self.oam_buffer
                                    .iter()
                                    .position(|sprite| sprite.x == screen_x)
                                    .unwrap(),
                            );

                            if self.reg_LCDC.obj_enable() {
                                let x = sprite.x;
                                self.fetch_sprite_tile(sprite);
                                let sprite_fetch_cycles =
                                    if !sprite_fetch_delayed && (2..9).contains(&dots_remaining) {
                                        sprite_fetch_delayed = true;
                                        if x == 0 {
                                            6 + dots_remaining - 3 + self.reg_SCX % 8
                                        } else {
                                            6 + dots_remaining - 3
                                        }
                                    } else {
                                        6
                                    };

                                self.fetcher_state = FetcherState::SpriteFetch {
                                    dots_remaining: sprite_fetch_cycles - 1,
                                    render_dots_remaining: dots_remaining,
                                    render_screen_x: screen_x,
                                    render_fetcher_x: fetcher_x,
                                    render_rendering_background: rendering_background,
                                    render_sprite_fetch_delayed: sprite_fetch_delayed,
                                };
                                return;
                            }

                            while self.oam_buffer.iter().any(|sprite| sprite.x == screen_x) {
                                self.oam_buffer.remove(
                                    self.oam_buffer
                                        .iter()
                                        .position(|sprite| sprite.x == screen_x)
                                        .unwrap(),
                                );
                            }
                        }

                        // Check if window has started
                        if self.reg_LCDC.window_enable()
                                && self.reg_WY <= self.reg_LY // TODO: change this to have it only have been == once in the frame
                                && screen_x == self.reg_WX.wrapping_add(1)
                                && screen_x <= 160 + 7
                                && rendering_background
                        {
                            self.pixel_fifo.clear();
                            self.fetch_window_tile(0);
                            self.fetcher_state = FetcherState::InitialWindowFetch {
                                dots_remaining: 5,
                                screen_x,
                            };
                            return;
                        }

                        if dots_remaining == 2 {
                            if rendering_background {
                                self.fetch_bg_tile(fetcher_x);
                            } else {
                                self.fetch_window_tile(fetcher_x);
                            }
                            fetcher_x = fetcher_x.wrapping_add(1);
                        }

                        let bg_pixel = self
                            .pixel_fifo
                            .pop_front()
                            .expect("There should always be a background pixel");
                        let sprite_pixel = self.sprite_fifo.pop_front().unwrap_or(PixelInfo {
                            color: 0,
                            palette: 0,
                            sprite_priority: false,
                            background_priority: true,
                        });

                        if (8..160 + 8).contains(&screen_x) {
                            let color = if sprite_pixel.color != 0
                                && !(bg_pixel.color != 0
                                    && self.reg_LCDC.bg_window_enable_priority()
                                    && sprite_pixel.background_priority)
                                && !self.first_frame
                            {
                                // Output sprite pixel
                                self.get_sprite_color(sprite_pixel.palette, sprite_pixel.color)
                            } else if self.reg_LCDC.bg_window_enable_priority() && !self.first_frame
                            {
                                // Output background / window pixel
                                self.get_color(bg_pixel.color)
                            } else {
                                0
                            };

                            self.frame_buffer[self.reg_LY as usize * 160 + screen_x as usize - 8] =
                                color;
                        }
                        screen_x = screen_x.wrapping_add(1);

                        if dots_remaining == 1 {
                            dots_remaining = 8;
                            sprite_fetch_delayed = false;
                        } else {
                            dots_remaining -= 1;
                        }

                        if screen_x == 160 + 8 {
                            if !rendering_background {
                                self.window_y += 1;
                            }
                            self.ppu_mode = PPUMode::HorizontalBlank;
                            self.stat_delay = 3;
                            self.update_reg_STAT();
                        } else {
                            self.fetcher_state = FetcherState::RenderingTile {
                                dots_remaining,
                                screen_x,
                                fetcher_x,
                                rendering_background,
                                sprite_fetch_delayed,
                            };
                        }
                    }
                }
            }
        }
        self.dot_counter += 1;
    }

    fn fetch_sprite_tile(&mut self, sprite: Sprite) {
        let sprite_row = self.reg_LY.wrapping_sub(sprite.y.wrapping_add(16));
        let tile_number = if self.reg_LCDC.obj_size() {
            let base_tile_number = sprite.tile_index & !0b1;
            let lower_tile = ((sprite_row & 0b1000) >> 3) ^ u8::from(sprite.attributes.y_flip());
            base_tile_number | lower_tile
        } else {
            sprite.tile_index
        };

        let tile_row = if sprite.attributes.y_flip() {
            7 - (sprite_row & 0x07)
        } else {
            sprite_row & 0x07
        };

        let tile_address = 0x8000 | (tile_number as u16) << 4 | (tile_row as u16) << 1;

        let tile_lo = self.tile_data[tile_address as usize - 0x8000];
        let tile_hi = self.tile_data[tile_address as usize - 0x8000 + 1];

        for _ in self.sprite_fifo.len()..8 {
            self.sprite_fifo.push_back(PixelInfo {
                color: 0,
                palette: 0,
                sprite_priority: false,
                background_priority: true,
            })
        }

        for i in 0..8 {
            let idx = if sprite.attributes.x_flip() { i } else { 7 - i };
            if self.sprite_fifo.get(i).is_none()
                || self
                    .sprite_fifo
                    .get(i)
                    .is_some_and(|pixel| pixel.color == 0)
            {
                let test = self.sprite_fifo.get_mut(i).unwrap();
                *test = PixelInfo {
                    color: ((tile_hi >> idx) & 1) << 1 | ((tile_lo >> idx) & 1),
                    palette: u8::from(sprite.attributes.dmg_palette()),
                    sprite_priority: false,
                    background_priority: sprite.attributes.priority(),
                };
            }
        }
    }

    fn fetch_bg_tile(&mut self, fetcher_x: u8) {
        let tile_map_base: u16 = if self.reg_LCDC.bg_tile_map() {
            0x9C00
        } else {
            0x9800
        };

        let tile_map_address = tile_map_base
            | (((self.reg_LY.wrapping_add(self.reg_SCY) / 8) as u16) << 5)
            | ((fetcher_x.wrapping_add(self.reg_SCX / 8) as u16) % 32);

        let tile_id = match tile_map_address {
            0x9800..=0x9BFF => self.background_map_1[tile_map_address as usize - 0x9800],
            0x9C00..=0x9FFF => self.background_map_2[tile_map_address as usize - 0x9C00],
            _ => panic!("Invalid tile address"),
        };

        let bit_12: u16 = match self.reg_LCDC.tile_addressing_mode() {
            false => match tile_id & 0b10000000 {
                0 => 1,
                _ => 0,
            },
            true => 0,
        };

        let tile_address = (0b100 << 13)
            | (bit_12 << 12)
            | ((tile_id as u16) << 4)
            | (((self.reg_LY.wrapping_add(self.reg_SCY) % 8) as u16) << 1);

        let tile_lo = self.tile_data[tile_address as usize - 0x8000];
        let tile_hi = self.tile_data[tile_address as usize - 0x8000 + 1];

        for i in 0..8 {
            let idx = 7 - i;
            self.pixel_fifo.push_back(PixelInfo {
                color: (((tile_hi >> idx) & 1) << 1) | ((tile_lo >> idx) & 1),
                palette: 0,
                sprite_priority: false,
                background_priority: false,
            })
        }
    }

    fn fetch_window_tile(&mut self, fetcher_x: u8) {
        let tile_map_base: u16 = if self.reg_LCDC.window_tile_map() {
            0x9C00
        } else {
            0x9800
        };

        let tile_map_address =
            tile_map_base | (((self.window_y / 8) as u16) << 5) | fetcher_x as u16;

        let tile_id = match tile_map_address {
            0x9800..=0x9BFF => self.background_map_1[tile_map_address as usize - 0x9800],
            0x9C00..=0x9FFF => self.background_map_2[tile_map_address as usize - 0x9C00],
            _ => panic!("Invalid tile address"),
        };

        // let bit_12: u16 = match self.reg_LCDC & 0b10000 {
        //     0 => match tile_id & 0b10000000 {
        //         0 => 1,
        //         _ => 0,
        //     },
        //     _ => 0,
        // };
        // let tile_address =
        //     0b100 << 13 | bit_12 << 12 | (tile_id as u16) << 4 | ((self.window_y % 8) as u16) << 1;

        let tile_address = if self.reg_LCDC.tile_addressing_mode() {
            0x8000 | ((tile_id as u16) << 4) | (((self.window_y % 8) as u16) << 1)
        } else {
            0x9000_u16.wrapping_add((tile_id as i8 as u16) << 4)
                | (((self.window_y % 8) as u16) << 1)
        };

        let tile_lo = self.tile_data[tile_address as usize - 0x8000];
        let tile_hi = self.tile_data[tile_address as usize - 0x8000 + 1];

        for i in 0..8 {
            let idx = 7 - i;
            self.pixel_fifo.push_back(PixelInfo {
                color: (((tile_hi >> idx) & 1) << 1) | ((tile_lo >> idx) & 1),
                palette: 0,
                sprite_priority: false,
                background_priority: false,
            })
        }
    }

    fn get_color(&mut self, color_id: u8) -> u8 {
        match color_id {
            0 => self.reg_BGP & 0b11,
            1 => (self.reg_BGP & 0b1100) >> 2,
            2 => (self.reg_BGP & 0b110000) >> 4,
            3 => (self.reg_BGP & 0b11000000) >> 6,
            _ => {
                panic!("Invalid color id");
            }
        }
    }

    fn get_sprite_color(&mut self, palette: u8, color_id: u8) -> u8 {
        match palette {
            0 => match color_id {
                1 => (self.reg_OBP0 & 0b1100) >> 2,
                2 => (self.reg_OBP0 & 0b110000) >> 4,
                3 => (self.reg_OBP0 & 0b11000000) >> 6,
                _ => {
                    panic!("Invalid color id");
                }
            },
            1 => match color_id {
                1 => (self.reg_OBP1 & 0b1100) >> 2,
                2 => (self.reg_OBP1 & 0b110000) >> 4,
                3 => (self.reg_OBP1 & 0b11000000) >> 6,
                _ => {
                    panic!("Invalid color id");
                }
            },
            _ => panic!("Invalid color palette"),
        }
    }

    fn update_reg_STAT(&mut self) {
        let old_statline = (self.reg_STAT.lyc_int_select() && self.reg_STAT.lyc_eq_lc())
            | (self.reg_STAT.mode_0_int_select() && self.reg_STAT.ppu_mode() == u2::new(0))
            | (self.reg_STAT.mode_1_int_select() && self.reg_STAT.ppu_mode() == u2::new(1))
            | (self.reg_STAT.mode_2_int_select() && (self.reg_STAT.ppu_mode() == u2::new(2)));

        if self.reg_LCDC.lcd_ppu_enable() {
            if self.stat_delay == 0 {
                let value: u8 = match self.ppu_mode {
                    PPUMode::HorizontalBlank => 0,
                    PPUMode::VerticalBlank => 1,
                    PPUMode::OAMScan => 2,
                    PPUMode::DrawingPixels => 3,
                };
                self.reg_STAT.set_ppu_mode(u2::new(value));
            }

            if self.new_line && self.reg_LY != 0 {
                self.reg_STAT.set_lyc_eq_lc(false);
            } else {
                self.reg_STAT.set_lyc_eq_lc(self.reg_LYC == self.reg_LY);
            }
        } else {
            self.reg_STAT.set_ppu_mode(u2::new(0));
        }

        let new_statline = (self.reg_STAT.lyc_int_select() && self.reg_STAT.lyc_eq_lc())
            | (self.reg_STAT.mode_0_int_select() && self.reg_STAT.ppu_mode() == u2::new(0))
            | (self.reg_STAT.mode_1_int_select() && self.reg_STAT.ppu_mode() == u2::new(1))
            | (self.reg_STAT.mode_2_int_select()
                && (self.reg_STAT.ppu_mode() == u2::new(2) || self.int_vblank));

        if !old_statline && new_statline {
            self.int_stat = true;
        }
    }

    pub(crate) fn read(&self, address: u16) -> u8 {
        match address {
            0x8000..=0x97FF => {
                if !self.reg_LCDC.lcd_ppu_enable()
                    || (self.reg_STAT.ppu_mode() == u2::new(0) && !self.first_line)
                    || (self.reg_STAT.ppu_mode() == u2::new(1)
                        && !self.first_line
                        && !self.new_line)
                    || (self.reg_STAT.ppu_mode() == u2::new(2)
                        && !self.first_line
                        && self.ppu_mode != PPUMode::DrawingPixels)
                    || (self.first_line && (self.dot_counter <= 80 || self.dot_counter >= 253))
                {
                    self.tile_data[address as usize - 0x8000]
                } else {
                    0xFF
                }
            }
            0x9800..=0x9BFF => {
                if !self.reg_LCDC.lcd_ppu_enable()
                    || (self.reg_STAT.ppu_mode() == u2::new(0)
                        && !self.first_line
                        && !self.new_line)
                    || (self.reg_STAT.ppu_mode() == u2::new(1)
                        && !self.first_line
                        && !self.new_line)
                    || (self.reg_STAT.ppu_mode() == u2::new(2)
                        && !self.first_line
                        && self.ppu_mode != PPUMode::DrawingPixels)
                    || (self.first_line && (self.dot_counter <= 80 || self.dot_counter >= 253))
                {
                    self.background_map_1[address as usize - 0x9800]
                } else {
                    0xFF
                }
            }
            0x9C00..=0x9FFF => {
                if !self.reg_LCDC.lcd_ppu_enable()
                    || (self.reg_STAT.ppu_mode() == u2::new(0)
                        && !self.first_line
                        && !self.new_line)
                    || (self.reg_STAT.ppu_mode() == u2::new(1)
                        && !self.first_line
                        && !self.new_line)
                    || (self.reg_STAT.ppu_mode() == u2::new(2)
                        && !self.first_line
                        && self.ppu_mode != PPUMode::DrawingPixels)
                    || (self.first_line && (self.dot_counter <= 80 || self.dot_counter >= 253))
                {
                    self.background_map_2[address as usize - 0x9C00]
                } else {
                    0xFF
                }
            }
            0xFE00..=0xFE9F => {
                if !self.reg_LCDC.lcd_ppu_enable()
                    || (self.reg_STAT.ppu_mode() == u2::new(0)
                        && !self.first_line
                        && !self.new_line)
                    || (self.reg_STAT.ppu_mode() == u2::new(1)
                        && !self.first_line
                        && !self.new_line)
                    || (self.first_line && (self.dot_counter <= 80 || self.dot_counter >= 253))
                {
                    self.object_attribute_memory[address as usize - 0xFE00]
                } else {
                    0xFF
                }
            }
            0xFF40 => self.reg_LCDC.raw_value(),
            0xFF41 => {
                if self.first_line {
                    let value = match self.dot_counter {
                        0..81 => 0,
                        81..253 => 3,
                        253.. => 0,
                    };
                    (self.reg_STAT.raw_value() & !0x3) | 0x80 | value
                } else {
                    self.reg_STAT.raw_value() | 0x80
                }
            }
            0xFF42 => self.reg_SCY,
            0xFF43 => self.reg_SCX,
            0xFF44 => self.reg_LY,
            0xFF45 => self.reg_LYC,
            0xFF47 => self.reg_BGP,
            0xFF48 => self.reg_OBP0,
            0xFF49 => self.reg_OBP1,
            0xFF4A => self.reg_WY,
            0xFF4B => self.reg_WX,
            0xFF4F => {
                // VRAM bank - CGB
                0xFF
            }
            0xFF68..=0xFF6C => {
                // CGB
                0xFF
            }
            _ => panic!("Invalid address received for PPU: {}", address),
        }
    }

    pub(crate) fn write(&mut self, address: u16, value: u8) {
        match address {
            0x8000..=0x97FF => {
                if !self.reg_LCDC.lcd_ppu_enable()
                    || (self.reg_STAT.ppu_mode() == u2::new(0) && !self.first_line)
                    || (self.reg_STAT.ppu_mode() == u2::new(1)
                        && !self.first_line
                        && !self.new_line)
                    || (self.reg_STAT.ppu_mode() == u2::new(2)
                        && !self.first_line
                        && self.ppu_mode != PPUMode::DrawingPixels)
                    || ((80..83).contains(&self.dot_counter) && !self.first_line)
                    || (self.first_line && (self.dot_counter <= 80 || self.dot_counter >= 253))
                {
                    self.tile_data[address as usize - 0x8000] = value
                }
            }
            0x9800..=0x9BFF => {
                if !self.reg_LCDC.lcd_ppu_enable()
                    || (self.reg_STAT.ppu_mode() == u2::new(0) && !self.first_line)
                    || (self.reg_STAT.ppu_mode() == u2::new(1)
                        && !self.first_line
                        && !self.new_line)
                    || (self.reg_STAT.ppu_mode() == u2::new(2)
                        && !self.first_line
                        && self.ppu_mode != PPUMode::DrawingPixels)
                    || ((80..83).contains(&self.dot_counter) && !self.first_line)
                    || (self.first_line && (self.dot_counter <= 80 || self.dot_counter >= 253))
                {
                    self.background_map_1[address as usize - 0x9800] = value
                }
            }
            0x9C00..=0x9FFF => {
                if !self.reg_LCDC.lcd_ppu_enable()
                    || (self.reg_STAT.ppu_mode() == u2::new(0) && !self.first_line)
                    || (self.reg_STAT.ppu_mode() == u2::new(1)
                        && !self.first_line
                        && !self.new_line)
                    || (self.reg_STAT.ppu_mode() == u2::new(2)
                        && !self.first_line
                        && self.ppu_mode != PPUMode::DrawingPixels)
                    || ((80..83).contains(&self.dot_counter) && !self.first_line)
                    || (self.first_line && (self.dot_counter <= 80 || self.dot_counter >= 253))
                {
                    self.background_map_2[address as usize - 0x9C00] = value
                }
            }
            0xFE00..=0xFE9F => {
                if !self.reg_LCDC.lcd_ppu_enable()
                    || (self.reg_STAT.ppu_mode() == u2::new(0) && !self.first_line)
                    || (self.reg_STAT.ppu_mode() == u2::new(1) && !self.first_line)
                    || ((80..83).contains(&self.dot_counter) && !self.first_line)
                    || (self.first_line && (self.dot_counter <= 80 || self.dot_counter >= 253))
                {
                    self.object_attribute_memory[address as usize - 0xFE00] = value
                }
            }
            0xFEA0..=0xFEFF => {
                // TODO: OAM corruption
            }
            0xFF40 => {
                self.reg_LCDC = LCDC::new_with_raw_value(value);
                if !self.reg_LCDC.lcd_ppu_enable() {
                    self.reg_LY = 0;
                    self.dot_counter = 4;
                    self.ppu_mode = PPUMode::OAMScan;
                    self.first_line = true;
                    self.first_frame = true;
                }
                self.update_reg_STAT()
            }
            0xFF41 => {
                let old_statline = (self.reg_STAT.lyc_int_select() && self.reg_STAT.lyc_eq_lc())
                    | (self.reg_STAT.mode_0_int_select() && self.reg_STAT.ppu_mode() == u2::new(0))
                    | (self.reg_STAT.mode_1_int_select() && self.reg_STAT.ppu_mode() == u2::new(1))
                    | (self.reg_STAT.mode_2_int_select()
                        && (self.reg_STAT.ppu_mode() == u2::new(2)
                            || (self.reg_STAT.ppu_mode() == u2::new(1)
                                && self.reg_LY == 143
                                && self.dot_counter == 455)));

                self.reg_STAT.set_lyc_int_select(value.bit(6));
                self.reg_STAT.set_mode_2_int_select(value.bit(5));
                self.reg_STAT.set_mode_1_int_select(value.bit(4));
                self.reg_STAT.set_mode_0_int_select(value.bit(3));

                let new_statline = (self.reg_STAT.lyc_int_select() && self.reg_STAT.lyc_eq_lc())
                    | (self.reg_STAT.mode_0_int_select() && self.reg_STAT.ppu_mode() == u2::new(0))
                    | (self.reg_STAT.mode_1_int_select() && self.reg_STAT.ppu_mode() == u2::new(1))
                    | (self.reg_STAT.mode_2_int_select()
                        && (self.reg_STAT.ppu_mode() == u2::new(2)
                            || (self.reg_STAT.ppu_mode() == u2::new(1)
                                && self.reg_LY == 143
                                && self.dot_counter == 455)));

                if !old_statline && new_statline && self.reg_LCDC.lcd_ppu_enable() {
                    self.int_stat = true;
                }
            }
            0xFF42 => self.reg_SCY = value,
            0xFF43 => self.reg_SCX = value,
            0xFF44 => {}
            0xFF45 => {
                self.reg_LYC = value;
                self.update_reg_STAT()
            }
            0xFF47 => self.reg_BGP = value,
            0xFF48 => self.reg_OBP0 = value,
            0xFF49 => self.reg_OBP1 = value,
            0xFF4A => self.reg_WY = value,
            0xFF4B => self.reg_WX = value,
            0xFF4F => {
                // VRAM bank - CGB
            }
            0xFF68..=0xFF6C => {
                // CGB
            }
            _ => println!("Invalid address received for PPU: {:#06X}", address),
        }
    }
}
