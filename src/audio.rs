use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleRate, Stream};
use log::{log, Level};
use std::sync::{Arc, Mutex};

pub(crate) struct AudioPlayer {
    buffer: Arc<Mutex<Vec<(f32, f32)>>>,
    pub(crate) sample_rate: cpal::SampleRate,
    stream: Option<Stream>,
}

impl AudioPlayer {
    pub(crate) fn new() -> Self {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("No output device available");

        let wanted_samplerate = SampleRate(44100);
        let supported_configs_range = match device.supported_output_configs() {
            Ok(config_range) => config_range,
            Err(_) => return Self::dummy(),
        };
        let mut supported_config = None;
        for config in supported_configs_range {
            if config.channels() == 2 && config.sample_format() == cpal::SampleFormat::F32 {
                if wanted_samplerate >= config.min_sample_rate()
                    && wanted_samplerate <= config.max_sample_rate()
                {
                    supported_config = Some(config.with_sample_rate(wanted_samplerate));
                } else {
                    supported_config = Some(config.with_max_sample_rate());
                }
            }
        }

        if supported_config.is_none() {
            panic!();
        }

        let supported_config = supported_config.unwrap();
        let shared_buffer = Arc::new(Mutex::new(Vec::new()));
        let stream_buffer = shared_buffer.clone();

        let stream = device
            .build_output_stream(
                &supported_config.config(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    cpal_thread(data, &stream_buffer)
                },
                move |err| log!(Level::Error, "{}", err),
                None,
            )
            .expect("Failed to build output stream");

        stream.play().expect("Failed to start stream");

        AudioPlayer {
            buffer: shared_buffer,
            sample_rate: supported_config.sample_rate(),
            stream: Some(stream),
        }
    }

    pub(crate) fn dummy() -> Self {
        let shared_buffer = Arc::new(Mutex::new(Vec::new()));
        AudioPlayer {
            buffer: shared_buffer,
            sample_rate: SampleRate(44100),
            stream: None,
        }
    }

    pub(crate) fn add_samples(&mut self, buf_left: &[f32], buf_right: &[f32]) {
        let mut buffer = self.buffer.lock().unwrap();

        for (l, r) in buf_left.iter().zip(buf_right) {
            if buffer.len() < self.sample_rate.0 as usize {
                buffer.push((*l, *r));
            }
        }
    }

    pub(crate) fn underflowed(&self) -> bool {
        self.buffer.lock().unwrap().is_empty()
    }
}

fn cpal_thread<T: Sample + FromSample<f32>>(
    outbuffer: &mut [T],
    audio_buffer: &Arc<Mutex<Vec<(f32, f32)>>>,
) {
    let mut inbuffer = audio_buffer.lock().unwrap();
    let outlen = inbuffer.len().min(outbuffer.len() / 2);
    for (i, (in_l, in_r)) in inbuffer.drain(..outlen).enumerate() {
        outbuffer[i * 2] = T::from_sample(in_l);
        outbuffer[i * 2 + 1] = T::from_sample(in_r);
    }
}
