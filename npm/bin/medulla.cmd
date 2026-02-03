@echo off
setlocal

set "SCRIPT_DIR=%~dp0"

if exist "%SCRIPT_DIR%medulla.exe" (
    "%SCRIPT_DIR%medulla.exe" %*
) else (
    echo Error: medulla binary not found. Try reinstalling with 'npm install -g medulla' 1>&2
    exit /b 1
)
