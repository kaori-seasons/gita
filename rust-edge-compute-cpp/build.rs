//! 构建脚本
//!
//! 配置C++ FFI桥接的构建

fn main() {
    // 配置CXX桥接
    cxx_build::bridge("src/ffi.rs")
        .file("src/ffi/cpp_bridge.cc")
        .file("src/ffi/json_parser.cc")
        .include("src/ffi")
        .flag_if_supported("-std=c++17")
        .compile("rust-edge-compute-cpp-ffi");
    
    // 重新编译触发
    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=src/ffi/cpp_bridge.h");
    println!("cargo:rerun-if-changed=src/ffi/cpp_bridge.cc");
    println!("cargo:rerun-if-changed=src/ffi/json_parser.h");
    println!("cargo:rerun-if-changed=src/ffi/json_parser.cc");
}

