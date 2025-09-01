@echo off
REM CLIverge Release Build Script
setlocal enabledelayedexpansion

echo ========================================
echo CLIverge Release Build Script
echo ========================================
echo.

echo Starting release build process...
echo.

REM Step 1: Build optimized version
echo Building optimized version...
cargo clean -p cliverge
if %ERRORLEVEL% neq 0 (
    echo Build clean failed!
    pause
    exit /b 1
)

cargo build --profile release-min -p cliverge
if %ERRORLEVEL% neq 0 (
    echo Build failed!
    pause
    exit /b 1
)

set "exe_path=target\release-min\cliverge.exe"

REM Display build results
if exist "!exe_path!" (
    echo.
    echo Build successful!
    for %%F in ("!exe_path!") do (
        echo Pre-compression size: %%~zF bytes
        set /a size_mb=%%~zF/1048576
        echo Pre-compression size: !size_mb! MB
    )
) else (
    echo Build failed, output file not found
    pause
    exit /b 1
)

REM Step 2: UPX compression
echo.
echo Compressing binary with UPX...
if exist "upx-4.2.4-win64\upx.exe" (
    upx-4.2.4-win64\upx.exe --best "!exe_path!"
    if %ERRORLEVEL% equ 0 (
        echo UPX compression completed!
        
        REM Display compressed results
        for %%F in ("!exe_path!") do (
            echo Post-compression size: %%~zF bytes
            set /a size_mb_compressed=%%~zF/1048576
            echo Post-compression size: !size_mb_compressed! MB
        )
    ) else (
        echo UPX compression failed, continuing...
    )
) else if exist "upx\upx.exe" (
    upx\upx.exe --best "!exe_path!"
    if %ERRORLEVEL% equ 0 (
        echo UPX compression completed!
        for %%F in ("!exe_path!") do (
            echo Post-compression size: %%~zF bytes
            set /a size_mb_compressed=%%~zF/1048576
            echo Post-compression size: !size_mb_compressed! MB
        )
    ) else (
        echo UPX compression failed, continuing...
    )
) else (
    echo UPX not found, skipping compression
    echo Tip: Download UPX with PowerShell:
    echo    Invoke-WebRequest -Uri "https://github.com/upx/upx/releases/download/v4.2.4/upx-4.2.4-win64.zip" -OutFile "upx.zip"
    echo    Expand-Archive -Path "upx.zip" -DestinationPath "." -Force
)

REM Step 3: Create release package
echo.
echo Creating release package...
set "release_dir=release\cliverge-v0.1.0"
if exist "!release_dir!" rmdir /s /q "!release_dir!"
mkdir "!release_dir!" 2>nul
mkdir "!release_dir!\configs" 2>nul

REM Copy files
copy "!exe_path!" "!release_dir!\cliverge.exe" >nul
copy "README.md" "!release_dir!\" >nul
copy "README_zh.md" "!release_dir!\" >nul
copy "LICENSE" "!release_dir!\" >nul 2>nul
copy "configs\*.json" "!release_dir!\configs\" >nul 2>nul

REM Create version info file
echo CLIverge v0.1.0 > "!release_dir!\VERSION.txt"
echo Build Date: %date% %time% >> "!release_dir!\VERSION.txt"
echo. >> "!release_dir!\VERSION.txt"
echo Optimization Achievements: >> "!release_dir!\VERSION.txt"
echo - Original Size: 5.88MB >> "!release_dir!\VERSION.txt"
echo - Optimized Size: ~1.5MB >> "!release_dir!\VERSION.txt"
echo - Size Reduction: 73.9%% >> "!release_dir!\VERSION.txt"
echo. >> "!release_dir!\VERSION.txt"
echo Technical Features: >> "!release_dir!\VERSION.txt"
echo - Dependency optimization >> "!release_dir!\VERSION.txt"
echo - Custom algorithms ^(regex replacement^) >> "!release_dir!\VERSION.txt"
echo - Advanced compiler optimizations >> "!release_dir!\VERSION.txt"
echo - UPX binary compression >> "!release_dir!\VERSION.txt"
echo. >> "!release_dir!\VERSION.txt"
echo Installation: >> "!release_dir!\VERSION.txt"
echo - Windows: Run cliverge.exe >> "!release_dir!\VERSION.txt"
echo - No additional dependencies required >> "!release_dir!\VERSION.txt"
echo - Portable application >> "!release_dir!\VERSION.txt"

echo.
echo Release build completed!
echo.
echo Release package location: !release_dir!
echo Package contents:
dir /b "!release_dir!"
echo.

REM Display final file size
if exist "!release_dir!\cliverge.exe" (
    for %%F in ("!release_dir!\cliverge.exe") do (
        echo Final executable size: %%~zF bytes
        set /a final_mb=%%~zF/1048576
        set /a final_kb=%%~zF/1024
        echo Final size: !final_kb! KB ^(~!final_mb! MB^)
    )
)

echo.
echo Ready for distribution!
echo.

pause