@echo off
REM Algorithm Plugins Framework Build Script for Windows
REM Supports MSVC (Visual Studio) compiler

setlocal enabledelayedexpansion

REM Color codes disabled (use plain text for batch compatibility)
REM Standard output format with brackets for status indicators

REM Check dependencies
echo.
echo Checking build dependencies...

where cmake >nul 2>nul
if errorlevel 1 (
    echo [ERROR] CMake not installed. Please install CMake 3.16 or higher
    exit /b 1
)
echo [SUCCESS] CMake found

REM Enhanced C++ compiler detection
echo [INFO] Checking for C++ compilers...
set "COMPILER_FOUND=0"
set "COMPILER_TYPE="

REM Method 1: Check if MSVC cl.exe is in PATH
where cl >nul 2>nul
if errorlevel 0 (
    set "COMPILER_FOUND=1"
    set "COMPILER_TYPE=MSVC"
    echo [SUCCESS] Found MSVC compiler (cl.exe)
    goto compiler_check_done
)

REM Method 2: Search in common Visual Studio paths
echo [INFO] MSVC not in PATH. Searching Visual Studio installation paths...

REM Check VS 2022 x64
if exist "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC" (
    for /d %%A in ("C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\*") do (
        if exist "%%A\bin\Hostx64\x64\cl.exe" (
            set "COMPILER_FOUND=1"
            set "COMPILER_TYPE=MSVC"
            set "PATH=%%A\bin\Hostx64\x64;!PATH!"
            echo [SUCCESS] Found MSVC compiler at: %%A\bin\Hostx64\x64
            goto compiler_check_done
        )
    )
)

REM Check VS 2019 x64
if exist "C:\Program Files\Microsoft Visual Studio\2019\Community\VC\Tools\MSVC" (
    for /d %%A in ("C:\Program Files\Microsoft Visual Studio\2019\Community\VC\Tools\MSVC\*") do (
        if exist "%%A\bin\Hostx64\x64\cl.exe" (
            set "COMPILER_FOUND=1"
            set "COMPILER_TYPE=MSVC"
            set "PATH=%%A\bin\Hostx64\x64;!PATH!"
            echo [SUCCESS] Found MSVC compiler at: %%A\bin\Hostx64\x64
            goto compiler_check_done
        )
    )
)

REM Check VS 2022 x86
if exist "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC" (
    for /d %%A in ("C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\*") do (
        if exist "%%A\bin\Hostx64\x86\cl.exe" (
            set "COMPILER_FOUND=1"
            set "COMPILER_TYPE=MSVC"
            set "PATH=%%A\bin\Hostx64\x86;!PATH!"
            echo [SUCCESS] Found MSVC compiler at: %%A\bin\Hostx64\x86
            goto compiler_check_done
        )
    )
)

REM Method 3: Check for Clang++
where clang++ >nul 2>nul
if not errorlevel 1 (
    set "COMPILER_FOUND=1"
    set "COMPILER_TYPE=Clang++"
    echo [SUCCESS] Found Clang++ compiler
    goto compiler_check_done
)

REM Method 4: Check for G++ (MinGW)
where g++ >nul 2>nul
if not errorlevel 1 (
    set "COMPILER_FOUND=1"
    set "COMPILER_TYPE=G++"
    echo [SUCCESS] Found G++ compiler (MinGW)
    goto compiler_check_done
)

REM No compiler found
:compiler_check_error
echo [ERROR] No supported C++ compiler found!
echo.
echo --- Option 1: Visual Studio (Recommended) ---
echo Download: https://visualstudio.microsoft.com/
echo   - Community Edition (FREE)
echo   - Select 'Desktop development with C++' workload
echo   - After install, use Developer Command Prompt or restart CMD
echo.
echo --- Option 2: MinGW (Lightweight) ---
echo Download: https://www.mingw-w64.org/downloads/
echo   - Install to C:\mingw64
echo   - Add to PATH: set PATH=C:\mingw64\bin;%%PATH%%
echo.
echo --- Option 3: Clang/LLVM ---
echo Download: https://releases.llvm.org/download.html
echo   - Make sure to add to PATH during installation
echo.
echo --- Troubleshooting ---
echo If you have Visual Studio installed:
echo   1. Open 'Developer Command Prompt for VS 2022' or 'VS 2019'
echo   2. Navigate to this folder
echo   3. Run this script
echo.
exit /b 1

:compiler_check_done
echo [SUCCESS] Compiler check complete: %COMPILER_TYPE%
echo.

REM Parse command line arguments
set "BUILD_TYPE=Release"
set "CLEAN_ONLY=0"
set "TEST_ONLY=0"
set "INSTALL_ONLY=0"

:parse_args
if "%1"=="" goto args_done
if "%1"=="--help" goto show_help
if "%1"=="-h" goto show_help
if "%1"=="--clean" goto set_clean
if "%1"=="-c" goto set_clean
if "%1"=="--test" goto set_test
if "%1"=="-t" goto set_test
if "%1"=="--debug" goto set_debug
if "%1"=="--release" goto set_release
goto invalid_arg

:set_clean
set "CLEAN_ONLY=1"
shift
goto parse_args

:set_test
set "TEST_ONLY=1"
shift
goto parse_args

:set_debug
set "BUILD_TYPE=Debug"
shift
goto parse_args

:set_release
set "BUILD_TYPE=Release"
shift
goto parse_args

:invalid_arg
echo %RED%[ERROR] Unknown argument: %1%RESET%
goto show_help

:show_help
echo.
echo Algorithm Plugins Framework Build Script for Windows
echo.
echo Usage: %0 [options]
echo.
echo Options:
echo   -h, --help     Show this help message
echo   -c, --clean    Clean build directory
echo   -t, --test     Run tests only
echo   --debug        Use Debug build type
echo   --release      Use Release build type (default)
echo.
echo Examples:
echo   %0                    - Full build process
echo   %0 --clean           - Clean build directory
echo   %0 --test            - Run tests only
echo   %0 --debug           - Debug build
echo.
exit /b 0

:args_done

echo.
echo Algorithm Plugins Framework Build Start
echo Build Type: %BUILD_TYPE%
echo.

REM Clean mode
if "%CLEAN_ONLY%"=="1" (
    if exist build (
        echo Removing build directory...
        rmdir /s /q build
        echo [SUCCESS] Build directory cleaned
    )
    if exist install (
        echo Removing install directory...
        rmdir /s /q install
        echo [SUCCESS] Install directory cleaned
    )
    exit /b 0
)

REM Create build directory
if exist build (
    echo [WARNING] Build directory exists, cleaning...
    rmdir /s /q build
)

echo Creating build directory...
mkdir build
cd build

REM Configure CMake
echo.
echo Configuring CMake...

REM Detect generator based on detected compiler
if "%COMPILER_TYPE%"=="MSVC" (
    REM MSVC found, try Visual Studio generator first, then fall back to NMake if needed
    echo Using Visual Studio or NMake generator for MSVC
    REM Try VS 2022 first
    cmake .. -G "Visual Studio 17 2022" -A x64 -DCMAKE_INSTALL_PREFIX=../install -DCMAKE_BUILD_TYPE=%BUILD_TYPE% 2>nul
    if errorlevel 1 (
        REM Try VS 2019
        cmake .. -G "Visual Studio 16 2019" -A x64 -DCMAKE_INSTALL_PREFIX=../install -DCMAKE_BUILD_TYPE=%BUILD_TYPE% 2>nul
        if errorlevel 1 (
            REM Fall back to NMake Makefiles
            echo [INFO] Visual Studio generators not available, using NMake Makefiles
            cmake .. -G "NMake Makefiles" -DCMAKE_BUILD_TYPE=%BUILD_TYPE% -DCMAKE_INSTALL_PREFIX=../install
        )
    )
) else (
    REM Fall back to Unix Makefiles or Ninja for other compilers
    echo Using default generator (Unix Makefiles or Ninja)
    cmake .. -DCMAKE_BUILD_TYPE=%BUILD_TYPE% -DCMAKE_INSTALL_PREFIX=../install
)

if errorlevel 1 (
    echo [ERROR] CMake configuration failed
    cd ..
    exit /b 1
)
echo [SUCCESS] CMake configuration complete

REM Build project
echo.
echo Building project...

cmake --build . --config %BUILD_TYPE% --parallel

if errorlevel 1 (
    echo [ERROR] Build failed
    cd ..
    exit /b 1
)
echo [SUCCESS] Build complete

REM Run tests
if exist %BUILD_TYPE%\plugin_tests.exe (
    echo.
    echo Running tests...
    %BUILD_TYPE%\plugin_tests.exe
    if errorlevel 1 (
        echo [WARNING] Some tests failed
    ) else (
        echo [SUCCESS] All tests passed
    )
) else if exist plugin_tests.exe (
    echo.
    echo Running tests...
    plugin_tests.exe
    if errorlevel 1 (
        echo [WARNING] Some tests failed
    ) else (
        echo [SUCCESS] All tests passed
    )
) else (
    echo [WARNING] Test executable not found, skipping tests
)

REM Install project
echo.
echo Installing project...
cmake --install . --config %BUILD_TYPE%

if errorlevel 1 (
    echo [ERROR] Installation failed
    cd ..
    exit /b 1
)
echo [SUCCESS] Installation complete

REM Finish
cd ..
echo.
echo [SUCCESS] Build process completed!
echo Install directory: %CD%\install
echo Library directory: %CD%\install\lib
echo Header directory: %CD%\install\include
echo.

endlocal
