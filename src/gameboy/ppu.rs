use egui::ahash::HashSetExt;
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
    attributes: u8,
    oam_index: u8,
}

impl Sprite {
    fn new(data: &[u8], index: usize, oam_index: u8) -> Self {
        Sprite {
            y: data[index],
            x: data[index + 1],
            tile_index: data[index + 2],
            attributes: data[index + 3],
            oam_index,
        }
    }

    fn attr_priority(&self) -> bool {
        self.attributes & 0b10000000 > 0
    }

    fn attr_y_flip(&self) -> bool {
        self.attributes & 0b01000000 > 0
    }

    fn attr_x_flip(&self) -> bool {
        self.attributes & 0b00100000 > 0
    }

    fn attr_dmg_palette(&self) -> bool {
        self.attributes & 0b00010000 > 0
    }

    fn attr_bank(&self) -> bool {
        self.attributes & 0b00001000 > 0
    }

    fn attr_cgb_palette(&self) -> u8 {
        self.attributes & 0b111
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
    object_attribute_memory: [u8; 160],
    // Registers
    reg_LCDC: u8,            // LCD Control
    pub(crate) reg_STAT: u8, // LCD status
    reg_SCY: u8,             // Viewport Y position
    reg_SCX: u8,             // Viewport X position
    pub(crate) reg_LY: u8,   // LCD Y coordinate
    pub(crate) reg_LYC: u8,  // LY compare
    reg_BGP: u8,             // BG palette data
    reg_OBP0: u8,            // OBJ palette 0 data
    reg_OBP1: u8,            // OBJ palette 1 data
    reg_WY: u8,              // Window Y position
    reg_WX: u8,              // Window X position
    // Interrupts
    pub(crate) int_vblank: bool,
    pub(crate) mode_transitioned: bool,
    pub(crate) lyc_ly_handled: bool,
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
            fetcher_state: FetcherState::InitialBgFetch { dots_remaining: 6 },
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
            reg_LCDC: 0x00,
            reg_STAT: 0x80,
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
            mode_transitioned: false,
            lyc_ly_handled: false,
        }
    }

    pub(crate) fn tick(&mut self, cycles: u32) {
        puffin::profile_scope!("emulate ppu");
        for _ in 0..cycles {
            match self.ppu_mode {
                PPUMode::HorizontalBlank => {
                    if self.dot_counter == 455 {
                        if self.reg_LY == 143 {
                            self.ppu_mode = PPUMode::VerticalBlank;
                            self.mode_transitioned = true;
                            self.int_vblank = true;
                            self.frame_buffer_vblanked = self.frame_buffer.to_vec();
                        } else {
                            self.ppu_mode = PPUMode::OAMScan;
                            self.oam_buffer.clear();
                            self.mode_transitioned = true;
                        }
                        self.dot_counter = 0;
                        self.reg_LY += 1;
                        continue;
                    }
                }
                PPUMode::VerticalBlank => {
                    if self.reg_LY == 153 && self.dot_counter == 4 {
                        self.reg_LY = 0;
                    }

                    if self.dot_counter == 455 {
                        self.dot_counter = 0;
                        // Check if final line, LY is already set to 0 due to "scanline 153 quirk"
                        if self.reg_LY == 0 {
                            self.reg_LY = 0;
                            self.window_y = 0;
                            self.ppu_mode = PPUMode::OAMScan;
                            self.oam_buffer.clear();
                            self.mode_transitioned = true;
                            continue;
                        } else {
                            self.reg_LY += 1;
                        }
                    }
                }
                PPUMode::OAMScan => {
                    if self.dot_counter % 2 == 0 {
                        // Check if sprite should be added to buffer
                        let sprite_idx = self.dot_counter / 2;
                        let mut sprite = Sprite::new(
                            &self.object_attribute_memory,
                            (sprite_idx * 4) as usize,
                            0,
                        );

                        if self.reg_LY + 16 >= sprite.y
                            && self.reg_LY + 16
                                < sprite.y + if self.reg_LCDC & 0b100 > 0 { 16 } else { 8 }
                            && self.oam_buffer.len() < 10
                        {
                            sprite.oam_index = self.oam_buffer.len() as u8;
                            self.oam_buffer.push(sprite);
                        }
                    }
                    if self.dot_counter == 79 {
                        self.ppu_mode = PPUMode::DrawingPixels;
                        self.mode_transitioned = true;
                        self.fetcher_state = FetcherState::InitialBgFetch { dots_remaining: 6 };
                        self.oam_buffer
                            .sort_by(|a, b| a.x.cmp(&b.x).then(a.oam_index.cmp(&b.oam_index)));
                    }
                }
                PPUMode::DrawingPixels => {
                    match self.fetcher_state {
                        FetcherState::InitialBgFetch { dots_remaining } => {
                            if dots_remaining == 0 {
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
                            if dots_remaining == 0 {
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

                                if self.reg_LCDC & 0b10 > 0 {
                                    self.fetch_sprite_tile(sprite);
                                    let sprite_fetch_cycles = if !sprite_fetch_delayed
                                        && (4..9).contains(&dots_remaining)
                                    {
                                        sprite_fetch_delayed = true;
                                        6 + dots_remaining - 3
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
                                    continue;
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
                            if self.reg_LCDC & 0b100000 > 0
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
                                continue;
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
                                let mut color = if sprite_pixel.color != 0
                                    && !(bg_pixel.color != 0
                                        && self.reg_LCDC & 0b1 > 0
                                        && sprite_pixel.background_priority)
                                {
                                    // Output sprite pixel
                                    self.get_sprite_color(sprite_pixel.palette, sprite_pixel.color)
                                } else if self.reg_LCDC & 0b1 > 0 {
                                    // Output background / window pixel
                                    self.get_color(bg_pixel.color)
                                } else {
                                    0
                                };

                                self.frame_buffer
                                    [self.reg_LY as usize * 160 + screen_x as usize - 8] = color;
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
                                self.mode_transitioned = true;
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
    }

    fn fetch_sprite_tile(&mut self, sprite: Sprite) {
        let sprite_row = self.reg_LY.wrapping_sub(sprite.y.wrapping_add(16));
        let tile_number = if self.reg_LCDC & 0b100 > 0 {
            let base_tile_number = sprite.tile_index & !0b1;
            let lower_tile = ((sprite_row & 0b1000) >> 3) ^ u8::from(sprite.attr_y_flip());
            base_tile_number | lower_tile
        } else {
            sprite.tile_index
        };

        let tile_row = if sprite.attr_y_flip() {
            7 - (sprite_row & 0x07)
        } else {
            sprite_row & 0x07
        };

        let tile_address = 0x8000 | (tile_number as u16) << 4 | (tile_row as u16) << 1;

        let tile_lo = self.tile_data[tile_address as usize - 0x8000];
        let tile_hi = self.tile_data[tile_address as usize - 0x8000 + 1];

        for i in self.sprite_fifo.len()..8 {
            self.sprite_fifo.push_back(PixelInfo {
                color: 0,
                palette: 0,
                sprite_priority: false,
                background_priority: true,
            })
        }

        for i in 0..8 {
            let idx = if sprite.attr_x_flip() { i } else { 7 - i };
            if self.sprite_fifo.get(i).is_none()
                || self
                    .sprite_fifo
                    .get(i)
                    .is_some_and(|pixel| pixel.color == 0)
            {
                self.sprite_fifo.insert(
                    i,
                    PixelInfo {
                        color: ((tile_hi >> idx) & 1) << 1 | ((tile_lo >> idx) & 1),
                        palette: u8::from(sprite.attr_dmg_palette()),
                        sprite_priority: false,
                        background_priority: sprite.attr_priority(),
                    },
                )
            }
        }
    }

    fn fetch_bg_tile(&mut self, fetcher_x: u8) {
        let tile_map_base: u16 = if self.reg_LCDC & 0b1000 > 0 {
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

        let bit_12: u16 = match self.reg_LCDC & 0b10000 {
            0 => match tile_id & 0b10000000 {
                0 => 1,
                _ => 0,
            },
            _ => 0,
        };

        let tile_address = 0b100 << 13
            | bit_12 << 12
            | (tile_id as u16) << 4
            | ((self.reg_LY.wrapping_add(self.reg_SCY) % 8) as u16) << 1;

        let tile_lo = self.tile_data[tile_address as usize - 0x8000];
        let tile_hi = self.tile_data[tile_address as usize - 0x8000 + 1];

        for i in 0..8 {
            let idx = 7 - i;
            self.pixel_fifo.push_back(PixelInfo {
                color: ((tile_hi >> idx) & 1) << 1 | ((tile_lo >> idx) & 1),
                palette: 0,
                sprite_priority: false,
                background_priority: false,
            })
        }
    }

    fn fetch_window_tile(&mut self, fetcher_x: u8) {
        let tile_map_base: u16 = if (self.reg_LCDC & 0b1000000) > 0 {
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

        let tile_address = if self.reg_LCDC & 0b10000 > 0 {
            0x8000 | (tile_id as u16) << 4 | ((self.window_y % 8) as u16) << 1
        } else {
            0x9000_u16.wrapping_add((tile_id as i8 as u16) << 4) | ((self.window_y % 8) as u16) << 1
        };

        let tile_lo = self.tile_data[tile_address as usize - 0x8000];
        let tile_hi = self.tile_data[tile_address as usize - 0x8000 + 1];

        for i in 0..8 {
            let idx = 7 - i;
            self.pixel_fifo.push_back(PixelInfo {
                color: ((tile_hi >> idx) & 1) << 1 | ((tile_lo >> idx) & 1),
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

    pub(crate) fn read(&self, address: u16) -> u8 {
        match address {
            0x8000..=0x97FF => self.tile_data[address as usize - 0x8000],
            0x9800..=0x9BFF => self.background_map_1[address as usize - 0x9800],
            0x9C00..=0x9FFF => self.background_map_2[address as usize - 0x9C00],
            0xFE00..=0xFE9F => self.object_attribute_memory[address as usize - 0xFE00],
            0xFF40 => self.reg_LCDC,
            0xFF41 => self.reg_STAT | 0x80,
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
            0x8000..=0x97FF => self.tile_data[address as usize - 0x8000] = value,
            0x9800..=0x9BFF => self.background_map_1[address as usize - 0x9800] = value,
            0x9C00..=0x9FFF => self.background_map_2[address as usize - 0x9C00] = value,
            0xFE00..=0xFE9F => self.object_attribute_memory[address as usize - 0xFE00] = value,
            0xFF40 => self.reg_LCDC = value,
            0xFF41 => self.reg_STAT = value,
            0xFF42 => self.reg_SCY = value,
            0xFF43 => self.reg_SCX = value,
            0xFF44 => self.reg_LY = value,
            0xFF45 => self.reg_LYC = value,
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
