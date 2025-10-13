# CMake构建配置文件 - 矩阵乘法算法插件
# 生产级构建配置，支持多种优化选项

cmake_minimum_required(VERSION 3.16)
project(MatrixMultiplicationPlugin VERSION 1.0.0 LANGUAGES CXX)

# 设置C++标准
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# 构建类型配置
if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()

# 编译器优化选项 - 边缘端优化
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra")
# 边缘端：减少优化级别以节省编译时间和内存
set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -g -O1")
set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} -O2 -DNDEBUG")
# 移除march=native（边缘端CPU可能不一致）和flto（增加内存使用）

# OpenMP支持
find_package(OpenMP REQUIRED)
if(OpenMP_CXX_FOUND)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${OpenMP_CXX_FLAGS}")
endif()

# 查找依赖库
find_package(PkgConfig REQUIRED)

# JSON库 (nlohmann_json)
pkg_check_modules(JSONCPP REQUIRED jsoncpp)
include_directories(${JSONCPP_INCLUDE_DIRS})
link_directories(${JSONCPP_LIBRARY_DIRS})

# 边缘端优化：移除复杂依赖，专注于内存效率
# OpenBLAS和Eigen在边缘端资源受限环境下不适用
message(STATUS "边缘端优化模式：禁用OpenBLAS和Eigen依赖")
set(USE_OPENBLAS OFF)
set(USE_EIGEN OFF)

# Boost库 (用于命令行参数解析)
find_package(Boost 1.70 REQUIRED COMPONENTS program_options system filesystem)

# 包含目录
include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/src
    ${Boost_INCLUDE_DIRS}
)

# 源文件 - 边缘端优化版本
set(SOURCES
    src/main.cpp
    src/matrix_multiplication_edge_optimized.cpp
    src/json_handler.cpp
    src/performance_monitor.cpp
)

# 创建可执行文件
add_executable(matrix_multiplication ${SOURCES})

# 链接库 - 边缘端优化版本
target_link_libraries(matrix_multiplication
    PRIVATE
        ${JSONCPP_LIBRARIES}
        ${Boost_LIBRARIES}
        OpenMP::OpenMP_CXX
)

# 编译定义
target_compile_definitions(matrix_multiplication
    PRIVATE
        VERSION_MAJOR=${PROJECT_VERSION_MAJOR}
        VERSION_MINOR=${PROJECT_VERSION_MINOR}
        VERSION_PATCH=${PROJECT_VERSION_PATCH}
)

# 安装目标
install(TARGETS matrix_multiplication
    RUNTIME DESTINATION bin
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
)

# 安装头文件（如果需要）
install(DIRECTORY src/
    DESTINATION include/matrix_multiplication
    FILES_MATCHING PATTERN "*.hpp"
)

# 构建选项
option(BUILD_TESTS "Build unit tests" OFF)
if(BUILD_TESTS)
    enable_testing()
    add_subdirectory(tests)
endif()

option(BUILD_BENCHMARKS "Build performance benchmarks" OFF)
if(BUILD_BENCHMARKS)
    add_subdirectory(benchmarks)
endif()

# 生成版本头文件
configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/src/version.hpp.in
    ${CMAKE_CURRENT_BINARY_DIR}/version.hpp
)

# 显示构建信息
message(STATUS "Build type: ${CMAKE_BUILD_TYPE}")
message(STATUS "Use OpenBLAS: ${USE_OPENBLAS}")
message(STATUS "Use Eigen: ${USE_EIGEN}")
message(STATUS "C++ Compiler: ${CMAKE_CXX_COMPILER}")
message(STATUS "C++ Flags: ${CMAKE_CXX_FLAGS}")
message(STATUS "Link Libraries: ${JSONCPP_LIBRARIES} ${Boost_LIBRARIES}")

# 代码覆盖率（仅在Debug模式下）
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    option(CODE_COVERAGE "Enable code coverage" OFF)
    if(CODE_COVERAGE)
        set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} --coverage")
        set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} --coverage")
    endif()
endif()
