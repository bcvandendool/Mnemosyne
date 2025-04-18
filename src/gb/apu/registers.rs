use arbitrary_int::{u2, u3, u4, u6};
use bitbybit::bitfield;

// Global control registers
#[bitfield(u8)]
pub(crate) struct NR52 {
    #[bit(7, rw)]
    audio_on: bool,

    #[bit(3, rw)]
    ch4_on: bool,

    #[bit(2, rw)]
    ch3_on: bool,

    #[bit(1, rw)]
    ch2_on: bool,

    #[bit(0, rw)]
    ch1_on: bool,
}

#[bitfield(u8)]
pub(crate) struct NR51 {
    #[bit(7, r)]
    ch4_left: bool,

    #[bit(6, rw)]
    ch3_left: bool,

    #[bit(5, rw)]
    ch2_left: bool,

    #[bit(4, rw)]
    ch1_left: bool,

    #[bit(3, rw)]
    ch4_right: bool,

    #[bit(2, rw)]
    ch3_right: bool,

    #[bit(1, rw)]
    ch2_right: bool,

    #[bit(0, rw)]
    ch1_right: bool,
}

#[bitfield(u8)]
pub(crate) struct NR50 {
    #[bit(7, rw)]
    vin_left: bool,

    #[bits(4..=6, rw)]
    left_volume: u3,

    #[bit(3, rw)]
    vin_right: bool,

    #[bits(0..=2, rw)]
    right_volume: u3,
}

// Pulse channel registers
#[bitfield(u8)]
pub(crate) struct NR10 {
    #[bits(4..=6, rw)]
    pace: u3,

    #[bit(3, rw)]
    direction: bool,

    #[bits(0..=2, rw)]
    individual_step: u3,
}

#[bitfield(u8)]
pub(crate) struct PulseTimerDutyCycle {
    #[bits(6..=7, rw)]
    wave_duty: u2,

    #[bits(0..=5, w)]
    initial_length_timer: u6,
}

// Channel 3 (Wave) registers
#[bitfield(u8)]
pub(crate) struct NR30 {
    #[bit(7, rw)]
    DAC_on: bool,
}

#[bitfield(u8)]
pub(crate) struct NR32 {
    #[bits(5..=6, rw)]
    output_level: u2,
}

// Channel 4 (Noise) registers
#[bitfield(u8)]
pub(crate) struct NR41 {
    #[bits(0..=5, rw)]
    initial_length_timer: u6,
}

#[bitfield(u8)]
pub(crate) struct NR43 {
    #[bits(4..=7, rw)]
    clock_shift: u4,

    #[bit(3, rw)]
    lsfr_width: bool,

    #[bits(0..=2, rw)]
    clock_divider: u3,
}

#[bitfield(u8)]
pub(crate) struct NR44 {
    #[bit(7, w)]
    trigger: bool,

    #[bit(6, rw)]
    length_enable: bool,
}

// Generic channel registers
#[bitfield(u8)]
pub(crate) struct PeriodHighControl {
    #[bit(7, w)]
    trigger: bool,

    #[bit(6, rw)]
    length_enable: bool,

    #[bits(0..=2, r,w)]
    period: u3,
}

#[bitfield(u8)]
pub(crate) struct VolumeEnvelope {
    #[bits(4..=7, rw)]
    initial_volume: u4,

    #[bit(3, rw)]
    env_dir: bool,

    #[bits(0..=2, rw)]
    sweep_pace: u3,
}
