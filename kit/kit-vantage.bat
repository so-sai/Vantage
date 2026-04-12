@echo off
:: kit-vantage.bat - Vantage CLI Shim (v1.2.3)
:: Purpose: Provide a stable entry point for Agent structural commands.

set "BIN_PATH=%~dp0..\target\debug\vantage.exe"
if not exist "%BIN_PATH%" (
    set "BIN_PATH=%~dp0..\target\release\vantage.exe"
)

if not exist "%BIN_PATH%" (
    echo [vantage] Error: binary not found. Please run 'cargo build' in /cli.
    exit /b 1
)

"%BIN_PATH%" %*
