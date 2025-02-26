mod audio;
mod osu;
mod constants;

use std::fs;
use rosu_map::Beatmap;
pub use audio::*;
pub use osu::*;
use std::path::{Path, PathBuf};


pub fn rate_map_from_beatmap(map: &mut Beatmap, osu_file_path : &str,  rate : f32) -> eyre::Result<()>{

    let audio_path = map.audio_file.clone();
    let audio_output_path = PathBuf::from(format!(
        "{}_{}.ogg",
        Path::new(&audio_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| eyre::eyre!("Invalid audio file name"))?,
        rate
    )).to_str().unwrap().to_string();
    change_osu_speed(map, osu_file_path, rate, &audio_output_path)?;

    let osu_file_dir = Path::new(&osu_file_path)
        .parent()
        .ok_or_else(|| eyre::eyre!("Invalid osu file directory"))?;
    let audio_path = osu_file_dir.join(audio_path);
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


pub fn rate_map(osu_file_path: &str, rate : f32) -> eyre::Result<()>{
    let mut map: Beatmap = rosu_map::Beatmap::from_path(osu_file_path)?;
    rate_map_from_beatmap(&mut map,osu_file_path,rate)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_map_different_speeds() -> eyre::Result<()> {
        // Premier test avec une vitesse de 1.5
        rate_map("./resources/test.osu", 1.5)?;

        // Vérification que les fichiers de sortie existent
        assert!(Path::new("./resources/test_1.5x.osu").exists());
        assert!(Path::new("./resources/decoy_1.5.ogg").exists());

        // Deuxième test avec une vitesse de 0.75
        rate_map("./resources/test.osu", 0.75)?;

        // Vérification que les fichiers de sortie existent
        assert!(Path::new("./resources/test_0.75x.osu").exists());
        assert!(Path::new("./resources/decoy_0.75.ogg").exists());

        Ok(())
    }
}
