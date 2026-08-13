fn main() {
    // sherpa-rs-sys копирует libonnxruntime/libsherpa-onnx-* рядом с итоговым
    // бинарником (target/debug, target/release), но не прописывает rpath —
    // без этого macOS не находит их при обычном запуске (только вручную через
    // DYLD_LIBRARY_PATH). Добавляем @executable_path, чтобы бинарник искал
    // динамические библиотеки в своей же папке.
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
    }
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    }
    tauri_build::build()
}
