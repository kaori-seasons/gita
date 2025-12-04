//! 构建脚本
//!
//! 配置C++ FFI桥接的构建
//! 支持在CI环境中编译C++ 模块

use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_default());
    
    // 检查是否在CI环境中
    let in_ci = env::var("CI").is_ok();
    
    // 设置构建信息
    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=src/ffi/cpp_bridge.h");
    println!("cargo:rerun-if-changed=src/ffi/cpp_bridge.cc");
    println!("cargo:rerun-if-changed=src/ffi/json_parser.h");
    println!("cargo:rerun-if-changed=src/ffi/json_parser.cc");
    
    // 检查C++编译器可用性
    let has_cpp_compiler = check_cpp_compiler();
    
    if has_cpp_compiler {
        println!("cargo:warning=C++ compiler found, compiling CXX bridge...");
        
        // 配置CXX桥接
        cxx_build::bridge("src/ffi.rs")
            .file("src/ffi/cpp_bridge.cc")
            .file("src/ffi/json_parser.cc")
            .include("src/ffi")
            .flag_if_supported("-std=c++17")
            .flag_if_supported("-Wall")
            .flag_if_supported("-Wextra")
            .flag_if_supported("-O2")
            .compile("rust-edge-compute-cpp-ffi");
        
        println!("cargo:rustc-cfg=cpp_enabled");
    } else {
        if in_ci {
            panic!("C++ compiler not found in CI environment. Please ensure build-essential, g++, gcc, and cmake are installed.");
        } else {
            println!("cargo:warning=C++ compiler not found, CXX bridge will not be compiled. Install: gcc g++ build-essential cmake");
        }
    }
}

/// 检查是否有可用的C++编译器
fn check_cpp_compiler() -> bool {
    // 尝试查找C++编译器
    for compiler in &["c++", "g++", "clang++"] {
        if std::process::Command::new(compiler)
            .arg("--version")
            .output()
            .is_ok()
        {
            return true;
        }
    }
    false
}

