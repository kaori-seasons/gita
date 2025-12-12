use std::path::PathBuf;
use std::env;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap(); // gita directory
    
    let cpp_plugins_dir = workspace_root.join("cpp_plugins");
    let cpp_plugins_lib = cpp_plugins_dir.join("build/lib");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    println!("cargo:rerun-if-changed=src/ffi/algorithm_ffi.h");
    println!("cargo:rerun-if-changed=src/ffi/algorithm_ffi.cc");
    
    // Compile C++ FFI implementation - output object files instead of archive
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("src/ffi/algorithm_ffi.cc")
        .include("src/ffi")
        .flag("-std=c++17")
        .flag("-Wall")
        .flag("-Wextra")
        .flag_if_supported("-fPIC");
    
    // Get the object file path instead of compiling to archive
    let obj_file = out_path.join("algorithm_ffi.o");
    
    // Manually compile to object file
    let mut compiler = build.get_compiler();
    let args = vec![
        "-std=c++17".to_string(),
        "-Wall".to_string(),
        "-Wextra".to_string(),
        "-fPIC".to_string(),
        "-Isrc/ffi".to_string(),
        "-c".to_string(),
        "src/ffi/algorithm_ffi.cc".to_string(),
        format!("-o{}", obj_file.display()),
    ];
    
    let status = std::process::Command::new(compiler.path())
        .args(&args)
        .status()
        .expect("Failed to compile C++ FFI");
    
    if !status.success() {
        panic!("Failed to compile C++ FFI implementation");
    }
    
    // Link the object file directly without creating archive
    println!("cargo:rustc-link-search=native={}", out_path.display());
    // Don't use static library link, instead directly provide the object file path
    println!("cargo:rustc-link-arg={}", obj_file.display());
    
    // Generate Rust bindings from C header using bindgen
    let bindings = bindgen::Builder::default()
        .header("src/ffi/algorithm_ffi.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");
    
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("algorithm_ffi_bindings.rs"))
        .expect("Couldn't write bindings!");
    
    // Link cpp_plugins libraries
    println!("cargo:rustc-link-search=native={}", cpp_plugins_lib.display());
    println!("cargo:rustc-link-lib=dylib=AlgorithmPlugins");
    println!("cargo:rustc-link-lib=static=AlgorithmPluginsCore");
}
