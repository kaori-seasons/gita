cmake_minimum_required(VERSION 3.16)
project(AlgorithmPlugins VERSION 1.0.0 LANGUAGES CXX)

# 设置C++标准
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 设置编译选项
if(MSVC)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /W4")
    set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} /Od /Zi")
    set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} /O2")
else()
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -Wpedantic")
    set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -g -O0")
    set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} -O3 -DNDEBUG")
endif()

# 设置输出目录
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)

# 包含目录
include_directories(${CMAKE_SOURCE_DIR}/include)
include_directories(${CMAKE_SOURCE_DIR}/third_party/include)

# 查找依赖库
find_package(Threads REQUIRED)

# 查找FFTW库（用于频谱分析）
find_package(PkgConfig QUIET)
if(PkgConfig_FOUND)
    pkg_check_modules(FFTW fftw3)
    if(FFTW_FOUND)
        include_directories(${FFTW_INCLUDE_DIRS})
        link_directories(${FFTW_LIBRARY_DIRS})
        set(FFTW_LIBRARIES ${FFTW_LIBRARIES})
    endif()
endif()

# 如果没有找到FFTW，使用内置实现
if(NOT FFTW_FOUND)
    message(STATUS "FFTW not found, using built-in FFT implementation")
    add_definitions(-DUSE_BUILTIN_FFT)
endif()

# 查找JSON库
find_package(nlohmann_json QUIET)
if(NOT nlohmann_json_FOUND)
    # 如果没有找到，使用内置的简单JSON实现
    add_definitions(-DUSE_BUILTIN_JSON)
endif()

# 源文件
set(PLUGIN_SOURCES
    src/plugin_base.cpp
    src/data_types.cpp
    src/feature_plugin_base.cpp
    src/vibrate31_plugin.cpp
    src/current_feature_plugin.cpp
    src/temperature_feature_plugin.cpp
    src/audio_feature_plugin.cpp
    src/decision_plugin_base.cpp
    src/motor97_plugin.cpp
    src/universal_classify1_plugin.cpp
    src/evaluation_plugin_base.cpp
    src/comp_realtime_health34_plugin.cpp
    src/error18_plugin.cpp
    src/event_plugin_base.cpp
    src/score_alarm5_plugin.cpp
    src/status_alarm4_plugin.cpp
    src/plugin_manager.cpp
    src/plugin_chain_manager.cpp
    src/plugin_config_manager.cpp
    src/plugin_monitor_manager.cpp
)

# 头文件
set(PLUGIN_HEADERS
    include/plugin_base.h
    include/data_types.h
    include/feature_plugin_base.h
    include/vibrate31_plugin.h
    include/current_feature_plugin.h
    include/temperature_feature_plugin.h
    include/audio_feature_plugin.h
    include/decision_plugin_base.h
    include/motor97_plugin.h
    include/universal_classify1_plugin.h
    include/evaluation_plugin_base.h
    include/comp_realtime_health34_plugin.h
    include/error18_plugin.h
    include/event_plugin_base.h
    include/score_alarm5_plugin.h
    include/status_alarm4_plugin.h
    include/plugin_manager.h
    include/plugin_chain_manager.h
    include/plugin_config_manager.h
    include/plugin_monitor_manager.h
)

# 创建核心库
add_library(AlgorithmPluginsCore STATIC ${PLUGIN_SOURCES} ${PLUGIN_HEADERS})
target_link_libraries(AlgorithmPluginsCore 
    Threads::Threads
    ${FFTW_LIBRARIES}
)

if(nlohmann_json_FOUND)
    target_link_libraries(AlgorithmPluginsCore nlohmann_json::nlohmann_json)
endif()

# 设置编译定义
target_compile_definitions(AlgorithmPluginsCore PRIVATE
    ALGORITHM_PLUGINS_VERSION="${PROJECT_VERSION}"
    ALGORITHM_PLUGINS_BUILD_DATE="${CMAKE_CURRENT_DATE}"
)

# 创建动态库
add_library(AlgorithmPlugins SHARED ${PLUGIN_SOURCES} ${PLUGIN_HEADERS})
target_link_libraries(AlgorithmPlugins 
    Threads::Threads
    ${FFTW_LIBRARIES}
)

if(nlohmann_json_FOUND)
    target_link_libraries(AlgorithmPlugins nlohmann_json::nlohmann_json)
endif()

# 设置动态库属性
set_target_properties(AlgorithmPlugins PROPERTIES
    VERSION ${PROJECT_VERSION}
    SOVERSION ${PROJECT_VERSION_MAJOR}
    PUBLIC_HEADER "${PLUGIN_HEADERS}"
)

# 创建示例程序
add_executable(plugin_example examples/plugin_example.cpp)
target_link_libraries(plugin_example AlgorithmPluginsCore)

# 创建测试程序
add_executable(plugin_tests tests/plugin_tests.cpp)
target_link_libraries(plugin_tests AlgorithmPluginsCore)

# 创建插件加载器示例
add_executable(plugin_loader examples/plugin_loader.cpp)
target_link_libraries(plugin_loader AlgorithmPluginsCore)

# 安装规则
install(TARGETS AlgorithmPluginsCore AlgorithmPlugins
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    RUNTIME DESTINATION bin
    PUBLIC_HEADER DESTINATION include/AlgorithmPlugins
)

install(DIRECTORY include/ DESTINATION include/AlgorithmPlugins
    FILES_MATCHING PATTERN "*.h"
)

# 创建插件包
include(CPack)
set(CPACK_PACKAGE_NAME "AlgorithmPlugins")
set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Algorithm Plugins Framework")
set(CPACK_PACKAGE_VENDOR "Algorithm Team")
set(CPACK_PACKAGE_CONTACT "algorithm@company.com")

# Windows特定设置
if(WIN32)
    set(CPACK_GENERATOR "ZIP")
    set(CPACK_PACKAGE_FILE_NAME "${CPACK_PACKAGE_NAME}-${CPACK_PACKAGE_VERSION}-win64")
else()
    set(CPACK_GENERATOR "TGZ")
    set(CPACK_PACKAGE_FILE_NAME "${CPACK_PACKAGE_NAME}-${CPACK_PACKAGE_VERSION}-linux")
endif()

# 创建插件配置文件
configure_file(
    ${CMAKE_SOURCE_DIR}/config/plugin_config.json.in
    ${CMAKE_BINARY_DIR}/plugin_config.json
    @ONLY
)

# 创建插件目录
file(MAKE_DIRECTORY ${CMAKE_BINARY_DIR}/plugins)
file(MAKE_DIRECTORY ${CMAKE_BINARY_DIR}/config)
file(MAKE_DIRECTORY ${CMAKE_BINARY_DIR}/logs)

# 打印构建信息
message(STATUS "Algorithm Plugins Framework")
message(STATUS "Version: ${PROJECT_VERSION}")
message(STATUS "Build type: ${CMAKE_BUILD_TYPE}")
message(STATUS "C++ standard: ${CMAKE_CXX_STANDARD}")
message(STATUS "Install prefix: ${CMAKE_INSTALL_PREFIX}")

if(FFTW_FOUND)
    message(STATUS "FFTW found: ${FFTW_LIBRARIES}")
else()
    message(STATUS "Using built-in FFT implementation")
endif()

if(nlohmann_json_FOUND)
    message(STATUS "nlohmann_json found")
else()
    message(STATUS "Using built-in JSON implementation")
endif()
