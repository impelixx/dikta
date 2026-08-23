fn main() {
    // sherpa-rs-sys копирует libonnxruntime/libsherpa-onnx-* рядом с итоговым
    // бинарником (target/debug, target/release), но не прописывает rpath —
    // без этого macOS не находит их при обычном запуске (только вручную через
    // DYLD_LIBRARY_PATH). @executable_path нужен для cargo build/run (dylib
    // рядом с бинарником) и для test-бинарников из target/debug/deps/ (там
    // dylib лежит на уровень выше — target/debug/). @executable_path/../Frameworks
    // нужен для собранного .app: Tauri копирует dylib'ы, перечисленные в
    // bundle.macOS.frameworks (tauri.conf.json), в Contents/Frameworks, а
    // исполняемый файл лежит в Contents/MacOS.
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/..");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
    }
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    }
    tauri_build::build()
}
