@echo off
REM ============================================================
REM VANTAGE SHIM v1.2.3 - Cognitive Interceptor
REM ============================================================
REM This shim ensures Agents/IDEs interact with the sensor 
REM using the standardized StructuralSignal JSON contract.
REM ============================================================

set VANTAGE_BIN=%~dp0..\target\debug\vantage-verify.exe

if not exist "%VANTAGE_BIN%" (
    echo [VANTAGE] Error: Sensor binary not found. Run 'cargo build' first.
    exit /b 1
)

"%VANTAGE_BIN%" --json %*
