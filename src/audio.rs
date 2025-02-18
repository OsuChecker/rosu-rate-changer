use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use std::fs;
use std::io::Write;
use hound;
use symphonia::core::audio::AudioBufferRef;
use symphonia::core::audio::Signal;
use vorbis_encoder::Encoder as VorbisEncoder;
use rosu_map;
use crate::constants::{BITS_PER_SAMPLE, BUFFER_SIZE, COMPRESSION_FACTOR, DEFAULT_SAMPLE_RATE, F_CUTOFF, OVERSAMPLING_FACTOR, SINC_LEN};

pub fn change_audio_speed_wav(input_path: &str, output_path: &str, speed: f32) -> eyre::Result<()> {
    let file = std::fs::File::open(input_path)?;
    let media_source = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = symphonia::core::probe::Hint::new();
    if input_path.ends_with(".mp3") {
        hint.with_extension("mp3");
    } else if input_path.ends_with(".wav") {
        hint.with_extension("wav");
    }

    let probe = symphonia::default::get_probe()
        .format(&hint, media_source, &Default::default(), &Default::default())?;

    let mut format = probe.format;
    let track = format.default_track().unwrap();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &Default::default())?;

    let sample_rate = track.codec_params.sample_rate.unwrap_or(DEFAULT_SAMPLE_RATE);
    let channels = track.codec_params.channels.unwrap().count();
    let output_sample_rate = DEFAULT_SAMPLE_RATE;

    let mut resampler = SincFixedIn::<f32>::new(
        (output_sample_rate as f64 / sample_rate as f64) * (1.0 / speed as f64),
        1.0,
        SincInterpolationParameters {
            sinc_len: SINC_LEN,
            f_cutoff: F_CUTOFF as f32,
            interpolation: SincInterpolationType::Cubic,
            oversampling_factor: OVERSAMPLING_FACTOR,
            window: WindowFunction::BlackmanHarris2
        },
        BUFFER_SIZE,
        channels
    )?;

    let mut output_file = hound::WavWriter::create(
        output_path,
        hound::WavSpec {
            channels: channels as u16,
            sample_rate: output_sample_rate,
            bits_per_sample: BITS_PER_SAMPLE,
            sample_format: hound::SampleFormat::Int,
        }
    )?;

    // Préallocation des buffers avec la taille appropriée
    let mut input_buffer = vec![Vec::with_capacity(BUFFER_SIZE); channels];
    let mut output_buffer = vec![Vec::new(); channels];
    let mut accumulated_samples: Vec<Vec<f32>> = vec![Vec::with_capacity(BUFFER_SIZE); channels];

    while let Ok(packet) = format.next_packet() {
        let decoded = decoder.decode(&packet)?;
        let frames = decoded.frames();

        match &decoded {
            AudioBufferRef::F32(buf) => {
                for frame in 0..frames {
                    for ch in 0..channels {
                        accumulated_samples[ch].push(*buf.chan(ch).get(frame).unwrap_or(&0.0));
                    }
                }
            },
            AudioBufferRef::S16(buf) => {
                for frame in 0..frames {
                    for ch in 0..channels {
                        accumulated_samples[ch].push(
                            *buf.chan(ch).get(frame).unwrap_or(&0) as f32 / 32768.0
                        );
                    }
                }
            },
            _ => return Err(eyre::eyre!("Format audio non supporté")),
        }

        while accumulated_samples[0].len() >= BUFFER_SIZE {
            for ch in 0..channels {
                input_buffer[ch].clear();
                input_buffer[ch].extend_from_slice(&accumulated_samples[ch][..BUFFER_SIZE]);
                accumulated_samples[ch].drain(..BUFFER_SIZE);
            }

            output_buffer = resampler.process(&input_buffer, None)?;

            for frame in 0..output_buffer[0].len() {
                for ch in 0..channels {
                    let sample = output_buffer[ch][frame];
                    // Compression dynamique utilisant la constante
                    let compressed = if sample > 0.0 {
                        (sample * COMPRESSION_FACTOR).min(COMPRESSION_FACTOR)
                    } else {
                        (sample * COMPRESSION_FACTOR).max(-COMPRESSION_FACTOR)
                    };
                    let sample_i16 = (compressed * 32767.0) as i16;
                    output_file.write_sample(sample_i16)?;
                }
            }
        }
    }

    if !accumulated_samples[0].is_empty() {
        for ch in 0..channels {
            input_buffer[ch].clear();
            input_buffer[ch].extend_from_slice(&accumulated_samples[ch]);
            input_buffer[ch].resize(BUFFER_SIZE, 0.0);
        }

        if let Ok(final_output) = resampler.process(&input_buffer, None) {
            for frame in 0..final_output[0].len() {
                for ch in 0..channels {
                    let sample = final_output[ch][frame];
                    let compressed = if sample > 0.0 {
                        (sample * COMPRESSION_FACTOR).min(COMPRESSION_FACTOR)
                    } else {
                        (sample * COMPRESSION_FACTOR).max(-COMPRESSION_FACTOR)
                    };
                    let sample_i16 = (compressed * 32767.0) as i16;
                    output_file.write_sample(sample_i16)?;
                }
            }
        }
    }

    output_file.finalize()?;
    Ok(())
}

pub fn change_audio_speed(input_path: &str, output_path: &str, speed: f32) -> eyre::Result<()> {
    let temp_wav = format!("{}.temp.wav", output_path);
    change_audio_speed_wav(input_path, &temp_wav, speed)?;
    convert_wav_to_ogg(&temp_wav, output_path)?;
    fs::remove_file(&temp_wav)?;
    Ok(())
}

pub fn convert_wav_to_ogg(input_wav: &str, output_ogg: &str) -> eyre::Result<()> {
    let mut reader = hound::WavReader::open(input_wav)?;
    let spec = reader.spec();

    let mut encoder = VorbisEncoder::new(
        spec.channels as u32,
        spec.sample_rate as u64,
        0.5
    ).map_err(|e| eyre::eyre!("Erreur lors de la création de l'encodeur Vorbis: {}", e))?;

    let mut output_file = std::fs::File::create(output_ogg)?;
    const CHUNK_SIZE: usize = 8192;
    let mut buffer: Vec<i16> = Vec::with_capacity(CHUNK_SIZE);
    loop {
        buffer.clear();
        for sample in reader.samples::<i16>().take(CHUNK_SIZE) {
            match sample {
                Ok(s) => buffer.push(s),
                Err(e) => {
                    if buffer.is_empty() {
                        return Err(eyre::eyre!("Erreur de lecture WAV: {}", e));
                    }
                    break;
                }
            }
        }
        if buffer.is_empty() {
            break;
        }
        let encoded_data = encoder.encode(&buffer)
            .map_err(|e| eyre::eyre!("Erreur lors de l'encodage: {}", e))?;
        output_file.write_all(&encoded_data)?;
    }
    let final_data = encoder.flush()
        .map_err(|e| eyre::eyre!("Erreur lors de la finalisation: {}", e))?;
    output_file.write_all(&final_data)?;

    Ok(())
}

