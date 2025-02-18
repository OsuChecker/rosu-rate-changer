use std::path::Path;
use rosu_map;
use rosu_map::Beatmap;
use rosu_map::section::hit_objects::HitObjectKind;


macro_rules! update_points {
    ($points:expr) =>
    {
        for point in $points {
            update_time(&mut point.time);
        }
    };
}


pub fn change_osu_speed(map: &mut Beatmap, input_path: &str, rate: f32, audio_path: &str) -> eyre::Result<()> {
    let multiplier: f64 = 1.0f64 / rate as f64;
    map.audio_file = audio_path.to_string();
    map.version = format!("{} {:.2}x", map.version, rate);
    let update_time = |time: &mut f64| *time = (*time * multiplier).floor();
    let control_points = &mut map.control_points;
    for point in &mut control_points.timing_points {
        update_time(&mut point.time);
        update_time(&mut point.beat_len);
    }

    update_points!(&mut control_points.timing_points);
    update_points!(&mut control_points.difficulty_points);
    update_points!(&mut control_points.effect_points);
    update_points!(&mut control_points.sample_points);

    for hit_object in &mut map.hit_objects {
        update_time(&mut hit_object.start_time);
        if let HitObjectKind::Hold(hold) = &mut hit_object.kind {
            update_time(&mut hold.duration);
        }
        else if let HitObjectKind::Spinner(spinner) = &mut hit_object.kind {
            update_time(&mut spinner.duration);
        }
        else if let HitObjectKind::Slider(slider) = &mut hit_object.kind{
            slider.velocity *= multiplier;
        }
    }


    let path = Path::new(input_path);
    let output_path = path.with_file_name(format!(
        "{}_{}x.osu",
        path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| eyre::eyre!("Folder is invalid"))?,
        rate
    ));
    map.encode_to_path(output_path)?;
    Ok(())
}