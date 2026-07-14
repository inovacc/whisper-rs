#[test]
#[ignore = "run once to (re)generate the fixture"]
fn generate_sine_fixture() {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 8000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    std::fs::create_dir_all("tests/fixtures").unwrap();
    let mut w = hound::WavWriter::create("tests/fixtures/sine_8k_stereo.wav", spec).unwrap();
    for n in 0..8000 {
        let s = ((n as f32 / 8000.0) * 440.0 * std::f32::consts::TAU).sin();
        let v = (s * i16::MAX as f32) as i16;
        w.write_sample(v).unwrap();
        w.write_sample(v).unwrap();
    }
    w.finalize().unwrap();
}

#[test]
#[ignore = "run once to (re)generate the fixture"]
fn generate_sine_f32_fixture() {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    std::fs::create_dir_all("tests/fixtures").unwrap();
    let mut w = hound::WavWriter::create("tests/fixtures/sine_f32_16k.wav", spec).unwrap();
    for n in 0..16000 {
        let s = ((n as f32 / 16000.0) * 440.0 * std::f32::consts::TAU).sin();
        w.write_sample(s).unwrap();
    }
    w.finalize().unwrap();
}
