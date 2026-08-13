//! Автономная проверка ASR-пайплайна: cargo run --example decode_test
//! Декодирует тестовые WAV из resources/model и печатает распознанный текст,
//! чтобы подтвердить, что GigaAM-CTC действительно понимает русскую речь.
use dikta_lib::asr::GigaAmRecognizer;
use hound::WavReader;

fn read_wav(path: &str) -> Vec<f32> {
    let mut reader = WavReader::open(path).expect("wav open failed");
    assert_eq!(reader.spec().sample_rate, 16000, "ожидается 16kHz");
    reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect()
}

fn main() {
    let base = "resources/model/sherpa-onnx-nemo-ctc-giga-am-russian-2024-10-24";
    let model = format!("{base}/model.int8.onnx");
    let tokens = format!("{base}/tokens.txt");

    let recognizer = GigaAmRecognizer::new(&model, &tokens, 2).expect("recognizer init failed");

    for wav in ["example.wav", "long_example.wav"] {
        let path = format!("{base}/test_wavs/{wav}");
        let samples = read_wav(&path);
        let text = recognizer.decode(&samples);
        println!("=== {wav} ===\n{text}\n");
    }
}
