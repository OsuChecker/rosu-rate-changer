use criterion::{criterion_group, criterion_main, Criterion};
use std::fs;

fn bench_change_audio_speed(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_processing");
    group.sample_size(10);
    let output_path = "./resources/decoy.ogg";

    for speed in [ 0.9, 1.1, 1.5, 2.0].iter() {
        group.bench_with_input(
            format!("speed_{}", speed),
            speed,
            |b, &speed| {
                b.iter(|| {
                    rosu_rate_changer::change_audio_speed(
                        "./resources/decoy.mp3",
                        output_path,
                        speed
                    ).unwrap();

                    // Nettoyage après chaque itération
                    if fs::metadata(output_path).is_ok() {
                        fs::remove_file(output_path).unwrap();
                    }
                })
            }
        );
    }

    group.finish();

    // Nettoyage final au cas où
    if fs::metadata(output_path).is_ok() {
        fs::remove_file(output_path).unwrap_or_default();
    }
}

criterion_group!(benches, bench_change_audio_speed);
criterion_main!(benches);