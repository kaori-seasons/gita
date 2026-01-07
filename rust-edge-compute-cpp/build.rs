use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap(); // gita directory

    let cpp_plugins_dir = workspace_root.join("cpp_plugins");
    let cpp_plugins_lib = cpp_plugins_dir.join("install/lib");

    println!("cargo:rerun-if-changed=src/ffi/algorithm_ffi.h");
    println!("cargo:rerun-if-changed=src/ffi/algorithm_ffi.cc");

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
    println!(
        "cargo:rustc-link-search=native={}",
        cpp_plugins_lib.display()
    );
    println!("cargo:rustc-link-lib=dylib=AlgorithmPlugins");

    // Add system libraries needed for C++
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=c++");
    } else {
        println!("cargo:rustc-link-lib=stdc++");
    }
}
