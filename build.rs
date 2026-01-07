// build.rs - 集成cpp_plugins架构的CXX编译配置

use std::env;
use std::path::PathBuf;

fn main() {
    // 告诉cargo在重新构建时重新运行
    println!("cargo:rerun-if-changed=src/ffi/bridge.rs");
    println!("cargo:rerun-if-changed=src/ffi/cpp/bridge.h");
    println!("cargo:rerun-if-changed=src/ffi/cpp/bridge.cc");
    println!("cargo:rerun-if-changed=src/capnp/algorithm.capnp");
    
    // 配置编译器标志
    println!("cargo:rustc-cfg=use_cxx");
    
    // 设置C++17标准
    println!("cargo:rustc-cfg=cxx17");
    
    // 编译C++ FFI桥接代码
    build_cpp_bridge();
    
    // 链接C++插件库
    link_cpp_plugins();
    
    // Cap'n Proto编译（如果启用capnproto特性）
    #[cfg(feature = "capnproto")]
    {
        compile_capnp();
    }
    
    // 仅在启用cxx特性时编译C++代码
    #[cfg(feature = "cxx")]
    {
        // 启用FFTW支持（如果可用）
        println!("cargo:rustc-cfg=enable_fftw");
    }
}

/// 编译C++ FFI桥接代码
fn build_cpp_bridge() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cpp_plugins_dir = PathBuf::from(&manifest_dir).join("cpp_plugins");
    let include_dir = cpp_plugins_dir.join("include");
    
    // 复制bridge.h到cxxbridge crate目录
    let cxx_include_dir = out_dir.join("cxxbridge").join("crate").join("ffi").join("cpp");
    std::fs::create_dir_all(&cxx_include_dir).unwrap();
    let src_bridge_h = PathBuf::from(&manifest_dir).join("src/ffi/cpp/bridge.h");
    let dst_bridge_h = cxx_include_dir.join("bridge.h");
    std::fs::copy(&src_bridge_h, &dst_bridge_h).unwrap();
    
    // 使用CXX编译bridge.rs
    let mut build = cxx_build::bridge("src/ffi/bridge.rs");
    build
        .file("src/ffi/cpp/bridge.cc")  // 使用生产增强的实现
        .include("src/ffi/cpp")
        .include(&include_dir)
        .include(&cpp_plugins_dir)  // 添加cpp_plugins目录
        .flag_if_supported("-std=c++17")
        .flag_if_supported("-Wall")
        .flag_if_supported("-Wno-unused-parameter")
        .cpp(true);
    
    // 在macOS上添加额外配置
    if cfg!(target_os = "macos") {
        build.flag_if_supported("-mmacosx-version-min=10.15");
    }
    
    build.compile("gita_bridge");
    
    println!("cargo:warning=Compiled C++ bridge code");
}

/// 链接C++插件库
fn link_cpp_plugins() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let cpp_plugins_dir = PathBuf::from(&manifest_dir).join("cpp_plugins");
    let install_dir = cpp_plugins_dir.join("install");
    let lib_dir = install_dir.join("lib");
    
    // 检查库是否存在
    if lib_dir.exists() {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=dylib=AlgorithmPlugins");
        println!("cargo:rustc-link-lib=static=AlgorithmPluginsCore");
        
        // 在macOS上设置rpath
        if cfg!(target_os = "macos") {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
        }
        // 在Linux上设置rpath
        else if cfg!(target_os = "linux") {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
        }
        
        println!("cargo:warning=Linked C++ plugin library from: {}", lib_dir.display());
    } else {
        println!("cargo:warning=C++ plugin library not found at: {}. Please run 'cd cpp_plugins && ./build.sh' first.", lib_dir.display());
    }
}

#[cfg(feature = "capnproto")]
fn compile_capnp() {
    use std::env;
    use std::path::PathBuf;
    
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let src_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    
    // 编译Cap'n Proto schema
    capnpc::CompilerCommand::new()
        .src_prefix(&src_dir.join("src/capnp"))
        .output_path(&out_dir)
        .file(src_dir.join("src/capnp/algorithm.capnp"))
        .run()
        .expect("Failed to compile Cap'n Proto schema");
    
    println!("cargo:warning=Compiled Cap'n Proto schema");
}