// build.rs - 集成cpp_plugins架构的CXX编译配置

fn main() {
    // 暂时禁用CXX桥接以测试基本构建
    println!("cargo:warning=Skipping CXX bridge compilation for now");
    
    // 告诉cargo在重新构建时重新运行
    println!("cargo:rerun-if-changed=src/ffi/bridge.rs");
    
    // 配置编译器标志
    println!("cargo:rustc-cfg=use_cxx");
    
    // 设置C++17标准
    println!("cargo:rustc-cfg=cxx17");
    
    // 启用FFTW支持（如果可用）
    #[cfg(feature = "fftw")]
    println!("cargo:rustc-cfg=enable_fftw");
}