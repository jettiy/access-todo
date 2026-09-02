@echo off
chcp 65001 >nul
title Access

cd /d "%~dp0"

if not exist ".access-secrets.cmd" (
    echo [Error] .access-secrets.cmd not found.
    pause
    exit /b 1
)
call ".access-secrets.cmd"
set "TODO_API_PORT=7878"
set "TODO_BIND=0.0.0.0"

taskkill /f /im Access-API.exe >nul 2>&1

echo [1/3] API server...
start "" /b "Access-API.exe"
timeout /t 2 /nobreak >nul

echo [2/3] Post-it widget...
start "" "Access-Postit.exe"

echo [3/3] Agent watcher...
start "" /b powershell -WindowStyle Hidden -ExecutionPolicy Bypass -File "%~dp0Access-Watch.ps1"

echo Done! Post-it windows appearing.
pause
