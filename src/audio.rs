use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use std::fs;
use std::io::Write;
use hound;
use symphonia::core::audio::AudioBufferRef;
use symphonia::core::audio::Signal;
use vorbis_encoder::Encoder as VorbisEncoder;
use rosu_map;
use crate::constants::{BITS_PER_SAMPLE, BUFFER_SIZE, COMPRESSION_FACTOR, DEFAULT_SAMPLE_RATE, F_CUTOFF, OVERSAMPLING_FACTOR, SINC_LEN};

pub fn change_audio_speed(input_path: &str, output_path: &str, speed: f32) -> eyre::Result<()> {
    let file = std::fs::File::open(input_path)?;
    let media_source = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = symphonia::core::probe::Hint::new();
    match input_path {
        p if p.ends_with(".mp3") => { hint.with_extension("mp3"); }
        p if p.ends_with(".wav") => { hint.with_extension("wav"); }
        _ => {}
    };


    let probe = symphonia::default::get_probe()
        .format(&hint, media_source, &Default::default(), &Default::default())?;

    let mut format = probe.format;
    let track = format.default_track().unwrap();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &Default::default())?;

    let sample_rate = track.codec_params.sample_rate.unwrap_or(DEFAULT_SAMPLE_RATE);
    let channels = track.codec_params.channels.unwrap().count();
    let output_sample_rate = DEFAULT_SAMPLE_RATE;

    // Augmentation de la taille du buffer pour réduire le nombre d'opérations
    const LARGER_BUFFER_SIZE: usize = BUFFER_SIZE * 4;

    let mut resampler = SincFixedIn::<f32>::new(
        (output_sample_rate as f64 / sample_rate as f64) * (1.0 / speed as f64),
        1.0,
        SincInterpolationParameters {
            sinc_len: SINC_LEN,
            f_cutoff: F_CUTOFF as f32,
            interpolation: SincInterpolationType::Linear, // Plus rapide que Cubic
            oversampling_factor: OVERSAMPLING_FACTOR,
            window: WindowFunction::Hann // Plus rapide que BlackmanHarris2
        },
        LARGER_BUFFER_SIZE,
        channels
    )?;

    let mut encoder = VorbisEncoder::new(
        channels as u32,
        output_sample_rate as u64,
        0.3 // Qualité plus basse = encodage plus rapide
    ).map_err(|e| eyre::eyre!("Erreur lors de la création de l'encodeur Vorbis: {}", e))?;

    // Utilisation de BufWriter pour des écritures plus efficaces
    let output_file = std::fs::File::create(output_path)?;
    let mut output_file = std::io::BufWriter::with_capacity(65536, output_file);

    // Préallocation avec des capacités optimisées
    let mut input_buffer = vec![Vec::with_capacity(LARGER_BUFFER_SIZE); channels];
    let mut output_buffer = vec![Vec::with_capacity(LARGER_BUFFER_SIZE); channels];
    let mut accumulated_samples = vec![Vec::with_capacity(LARGER_BUFFER_SIZE * 2); channels];
    let mut encoding_buffer = Vec::with_capacity(LARGER_BUFFER_SIZE * channels * 2);

    while let Ok(packet) = format.next_packet() {
        let decoded = decoder.decode(&packet)?;
        let frames = decoded.frames();

        // Traitement optimisé des échantillons
        match &decoded {
            AudioBufferRef::F32(buf) => {
                for ch in 0..channels {
                    accumulated_samples[ch].extend_from_slice(buf.chan(ch));
                }
            },
            AudioBufferRef::S16(buf) => {
                for ch in 0..channels {
                    let samples = buf.chan(ch);
                    accumulated_samples[ch].extend(
                        samples.iter().map(|&s| s as f32 / 32768.0)
                    );
                }
            },
            _ => return Err(eyre::eyre!("Format audio non supporté")),
        }

        // Traitement par plus grands blocs
        while accumulated_samples[0].len() >= LARGER_BUFFER_SIZE {
            for ch in 0..channels {
                input_buffer[ch].clear();
                input_buffer[ch].extend_from_slice(&accumulated_samples[ch][..LARGER_BUFFER_SIZE]);
                accumulated_samples[ch].drain(..LARGER_BUFFER_SIZE);
            }

            output_buffer = resampler.process(&input_buffer, None)?;

            // Optimisation de la conversion et compression
            encoding_buffer.clear();
            for frame in 0..output_buffer[0].len() {
                for ch in 0..channels {
                    let sample = output_buffer[ch][frame];
                    // Simplification de la compression
                    let compressed = (sample * COMPRESSION_FACTOR).clamp(-COMPRESSION_FACTOR, COMPRESSION_FACTOR);
                    encoding_buffer.push((compressed * 32767.0) as i16);
                }
            }

            let encoded_data = encoder.encode(&encoding_buffer)
                .map_err(|e| eyre::eyre!("Erreur lors de l'encodage: {}", e))?;
            output_file.write_all(&encoded_data)?;
        }
    }

    // Traitement final des échantillons restants
    if !accumulated_samples[0].is_empty() {
        let remaining_len = accumulated_samples[0].len();
        for ch in 0..channels {
            input_buffer[ch].clear();
            input_buffer[ch].extend_from_slice(&accumulated_samples[ch][..remaining_len]);
            input_buffer[ch].resize(LARGER_BUFFER_SIZE, 0.0);
        }

        if let Ok(final_output) = resampler.process(&input_buffer, None) {
            encoding_buffer.clear();
            encoding_buffer.reserve(final_output[0].len() * channels);

            for frame in 0..final_output[0].len() {
                for ch in 0..channels {
                    let compressed = (final_output[ch][frame] * COMPRESSION_FACTOR)
                        .clamp(-COMPRESSION_FACTOR, COMPRESSION_FACTOR);
                    encoding_buffer.push((compressed * 32767.0) as i16);
                }
            }

            let encoded_data = encoder.encode(&encoding_buffer)
                .map_err(|e| eyre::eyre!("Erreur lors de l'encodage: {}", e))?;
            output_file.write_all(&encoded_data)?;
        }
    }

    let final_data = encoder.flush()
        .map_err(|e| eyre::eyre!("Erreur lors de la finalisation: {}", e))?;
    output_file.write_all(&final_data)?;
    output_file.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_change_audio_speed_direct_ogg() -> eyre::Result<()> {
        // Vérifier les deux formats possibles
        let input_paths = ["./resources/decoy.mp3", "./resources/decoy.wav"];
        let input_path = input_paths.iter()
            .find(|&path| Path::new(path).exists())
            .ok_or_else(|| eyre::eyre!("Aucun fichier audio trouvé dans ./resources/"))?;

        let output_path = "./resources/decoy_speed_test.ogg";

        println!("Utilisation du fichier d'entrée : {}", input_path);

        // Supprimer le fichier de sortie s'il existe déjà
        if Path::new(output_path).exists() {
            fs::remove_file(output_path)?;
        }

        // Test avec une vitesse de 1.5x
        println!("Début de la conversion...");
        change_audio_speed(input_path, output_path, 1.1)?;
        println!("Conversion terminée");

        // Vérifier que le fichier de sortie a été créé
        assert!(Path::new(output_path).exists(), "Le fichier de sortie n'a pas été créé");

        // Vérifier que le fichier de sortie n'est pas vide
        let metadata = fs::metadata(output_path)?;
        assert!(metadata.len() > 0, "Le fichier de sortie est vide");

        // Nettoyer après le test
        //fs::remove_file(output_path)?;

        Ok(())
    }
}