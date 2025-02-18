mod audio;
mod osu;
mod constants;

use std::fs;
use rosu_map::Beatmap;
pub use audio::*;
pub use osu::*;
use std::path::{Path, PathBuf};


fn rate_map(osu_file_path: &str, rate : f32) -> eyre::Result<()>{
    let mut map: Beatmap = rosu_map::Beatmap::from_path(osu_file_path)?;
    let audio_path = map.audio_file.clone();
    let audio_output_path = PathBuf::from(format!(
        "{}_{}.ogg",
        Path::new(&audio_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| eyre::eyre!("Invalid audio file name"))?,
        rate
    )).to_str().unwrap().to_string();

    change_osu_speed(&mut map, osu_file_path, 1.5, &audio_output_path)?;


    let osu_file_dir = Path::new(&osu_file_path)
        .parent()
        .ok_or_else(|| eyre::eyre!("Invalid osu file directory"))?;
    let audio_path = osu_file_dir.join(&map.audio_file);
    let audio_output_path = audio_path.with_file_name(format!(
        "{}_{}.ogg",
        audio_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| eyre::eyre!("Invalid audio file name"))?,
        rate
    )).to_str().unwrap().to_string();

    change_audio_speed(&audio_path.to_str().unwrap(), &audio_output_path, rate)?;
    Ok(())
}

