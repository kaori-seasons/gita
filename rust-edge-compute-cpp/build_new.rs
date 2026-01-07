//! Build script for Rust Edge Compute Framework - C++ FFI with Cap'n Proto
//!
//! Integrates:
//! 1. Cap'n Proto schema compilation for zero-copy serialization
//! 2. CXX bridge for Rust-C++ interoperability
//! 3. C++ source file compilation

use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let src_dir = PathBuf::from(".");

    println!("cargo:rerun-if-changed=src/capnp/algorithm.capnp");
    println!("cargo:rerun-if-changed=src/ffi/bridge.rs");
    println!("cargo:rerun-if-changed=src/ffi/cpp_bridge.h");
    println!("cargo:rerun-if-changed=src/ffi/cpp_bridge.cc");

    // ========================================================================
    // Phase 1: Compile Cap'n Proto Schemas
    // ========================================================================
    
    #[cfg(feature = "capnproto")]
    {
        println!("cargo:warning=Compiling Cap'n Proto schemas...");
        
        // Compile schema to Rust code
        capnpc::CompilerCommand::new()
            .src_prefix(&src_dir.join("src/capnp"))
            .output_path(&out_dir)
            .file(src_dir.join("src/capnp/algorithm.capnp"))
            .run()
            .expect("Failed to compile Cap'n Proto schema");

        // For C++, use capnp compiler (requires capnp binary)
        if has_capnp_compiler() {
            compile_capnp_cpp(&src_dir, &out_dir);
        } else {
            println!("cargo:warning=capnp compiler not found - C++ schema skipped");
            println!("cargo:warning=Install with: apt-get install capnproto");
        }

        // Add include path for generated C++ code
        println!(
            "cargo:rustc-link-search={}",
            out_dir.join("capnp").display()
        );
    }

    // ========================================================================
    // Phase 2: Configure CXX Bridge (if C++ compiler available)
    // ========================================================================
    
    #[cfg(feature = "cxx")]
    {
        if has_cpp_compiler() {
            println!("cargo:warning=Compiling CXX bridge...");
            
            cxx_build::bridge("src/ffi/bridge.rs")
                .file("src/ffi/cpp_bridge.cc")
                .include("src/ffi")
                .include(out_dir.join("capnp"))
                .flag_if_supported("-std=c++17")
                .flag_if_supported("-Wall")
                .flag_if_supported("-Wextra")
                .compile("rust-edge-compute-ffi");
        } else {
            println!("cargo:warning=C++ compiler not found - CXX bridge skipped");
            println!("cargo:warning=Install: gcc g++ build-essential (Ubuntu/Debian)");
        }
    }

    #[cfg(not(feature = "cxx"))]
    {
        println!("cargo:warning=CXX feature disabled - C++ bridge compilation skipped");
    }

    // ========================================================================
    // Phase 3: Configure Compiler Settings
    // ========================================================================
    
    println!("cargo:rustc-cfg=use_cxx");
    println!("cargo:rustc-cfg=cxx17");

    #[cfg(feature = "capnproto")]
    {
        println!("cargo:rustc-cfg=enable_capnproto");
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if capnp compiler is available on the system
fn has_capnp_compiler() -> bool {
    std::process::Command::new("capnp")
        .arg("--version")
        .output()
        .is_ok()
}

/// Check if C++ compiler (g++ or clang++) is available
fn has_cpp_compiler() -> bool {
    std::process::Command::new("c++")
        .arg("--version")
        .output()
        .is_ok()
        || std::process::Command::new("g++")
            .arg("--version")
            .output()
            .is_ok()
        || std::process::Command::new("clang++")
            .arg("--version")
            .output()
            .is_ok()
}

/// Compile Cap'n Proto schema to C++
#[cfg(feature = "capnproto")]
fn compile_capnp_cpp(src_dir: &std::path::PathBuf, out_dir: &std::path::PathBuf) {
    let schema_file = src_dir.join("src/capnp/algorithm.capnp");
    let output_dir = out_dir.join("capnp");

    std::fs::create_dir_all(&output_dir).ok();

    // Run capnp compiler to generate C++ code
    let status = std::process::Command::new("capnp")
        .arg("compile")
        .arg("-oc++")
        .arg(format!("--output-dir={}", output_dir.display()))
        .arg("-Isrc/capnp")
        .arg(&schema_file)
        .status()
        .expect("Failed to execute capnp compiler");

    if !status.success() {
        panic!("capnp schema compilation failed");
    }

    println!(
        "cargo:warning=Generated C++ code in {}",
        output_dir.display()
    );
}

// Re-export capnpc if capnproto feature is enabled
#[cfg(feature = "capnproto")]
extern crate capnpc;
