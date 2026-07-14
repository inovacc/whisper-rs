use std::env;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let wcpp = root.join("vendor/whisper.cpp");
    let ggml = wcpp.join("ggml");

    let target = env::var("TARGET").unwrap_or_default();
    let is_msvc = target.contains("msvc");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .include(wcpp.join("include"))
        .include(ggml.join("include"))
        .include(ggml.join("src"))
        .include(ggml.join("src/ggml-cpu"))
        .define("GGML_USE_CPU", None)
        .warnings(false);

    if is_msvc {
        build.define("_CRT_SECURE_NO_WARNINGS", None);
        build.flag_if_supported("/utf-8");
    } else {
        build.flag_if_supported("-pthread");
    }

    build
        // whisper.cpp core
        .file(wcpp.join("src/whisper.cpp"))
        // ggml-base
        .file(ggml.join("src/ggml.c"))
        .file(ggml.join("src/ggml-alloc.c"))
        .file(ggml.join("src/ggml-backend.cpp"))
        .file(ggml.join("src/ggml-opt.cpp"))
        .file(ggml.join("src/ggml-threading.cpp"))
        .file(ggml.join("src/ggml-quants.c"))
        // ggml backend registry (CPU-only via GGML_USE_CPU guard)
        .file(ggml.join("src/ggml-backend-reg.cpp"))
        // ggml CPU backend
        .file(ggml.join("src/ggml-cpu/ggml-cpu.c"))
        .file(ggml.join("src/ggml-cpu/ggml-cpu.cpp"))
        .file(ggml.join("src/ggml-cpu/ggml-cpu-aarch64.cpp"))
        .file(ggml.join("src/ggml-cpu/ggml-cpu-quants.c"))
        .file(ggml.join("src/ggml-cpu/ggml-cpu-traits.cpp"));

    build.compile("whisper");

    if is_msvc {
        println!("cargo:rustc-link-lib=dylib=advapi32");
    }

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=vendor/whisper.cpp/include/whisper.h");

    let mut bindgen_builder = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("--target={target}"))
        .clang_arg(format!("-I{}", wcpp.join("include").display()))
        .clang_arg(format!("-I{}", ggml.join("include").display()))
        .allowlist_function("whisper_.*")
        .allowlist_type("whisper_.*")
        .allowlist_var("WHISPER_.*");

    if is_msvc {
        bindgen_builder = bindgen_builder.clang_arg("-fms-compatibility");
    }

    let bindings = bindgen_builder.generate().expect("bindgen failed to generate whisper.cpp bindings");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings.write_to_file(out.join("bindings.rs")).expect("write bindings.rs");
}
