use dikta_lib::asr::{CtcPaths, Recognizer};
use std::env;
use hound::WavReader;
use std::time::Instant;

fn read_wav(path: &std::path::Path) -> (Vec<f32>, f64) {
    let mut reader = WavReader::open(path).expect("wav open failed");
    let spec = reader.spec();
    let samples: Vec<f32> = if spec.sample_format == hound::SampleFormat::Float {
        reader.samples::<f32>().map(|s| s.unwrap()).collect()
    } else {
        reader.samples::<i16>().map(|s| s.unwrap() as f32 / i16::MAX as f32).collect()
    };
    let duration = samples.len() as f64 / spec.sample_rate as f64;
    (samples, duration)
}

fn main() {
    let base = dirs::data_dir().unwrap().join("dikta/models/sherpa-onnx-nemo-fast-conformer-ctc-be-de-en-es-fr-hr-it-pl-ru-uk-20k-int8");
    let model = base.join("model.int8.onnx");
    let tokens = base.join("tokens.txt");
    let dim: i32 = env::var("FEAT_DIM").ok().and_then(|s| s.parse().ok()).unwrap_or(64);
    println!("feature_dim = {dim}");
    let load_start = Instant::now();
    let rec = Recognizer::new_ctc_with_dim(
        CtcPaths { model: model.to_str().unwrap(), tokens: tokens.to_str().unwrap() },
        2,
        dim,
    ).expect("init failed");
    println!("load: {}ms\n", load_start.elapsed().as_millis());

    for wav in ["ru-russian.wav", "en-english.wav", "fr-french.wav", "it-italian.wav", "uk-ukrainian.wav"] {
        let path = base.join("test_wavs").join(wav);
        if !path.exists() { continue; }
        let (samples, duration) = read_wav(&path);
        let t0 = Instant::now();
        let text = rec.decode(&samples);
        let dt = t0.elapsed().as_secs_f64();
        println!("=== {wav} ({duration:.2}s, RTF={:.4}) ===\n{text}\n", dt / duration);
    }
}
