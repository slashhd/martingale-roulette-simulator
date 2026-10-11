@echo off
setlocal
title Martingale Roulette Simulator
cd /d "%~dp0"
echo Martingale Roulette Simulator
echo Open http://127.0.0.1:3000 after the server reports it is ready.
echo Press Ctrl+C to stop.
echo.

if exist "martingale_sim.exe" (
    "martingale_sim.exe"
) else (
    where cargo >nul 2>nul
    if errorlevel 1 (
        echo Rust is required for a source download. Install it from https://rustup.rs
        echo Or download a prebuilt executable from the GitHub Releases page.
        pause
        exit /b 1
    )
    cargo run --release --locked
)
if errorlevel 1 (
    echo.
    echo The simulator failed to start. Check the error above.
    pause
    exit /b 1
)
