use arbitrary_int::u2;
use bitbybit::bitfield;

#[bitfield(u8)]
pub(crate) struct LCDC {
    #[bit(7, rw)]
    lcd_ppu_enable: bool,

    #[bit(6, rw)]
    window_tile_map: bool,

    #[bit(5, rw)]
    window_enable: bool,

    #[bit(4, rw)]
    tile_addressing_mode: bool,

    #[bit(3, rw)]
    bg_tile_map: bool,

    #[bit(2, rw)]
    obj_size: bool,

    #[bit(1, rw)]
    obj_enable: bool,

    #[bit(0, rw)]
    bg_window_enable_priority: bool,
}

#[bitfield(u8)]
pub(crate) struct STAT {
    #[bit(6, rw)]
    lyc_int_select: bool,

    #[bit(5, rw)]
    mode_2_int_select: bool,

    #[bit(4, rw)]
    mode_1_int_select: bool,

    #[bit(3, rw)]
    mode_0_int_select: bool,

    #[bit(2, rw)]
    lyc_eq_lc: bool,

    #[bits(0..=1, rw)]
    ppu_mode: u2,
}
