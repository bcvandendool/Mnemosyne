mod registers;

use crate::audio::AudioPlayer;
use arbitrary_int::{u3, Number};
use blip_buf::BlipBuf;
use intbits::Bits;
use registers::*;

const WAVE_PATTERN_DUTIES: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5% duty cycle
    [0, 0, 0, 0, 0, 0, 1, 1], // 25% duty cycle
    [0, 0, 0, 0, 1, 1, 1, 1], // 50% duty cycle
    [1, 1, 1, 1, 1, 1, 0, 0], // 75% duty cycle
];

pub(crate) struct APU {
    // Registers
    reg_NR10: NR10,
    reg_NR11: PulseTimerDutyCycle,
    reg_NR12: VolumeEnvelope,
    reg_NR13: u8,
    reg_NR14: PeriodHighControl,
    reg_NR21: PulseTimerDutyCycle,
    reg_NR22: VolumeEnvelope,
    reg_NR23: u8,
    reg_NR24: PeriodHighControl,
    reg_NR30: NR30,
    reg_NR31: u8,
    reg_NR32: NR32,
    reg_NR33: u8,
    reg_NR34: PeriodHighControl,
    reg_NR41: NR41,
    reg_NR42: VolumeEnvelope,
    reg_NR43: NR43,
    reg_NR44: NR44,
    reg_NR50: NR50,
    reg_NR51: NR51,
    reg_NR52: NR52,
    // RAM
    wave_ram: [u8; 16],
    // Internal state
    blip_ch1: BlipBuf,
    blip_ch2: BlipBuf,
    blip_ch3: BlipBuf,
    blip_ch4: BlipBuf,
    last_amp_ch1: u8,
    last_amp_ch2: u8,
    last_amp_ch3: u8,
    last_amp_ch4: u8,
    DIV_APU: u8,
    // Length timers
    length_timer_ch1: u16,
    length_timer_ch2: u16,
    length_timer_ch3: u16,
    length_timer_ch4: u16,
    // DACs
    DAC_ch1_enabled: bool,
    DAC_ch2_enabled: bool,
    DAC_ch3_enabled: bool,
    DAC_ch4_enabled: bool,
    // Wave duty
    frequency_timer_ch1: u32,
    frequency_timer_ch2: u32,
    wave_duty_position_ch1: u3,
    wave_duty_position_ch2: u3,
    // Envelope function
    period_timer_ch1: u8,
    period_timer_ch2: u8,
    period_timer_ch4: u8,
    current_volume_ch1: u8,
    current_volume_ch2: u8,
    current_volume_ch4: u8,
    // Sweep control
    sweep_enabled: bool,
    shadow_frequency: u32,
    sweep_timer: u8,
    done_sweep_calc: bool,
    // Audio player
    audio_player: AudioPlayer,
    output_period: u32,
    output_timer: u32,
    // Wave channel
    wave_duty_position_ch3: u8,
    sample_buffer: u8,
    frequency_timer_ch3: u32,
    just_read_ch3: bool,
    just_read_ch3_counter: u32,
    // Noise channel
    frequency_timer_ch4: u32,
    LFSR: u16,
    // high pass filter
    capacitor_left: f32,
    capacitor_right: f32,
}

const CLOCK_RATE: f64 = 4194304.0;
const OUTPUT_SAMPLE_COUNT: u32 = 2000;

impl APU {
    pub(crate) fn new(audio_player: AudioPlayer) -> APU {
        let mut blip_ch1 = BlipBuf::new(audio_player.sample_rate.0 / 10);
        blip_ch1.set_rates(CLOCK_RATE, audio_player.sample_rate.0 as f64);

        let mut blip_ch2 = BlipBuf::new(audio_player.sample_rate.0 / 10);
        blip_ch2.set_rates(CLOCK_RATE, audio_player.sample_rate.0 as f64);

        let mut blip_ch3 = BlipBuf::new(audio_player.sample_rate.0 / 10);
        blip_ch3.set_rates(CLOCK_RATE, audio_player.sample_rate.0 as f64);

        let mut blip_ch4 = BlipBuf::new(audio_player.sample_rate.0 / 10);
        blip_ch4.set_rates(CLOCK_RATE, audio_player.sample_rate.0 as f64);

        let output_period = ((OUTPUT_SAMPLE_COUNT as u64 * CLOCK_RATE as u64)
            / audio_player.sample_rate.0 as u64) as u32;

        APU {
            reg_NR10: NR10::ZERO,
            reg_NR11: PulseTimerDutyCycle::ZERO,
            reg_NR12: VolumeEnvelope::ZERO,
            reg_NR13: 0,
            reg_NR14: PeriodHighControl::ZERO,
            reg_NR21: PulseTimerDutyCycle::ZERO,
            reg_NR22: VolumeEnvelope::ZERO,
            reg_NR23: 0,
            reg_NR24: PeriodHighControl::ZERO,
            reg_NR30: NR30::ZERO,
            reg_NR31: 0,
            reg_NR32: NR32::ZERO,
            reg_NR33: 0,
            reg_NR34: PeriodHighControl::ZERO,
            reg_NR41: NR41::ZERO,
            reg_NR42: VolumeEnvelope::ZERO,
            reg_NR43: NR43::ZERO,
            reg_NR44: NR44::ZERO,
            reg_NR50: NR50::ZERO,
            reg_NR51: NR51::ZERO,
            reg_NR52: NR52::ZERO,
            wave_ram: [0; 16],
            blip_ch1,
            blip_ch2,
            blip_ch3,
            blip_ch4,
            last_amp_ch1: 0,
            last_amp_ch2: 0,
            last_amp_ch3: 0,
            last_amp_ch4: 0,
            DIV_APU: 0,
            length_timer_ch1: 0,
            length_timer_ch2: 0,
            length_timer_ch3: 0,
            length_timer_ch4: 0,
            DAC_ch1_enabled: false,
            DAC_ch2_enabled: false,
            DAC_ch3_enabled: false,
            DAC_ch4_enabled: false,
            frequency_timer_ch1: 0,
            frequency_timer_ch2: 0,
            wave_duty_position_ch1: u3::new(0),
            wave_duty_position_ch2: u3::new(0),
            period_timer_ch1: 0,
            period_timer_ch2: 0,
            period_timer_ch4: 0,
            current_volume_ch1: 0,
            current_volume_ch2: 0,
            current_volume_ch4: 0,
            sweep_enabled: false,
            shadow_frequency: 0,
            sweep_timer: 0,
            done_sweep_calc: false,
            audio_player,
            output_period,
            output_timer: 0,
            wave_duty_position_ch3: 0,
            sample_buffer: 0,
            frequency_timer_ch3: 0,
            just_read_ch3: false,
            just_read_ch3_counter: 0,
            frequency_timer_ch4: 1,
            LFSR: 0,
            capacitor_left: 0.0,
            capacitor_right: 0.0,
        }
    }

    pub(crate) fn tick(&mut self, div_apu: bool) {
        if div_apu {
            self.DIV_APU = self.DIV_APU.wrapping_add(1);
            if self.reg_NR52.audio_on() {
                // Handle DIV-APU counter events
                if self.DIV_APU % 2 == 0 {
                    // Sound length
                    if self.reg_NR14.length_enable() && self.length_timer_ch1 > 0 {
                        self.length_timer_ch1 -= 1;
                        if self.length_timer_ch1 == 0 {
                            self.reg_NR52.set_ch1_on(false);
                        }
                    }

                    if self.reg_NR24.length_enable() && self.length_timer_ch2 > 0 {
                        self.length_timer_ch2 -= 1;
                        if self.length_timer_ch2 == 0 {
                            self.reg_NR52.set_ch2_on(false);
                        }
                    }

                    if self.reg_NR34.length_enable() && self.length_timer_ch3 > 0 {
                        self.length_timer_ch3 -= 1;
                        if self.length_timer_ch3 == 0 {
                            self.reg_NR52.set_ch3_on(false);
                        }
                    }

                    if self.reg_NR44.length_enable() && self.length_timer_ch4 > 0 {
                        self.length_timer_ch4 -= 1;
                        if self.length_timer_ch4 == 0 {
                            self.reg_NR52.set_ch4_on(false);
                        }
                    }
                }
                if self.DIV_APU % 4 == 0 {
                    // CH1 freq sweep
                    if self.sweep_timer > 0 {
                        self.sweep_timer -= 1;
                    }

                    if self.sweep_timer == 0 {
                        if self.reg_NR10.pace().value() > 0 {
                            self.sweep_timer = self.reg_NR10.pace().value();
                        } else {
                            self.sweep_timer = 8;
                        }

                        if self.sweep_enabled && self.reg_NR10.pace().value() > 0 {
                            let mut new_frequency =
                                self.shadow_frequency >> self.reg_NR10.individual_step().value();

                            if self.reg_NR10.direction() {
                                new_frequency = self.shadow_frequency - new_frequency;
                                self.done_sweep_calc = true;
                            } else {
                                new_frequency = self.shadow_frequency + new_frequency;
                            }

                            if new_frequency >= 2048 {
                                self.reg_NR52.set_ch1_on(false);
                            }

                            if new_frequency < 2048 && self.reg_NR10.individual_step().value() > 0 {
                                self.shadow_frequency = new_frequency;

                                self.reg_NR13 = self.shadow_frequency as u8;
                                self.reg_NR14
                                    .set_period(u3::new(self.shadow_frequency.bits(8..11) as u8));

                                new_frequency = self.shadow_frequency
                                    >> self.reg_NR10.individual_step().value();

                                if self.reg_NR10.direction() {
                                    new_frequency = self.shadow_frequency - new_frequency;
                                    self.done_sweep_calc = true;
                                } else {
                                    new_frequency = self.shadow_frequency + new_frequency;
                                }

                                if new_frequency >= 2048 {
                                    self.reg_NR52.set_ch1_on(false);
                                }
                            }
                        }
                    }
                }
                if self.DIV_APU % 8 == 0 {
                    // Envelope sweep
                    if self.reg_NR12.sweep_pace().value() != 0 {
                        if self.period_timer_ch1 > 0 {
                            self.period_timer_ch1 -= 1;
                        }

                        if self.period_timer_ch1 == 0 {
                            self.period_timer_ch1 = self.reg_NR12.sweep_pace().value();

                            if self.current_volume_ch1 < 0xF && self.reg_NR12.env_dir() {
                                self.current_volume_ch1 += 1;
                            } else if self.current_volume_ch1 > 0x0 && !self.reg_NR12.env_dir() {
                                self.current_volume_ch1 -= 1;
                            }
                        }
                    }

                    if self.reg_NR22.sweep_pace().value() != 0 {
                        if self.period_timer_ch2 > 0 {
                            self.period_timer_ch2 -= 1;
                        }

                        if self.period_timer_ch2 == 0 {
                            self.period_timer_ch2 = self.reg_NR22.sweep_pace().value();

                            if self.current_volume_ch2 < 0xF && self.reg_NR22.env_dir() {
                                self.current_volume_ch2 += 1;
                            } else if self.current_volume_ch2 > 0x0 && !self.reg_NR22.env_dir() {
                                self.current_volume_ch2 -= 1;
                            }
                        }
                    }

                    if self.reg_NR42.sweep_pace().value() != 0 {
                        if self.period_timer_ch4 > 0 {
                            self.period_timer_ch4 -= 1;
                        }

                        if self.period_timer_ch4 == 0 {
                            self.period_timer_ch4 = self.reg_NR42.sweep_pace().value();

                            if self.current_volume_ch4 < 0xF && self.reg_NR42.env_dir() {
                                self.current_volume_ch4 += 1;
                            } else if self.current_volume_ch4 > 0x0 && !self.reg_NR42.env_dir() {
                                self.current_volume_ch4 -= 1;
                            }
                        }
                    }
                }
            }
        }

        // Frequency timers
        if self.reg_NR52.ch1_on() {
            self.frequency_timer_ch1 += 1;
            if self.frequency_timer_ch1 == 2048 * 4 {
                self.frequency_timer_ch1 =
                    ((u32::from(self.reg_NR14.period()) << 8) | self.reg_NR13 as u32) * 4;
                self.wave_duty_position_ch1 = self.wave_duty_position_ch1.wrapping_add(u3::new(1));
            }
        }

        if self.reg_NR52.ch2_on() {
            self.frequency_timer_ch2 += 1;
            if self.frequency_timer_ch2 == 2048 * 4 {
                self.frequency_timer_ch2 =
                    ((u32::from(self.reg_NR24.period()) << 8) | self.reg_NR23 as u32) * 4;
                self.wave_duty_position_ch2 = self.wave_duty_position_ch2.wrapping_add(u3::new(1));
            }
        }

        if self.reg_NR52.ch3_on() {
            self.frequency_timer_ch3 += 1;
            if self.frequency_timer_ch3 == 2048 * 2 {
                self.frequency_timer_ch3 =
                    ((u32::from(self.reg_NR34.period()) << 8) | self.reg_NR33 as u32) * 2;
                self.wave_duty_position_ch3 = (self.wave_duty_position_ch3 + 1) % 32;

                self.sample_buffer = if self.wave_duty_position_ch3 % 2 == 0 {
                    self.wave_ram[self.wave_duty_position_ch3 as usize / 2].bits(0..4)
                } else {
                    self.wave_ram[self.wave_duty_position_ch3 as usize / 2].bits(4..)
                };
                //self.just_read_ch3 = false;
                self.just_read_ch3 = true; // TODO: figure this nonsense out
                self.just_read_ch3_counter = 0;
            } else if self.just_read_ch3 {
                // self.frequency_timer_ch3
                //                 >= ((u32::from(self.reg_NR34.period()) << 8) | self.reg_NR33 as u32) * 4 + 4
                self.just_read_ch3_counter += 1;
                if self.just_read_ch3_counter == 2 {
                    self.just_read_ch3 = false;
                }
            }
            // } else if self.frequency_timer_ch3 > 2047 * 2 {
            //     self.just_read_ch3 = true;
            //     self.just_read_ch3_counter = 0;
            // }
            // } else {
            //     self.just_read_ch3 = false;
            // }
        } else {
            self.just_read_ch3 = false;
        }

        if self.reg_NR52.ch4_on() {
            self.frequency_timer_ch4 -= 1;
            if self.frequency_timer_ch4 == 0 {
                let divisor: u32 = match self.reg_NR43.clock_divider().value() {
                    0 => 8,
                    1 => 16,
                    2 => 32,
                    3 => 48,
                    4 => 64,
                    5 => 80,
                    6 => 96,
                    7 => 112,
                    _ => panic!("impossible"),
                };
                self.frequency_timer_ch4 = divisor << self.reg_NR43.clock_shift().value();

                let xor_result = self.LFSR.bit(0) ^ self.LFSR.bit(1);
                self.LFSR >>= 1;
                self.LFSR.set_bit(14, xor_result);

                if self.reg_NR43.lsfr_width() {
                    self.LFSR.set_bit(6, xor_result);
                }
            }
        }

        // Channel amplitudes
        if self.reg_NR52.ch1_on() {
            let amp_ch1 = WAVE_PATTERN_DUTIES[self.reg_NR11.wave_duty().value() as usize]
                [self.wave_duty_position_ch1.value() as usize]
                * self.current_volume_ch1;
            if amp_ch1 != self.last_amp_ch1 {
                self.blip_ch1
                    .add_delta(self.output_timer, amp_ch1 as i32 - self.last_amp_ch1 as i32);
                self.last_amp_ch1 = amp_ch1;
            }
        } else if !self.reg_NR52.ch1_on() && self.last_amp_ch1 != 0 {
            self.blip_ch1
                .add_delta(self.output_timer, -(self.last_amp_ch1 as i32));
            self.last_amp_ch1 = 0;
        }

        if self.reg_NR52.ch2_on() {
            let amp_ch2 = WAVE_PATTERN_DUTIES[self.reg_NR21.wave_duty().value() as usize]
                [self.wave_duty_position_ch2.value() as usize]
                * self.current_volume_ch2;
            if amp_ch2 != self.last_amp_ch2 {
                self.blip_ch2
                    .add_delta(self.output_timer, amp_ch2 as i32 - self.last_amp_ch2 as i32);
                self.last_amp_ch2 = amp_ch2;
            }
        } else if !self.reg_NR52.ch2_on() && self.last_amp_ch2 != 0 {
            self.blip_ch2
                .add_delta(self.output_timer, -(self.last_amp_ch2 as i32));
            self.last_amp_ch2 = 0;
        }

        if self.reg_NR52.ch3_on() && self.DAC_ch3_enabled {
            let mut amp_ch3 = self.sample_buffer;
            match self.reg_NR32.output_level().value() {
                0b00 => amp_ch3 >>= 4,
                0b01 => amp_ch3 >>= 0,
                0b10 => amp_ch3 >>= 1,
                0b11 => amp_ch3 >>= 2,
                _ => panic!("impossible"),
            }
            if amp_ch3 != self.last_amp_ch3 {
                self.blip_ch3
                    .add_delta(self.output_timer, amp_ch3 as i32 - self.last_amp_ch3 as i32);
                self.last_amp_ch3 = amp_ch3;
            }
        } else if !(self.reg_NR52.ch3_on() && self.DAC_ch3_enabled) && self.last_amp_ch3 != 0 {
            self.blip_ch3
                .add_delta(self.output_timer, -(self.last_amp_ch3 as i32));
            self.last_amp_ch3 = 0;
        }

        if self.reg_NR52.ch4_on() {
            let amp_ch4 = (!self.LFSR.bit(0)) as u8 * self.current_volume_ch4;
            if amp_ch4 != self.last_amp_ch4 {
                self.blip_ch4
                    .add_delta(self.output_timer, amp_ch4 as i32 - self.last_amp_ch4 as i32);
                self.last_amp_ch4 = amp_ch4;
            }
        } else if !self.reg_NR52.ch4_on() && self.last_amp_ch4 != 0 {
            self.blip_ch4
                .add_delta(self.output_timer, -(self.last_amp_ch4 as i32));
            self.last_amp_ch4 = 0;
        }

        if self.output_timer == self.output_period {
            self.blip_ch1.end_frame(self.output_timer);
            self.blip_ch2.end_frame(self.output_timer);
            self.blip_ch3.end_frame(self.output_timer);
            self.blip_ch4.end_frame(self.output_timer);

            self.output_timer = 0;

            let samples_avail = self.blip_ch1.samples_avail();

            // Input buffers
            let buf_ch1 = &mut [0; OUTPUT_SAMPLE_COUNT as usize];
            self.blip_ch1.read_samples(buf_ch1, false);
            let buf_ch2 = &mut [0; OUTPUT_SAMPLE_COUNT as usize];
            self.blip_ch2.read_samples(buf_ch2, false);
            let buf_ch3 = &mut [0; OUTPUT_SAMPLE_COUNT as usize];
            self.blip_ch3.read_samples(buf_ch3, false);
            let buf_ch4 = &mut [0; OUTPUT_SAMPLE_COUNT as usize];
            self.blip_ch4.read_samples(buf_ch4, false);

            // Output buffer
            let buf_left = &mut [0.0; OUTPUT_SAMPLE_COUNT as usize];
            let buf_right = &mut [0.0; OUTPUT_SAMPLE_COUNT as usize];

            // Process samples
            for i in 0..samples_avail as usize {
                // DACs
                let dac_output_ch1 = if self.DAC_ch1_enabled {
                    (buf_ch1[i] as f32 / 7.5) - 1.0
                } else {
                    0.0
                };
                let dac_output_ch2 = if self.DAC_ch2_enabled {
                    (buf_ch2[i] as f32 / 7.5) - 1.0
                } else {
                    0.0
                };
                let dac_output_ch3 = if self.DAC_ch3_enabled {
                    (buf_ch3[i] as f32 / 7.5) - 1.0
                } else {
                    0.0
                };
                let dac_output_ch4 = if self.DAC_ch4_enabled {
                    (buf_ch4[i] as f32 / 7.5) - 1.0
                } else {
                    0.0
                };

                // Mixing and panning
                let mut sample_left = 0.0;
                if self.reg_NR51.ch1_left() {
                    sample_left += dac_output_ch1;
                }
                if self.reg_NR51.ch2_left() {
                    sample_left += dac_output_ch2;
                }
                if self.reg_NR51.ch3_left() {
                    sample_left += dac_output_ch3;
                }
                if self.reg_NR51.ch4_left() {
                    sample_left += dac_output_ch4;
                }

                sample_left *= (1.0 / 15.0) * 0.25 * self.reg_NR50.left_volume().value() as f32;
                buf_left[i] = sample_left;

                let mut sample_right = 0.0;
                if self.reg_NR51.ch1_right() {
                    sample_right += dac_output_ch1;
                }
                if self.reg_NR51.ch2_right() {
                    sample_right += dac_output_ch2;
                }
                if self.reg_NR51.ch3_right() {
                    sample_right += dac_output_ch3;
                }
                if self.reg_NR51.ch4_right() {
                    sample_right += dac_output_ch4;
                }
                sample_right *= (1.0 / 15.0) * 0.25 * self.reg_NR50.right_volume().value() as f32;
                buf_right[i] = sample_right;
            }

            self.audio_player.add_samples(
                &buf_left[..samples_avail as usize],
                &buf_right[..samples_avail as usize],
            );
        } else {
            self.output_timer += 1;
        }
    }

    pub(crate) fn read(&self, address: u16) -> u8 {
        match address {
            0xFF10 => self.reg_NR10.raw_value() | 0x80,
            0xFF11 => self.reg_NR11.raw_value() | 0x3F,
            0xFF12 => self.reg_NR12.raw_value(),
            0xFF13 => 0xFF,
            0xFF14 => self.reg_NR14.raw_value() | 0xBF,
            0xFF16 => self.reg_NR21.raw_value() | 0x3F,
            0xFF17 => self.reg_NR22.raw_value(),
            0xFF18 => 0xFF,
            0xFF19 => self.reg_NR24.raw_value() | 0xBF,
            0xFF1A => self.reg_NR30.raw_value() | 0x7F,
            0xFF1B => 0xFF,
            0xFF1C => self.reg_NR32.raw_value() | 0x9F,
            0xFF1D => 0xFF,
            0xFF1E => self.reg_NR34.raw_value() | 0xBF,
            0xFF20 => 0xFF,
            0xFF21 => self.reg_NR42.raw_value(),
            0xFF22 => self.reg_NR43.raw_value(),
            0xFF23 => self.reg_NR44.raw_value() | 0xBF,
            0xFF24 => self.reg_NR50.raw_value(),
            0xFF25 => self.reg_NR51.raw_value(),
            0xFF26 => self.reg_NR52.raw_value() | 0x70,
            0xFF30..=0xFF3F => {
                if self.reg_NR52.ch3_on() {
                    if self.just_read_ch3 {
                        self.wave_ram[self.wave_duty_position_ch3 as usize / 2]
                    } else {
                        0xFF
                    }
                } else {
                    self.wave_ram[(address - 0xFF30) as usize]
                }
            }
            _ => 0xFF,
        }
    }

    pub(crate) fn write(&mut self, address: u16, value: u8) {
        match address {
            0xFF10 => {
                if self.reg_NR52.audio_on() {
                    let prev_direction = self.reg_NR10.direction();
                    self.reg_NR10 = NR10::new_with_raw_value(value);
                    if prev_direction && !self.reg_NR10.direction() && self.done_sweep_calc {
                        // Cleared sweep direction bit and a sweep calculation has been made, disable channel
                        self.reg_NR52.set_ch1_on(false);
                    }
                }
            }
            0xFF11 => {
                if self.reg_NR52.audio_on() {
                    self.reg_NR11 = PulseTimerDutyCycle::new_with_raw_value(value);
                }
                self.length_timer_ch1 = 64 - (value.bits(0..6) as u16);
            }
            0xFF12 => {
                if self.reg_NR52.audio_on() {
                    self.reg_NR12 = VolumeEnvelope::new_with_raw_value(value);
                    if value.bits(3..=7) == 0 {
                        self.DAC_ch1_enabled = false;
                        self.reg_NR52.set_ch1_on(false);
                    } else {
                        self.DAC_ch1_enabled = true;
                    }
                }
            }
            0xFF13 => {
                if self.reg_NR52.audio_on() {
                    self.reg_NR13 = value
                }
            }
            0xFF14 => {
                if self.reg_NR52.audio_on() {
                    let prev_length_enable = self.reg_NR14.length_enable();
                    self.reg_NR14 = PeriodHighControl::new_with_raw_value(value);

                    if self.DIV_APU % 2 == 0
                        && !prev_length_enable
                        && self.reg_NR14.length_enable()
                        && self.length_timer_ch1 > 0
                    {
                        self.length_timer_ch1 -= 1;
                        if self.length_timer_ch1 == 0 {
                            self.reg_NR52.set_ch1_on(false);
                        }
                    }

                    if value.bit(7) {
                        if self.length_timer_ch1 == 0 {
                            self.length_timer_ch1 = 64;
                            if self.reg_NR14.length_enable() && self.DIV_APU % 2 == 0 {
                                self.length_timer_ch1 = 63;
                            }
                        }

                        if self.DAC_ch1_enabled {
                            self.reg_NR52.set_ch1_on(true);
                        }

                        self.period_timer_ch1 = self.reg_NR12.sweep_pace().into();
                        self.current_volume_ch1 = self.reg_NR12.initial_volume().into();

                        self.shadow_frequency =
                            (u32::from(self.reg_NR14.period()) << 8) | self.reg_NR13 as u32;
                        self.frequency_timer_ch1 = self.shadow_frequency * 4;

                        if self.reg_NR10.pace().value() > 0 {
                            self.sweep_timer = self.reg_NR10.pace().value();
                        } else {
                            self.sweep_timer = 8;
                        }

                        self.sweep_enabled = self.reg_NR10.pace().value() > 0
                            || self.reg_NR10.individual_step().value() > 0;
                        self.done_sweep_calc = false;

                        if self.reg_NR10.individual_step().value() > 0 {
                            let mut new_frequency =
                                self.shadow_frequency >> self.reg_NR10.individual_step().value();

                            if self.reg_NR10.direction() {
                                new_frequency = self.shadow_frequency - new_frequency;
                                self.done_sweep_calc = true;
                            } else {
                                new_frequency = self.shadow_frequency + new_frequency;
                            }

                            if new_frequency >= 2048 {
                                self.reg_NR52.set_ch1_on(false);
                            }
                        }
                    }
                }
            }
            0xFF16 => {
                if self.reg_NR52.audio_on() {
                    self.reg_NR21 = PulseTimerDutyCycle::new_with_raw_value(value);
                }
                self.length_timer_ch2 = 64 - (value.bits(0..6) as u16);
            }
            0xFF17 => {
                if self.reg_NR52.audio_on() {
                    self.reg_NR22 = VolumeEnvelope::new_with_raw_value(value);
                    if value.bits(3..=7) == 0 {
                        self.DAC_ch2_enabled = false;
                        self.reg_NR52.set_ch2_on(false);
                    } else {
                        self.DAC_ch2_enabled = true;
                    }
                }
            }
            0xFF18 => {
                if self.reg_NR52.audio_on() {
                    self.reg_NR23 = value
                }
            }
            0xFF19 => {
                if self.reg_NR52.audio_on() {
                    let prev_length_enable = self.reg_NR24.length_enable();
                    self.reg_NR24 = PeriodHighControl::new_with_raw_value(value);

                    if self.DIV_APU % 2 == 0
                        && !prev_length_enable
                        && self.reg_NR24.length_enable()
                        && self.length_timer_ch2 > 0
                    {
                        self.length_timer_ch2 -= 1;
                        if self.length_timer_ch2 == 0 {
                            self.reg_NR52.set_ch2_on(false);
                        }
                    }

                    if value.bit(7) {
                        if self.length_timer_ch2 == 0 {
                            self.length_timer_ch2 = 64;
                            if self.reg_NR24.length_enable() && self.DIV_APU % 2 == 0 {
                                self.length_timer_ch2 = 63;
                            }
                        }

                        if self.DAC_ch2_enabled {
                            self.reg_NR52.set_ch2_on(true);
                        }

                        self.period_timer_ch2 = self.reg_NR22.sweep_pace().into();
                        self.current_volume_ch2 = self.reg_NR22.initial_volume().into();

                        self.frequency_timer_ch2 =
                            ((u32::from(self.reg_NR24.period()) << 8) | self.reg_NR23 as u32) * 4;
                    }
                }
            }
            0xFF1A => {
                if self.reg_NR52.audio_on() {
                    self.reg_NR30 = NR30::new_with_raw_value(value);
                    if self.reg_NR30.DAC_on() {
                        self.DAC_ch3_enabled = true;
                    } else {
                        self.DAC_ch3_enabled = false;
                        self.reg_NR52.set_ch3_on(false);
                    }
                }
            }
            0xFF1B => {
                self.length_timer_ch3 = 256 - (value as u16);
            }
            0xFF1C => {
                if self.reg_NR52.audio_on() {
                    self.reg_NR32 = NR32::new_with_raw_value(value)
                }
            }
            0xFF1D => {
                if self.reg_NR52.audio_on() {
                    self.reg_NR33 = value
                }
            }
            0xFF1E => {
                if self.reg_NR52.audio_on() {
                    let prev_length_enable = self.reg_NR34.length_enable();
                    self.reg_NR34 = PeriodHighControl::new_with_raw_value(value);

                    if self.DIV_APU % 2 == 0
                        && !prev_length_enable
                        && self.reg_NR34.length_enable()
                        && self.length_timer_ch3 > 0
                    {
                        self.length_timer_ch3 -= 1;
                        if self.length_timer_ch3 == 0 {
                            self.reg_NR52.set_ch3_on(false);
                        }
                    }

                    if value.bit(7) {
                        if self.reg_NR52.ch3_on()
                            && self.DAC_ch3_enabled
                            && self.frequency_timer_ch3 >= 2047 * 2
                        {
                            let index = ((self.wave_duty_position_ch3 as usize + 1) % 32) / 2;
                            match index {
                                0..=3 => {
                                    self.wave_ram[0] = self.wave_ram[index];
                                }
                                4..=7 => {
                                    self.wave_ram[0] = self.wave_ram[4];
                                    self.wave_ram[1] = self.wave_ram[5];
                                    self.wave_ram[2] = self.wave_ram[6];
                                    self.wave_ram[3] = self.wave_ram[7];
                                }
                                8..=11 => {
                                    self.wave_ram[0] = self.wave_ram[8];
                                    self.wave_ram[1] = self.wave_ram[9];
                                    self.wave_ram[2] = self.wave_ram[10];
                                    self.wave_ram[3] = self.wave_ram[11];
                                }
                                12..=15 => {
                                    self.wave_ram[0] = self.wave_ram[12];
                                    self.wave_ram[1] = self.wave_ram[13];
                                    self.wave_ram[2] = self.wave_ram[14];
                                    self.wave_ram[3] = self.wave_ram[15];
                                }
                                _ => {
                                    panic!("Impossible")
                                }
                            }
                        }

                        if self.length_timer_ch3 == 0 {
                            self.length_timer_ch3 = 256;
                            if self.reg_NR34.length_enable() && self.DIV_APU % 2 == 0 {
                                self.length_timer_ch3 = 255;
                            }
                        }

                        if self.DAC_ch3_enabled {
                            self.reg_NR52.set_ch3_on(true);
                        }

                        self.wave_duty_position_ch3 = 0;

                        self.frequency_timer_ch3 = ((u32::from(self.reg_NR34.period()) << 8)
                            | self.reg_NR33 as u32)
                            .saturating_sub(3)
                            * 2;
                    }
                }
            }
            0xFF20 => {
                if self.reg_NR52.audio_on() {
                    self.reg_NR41 = NR41::new_with_raw_value(value);
                }
                self.length_timer_ch4 = 64 - (value.bits(0..6) as u16);
            }
            0xFF21 => {
                if self.reg_NR52.audio_on() {
                    self.reg_NR42 = VolumeEnvelope::new_with_raw_value(value);
                    if value.bits(3..=7) == 0 {
                        self.DAC_ch4_enabled = false;
                        self.reg_NR52.set_ch4_on(false);
                    } else {
                        self.DAC_ch4_enabled = true;
                    }
                }
            }
            0xFF22 => {
                if self.reg_NR52.audio_on() {
                    self.reg_NR43 = NR43::new_with_raw_value(value)
                }
            }
            0xFF23 => {
                if self.reg_NR52.audio_on() {
                    let prev_length_enable = self.reg_NR44.length_enable();
                    self.reg_NR44 = NR44::new_with_raw_value(value);

                    if self.DIV_APU % 2 == 0
                        && !prev_length_enable
                        && self.reg_NR44.length_enable()
                        && self.length_timer_ch4 > 0
                    {
                        self.length_timer_ch4 -= 1;
                        if self.length_timer_ch4 == 0 {
                            self.reg_NR52.set_ch4_on(false);
                        }
                    }

                    if value.bit(7) {
                        if self.length_timer_ch4 == 0 {
                            self.length_timer_ch4 = 64;
                            if self.reg_NR44.length_enable() && self.DIV_APU % 2 == 0 {
                                self.length_timer_ch4 = 63;
                            }
                        }

                        if self.DAC_ch4_enabled {
                            self.reg_NR52.set_ch4_on(true);
                        }
                    }
                    self.period_timer_ch4 = self.reg_NR42.sweep_pace().into();
                    self.current_volume_ch4 = self.reg_NR42.initial_volume().into();

                    let divisor: u32 = match self.reg_NR43.clock_divider().value() {
                        0 => 8,
                        1 => 16,
                        2 => 32,
                        3 => 48,
                        4 => 64,
                        5 => 80,
                        6 => 96,
                        7 => 112,
                        _ => panic!("impossible"),
                    };
                    self.frequency_timer_ch4 = divisor << self.reg_NR43.clock_shift().value();
                    self.LFSR = 0x7FFF;
                }
            }
            0xFF24 => {
                if self.reg_NR52.audio_on() {
                    self.reg_NR50 = NR50::new_with_raw_value(value)
                }
            }
            0xFF25 => {
                if self.reg_NR52.audio_on() {
                    self.reg_NR51 = NR51::new_with_raw_value(value)
                }
            }
            0xFF26 => {
                if !self.reg_NR52.audio_on() && value.bit(7) {
                    // APU went from off to on
                    self.DIV_APU = 1;
                }

                self.reg_NR52.set_audio_on(value.bit(7));

                if !self.reg_NR52.audio_on() {
                    // Audio turned off, clear all APU registers
                    self.reg_NR10 = NR10::ZERO;
                    self.reg_NR11 = PulseTimerDutyCycle::new_with_raw_value(0);
                    self.reg_NR12 = VolumeEnvelope::ZERO;
                    self.DAC_ch1_enabled = false;
                    self.reg_NR52.set_ch1_on(false);
                    self.reg_NR13 = 0;
                    self.reg_NR14 = PeriodHighControl::ZERO;
                    self.reg_NR21 = PulseTimerDutyCycle::new_with_raw_value(0);
                    self.reg_NR22 = VolumeEnvelope::ZERO;
                    self.reg_NR23 = 0;
                    self.reg_NR24 = PeriodHighControl::ZERO;
                    self.reg_NR30 = NR30::ZERO;
                    self.reg_NR32 = NR32::ZERO;
                    self.reg_NR33 = 0;
                    self.reg_NR34 = PeriodHighControl::ZERO;
                    self.reg_NR42 = VolumeEnvelope::ZERO;
                    self.reg_NR43 = NR43::ZERO;
                    self.reg_NR44 = NR44::ZERO;
                    self.reg_NR50 = NR50::ZERO;
                    self.reg_NR51 = NR51::ZERO;
                    self.reg_NR52 = NR52::ZERO;
                    self.sample_buffer = 0;
                }
            }
            0xFF30..=0xFF3F => {
                if self.reg_NR52.ch3_on() {
                    if self.just_read_ch3 {
                        self.wave_ram[self.wave_duty_position_ch3 as usize / 2] = value;
                    }
                } else {
                    self.wave_ram[(address - 0xFF30) as usize] = value;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn skip_bootrom(&mut self) {
        // Internal state
        self.length_timer_ch1 = 64;
        self.DAC_ch1_enabled = true;
        self.frequency_timer_ch1 = 7960;
        self.DIV_APU = 66;
        self.period_timer_ch1 = 2;
        self.sweep_enabled = false;
        self.shadow_frequency = 1985;
        self.sweep_timer = 1;

        // Register state
        self.reg_NR11 = PulseTimerDutyCycle::new_with_raw_value(0x80);
        self.reg_NR12 = VolumeEnvelope::new_with_raw_value(0xF3);
        self.reg_NR50 = NR50::new_with_raw_value(0x77);
        self.reg_NR51 = NR51::new_with_raw_value(0xF3);
        self.reg_NR52.set_ch1_on(true);
    }
}
