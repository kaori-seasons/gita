// build.rs - 集成cpp_plugins架构的CXX编译配置

fn main() {
    // 配置CXX桥接，包含cpp_plugins目录
    let mut build = cxx_build::bridge("src/ffi/bridge.rs");

    // 添加include路径
    build = build.include("src/ffi/cpp")
        .include("cpp_plugins/include");

    // 添加C++源文件
    build = build.file("src/ffi/cpp/bridge.cc")
        .file("cpp_plugins/src/plugin_base.cpp")
        .file("cpp_plugins/src/data_types.cpp")
        .file("cpp_plugins/src/feature_plugin_base.cpp")
        .file("cpp_plugins/src/vibrate31_plugin.cpp");

    // 编译cxx桥接
    build.compile("cxx-bridge");

    // 告诉cargo在重新构建时重新运行
    println!("cargo:rerun-if-changed=src/ffi/bridge.rs");
    println!("cargo:rerun-if-changed=src/ffi/cpp/bridge.h");
    println!("cargo:rerun-if-changed=src/ffi/cpp/bridge.cc");

    // 重新运行条件：cpp_plugins相关文件
    println!("cargo:rerun-if-changed=cpp_plugins/include/");
    println!("cargo:rerun-if-changed=cpp_plugins/src/");

    // 配置编译器标志
    println!("cargo:rustc-cfg=use_cxx");

    // 设置C++17标准
    println!("cargo:rustc-cfg=cxx17");

    // 启用FFTW支持（如果可用）
    #[cfg(feature = "fftw")]
    println!("cargo:rustc-cfg=enable_fftw");
}
