@echo off
chcp 65001 >nul
title Access - 오늘의 할 일

cd /d "%~dp0"

REM 민감 정보(GITHUB_TOKEN, TODO_GIST_ID)는 .access-secrets.cmd에서 로드
REM 이 파일은 ACL로 보호되며 버전 관리에서 제외됨
if not exist "%~dp0.access-secrets.cmd" (
    echo [오류] .access-secrets.cmd 파일이 없습니다.
    echo GitHub Token과 Gist ID를 포함한 .access-secrets.cmd를 생성하세요.
    pause
    exit /b 1
)
call "%~dp0.access-secrets.cmd"
set "TODO_API_PORT=7878"
REM 이 PC는 WSL 불용 — localhost 전용 (외부 노출 금지)
set "TODO_BIND=127.0.0.1"

echo ========================================
echo   Access - 오늘의 할 일 시작
echo ========================================
echo.

REM 이미 실행 중이면 API 서버만 재시용
taskkill /f /im Access-API.exe >nul 2>&1

echo [1/3] API 서버 시작 중... (콘솔 비종속 + 로그 기록)
powershell -NoProfile -Command "Start-Process -FilePath '%~dp0Access-API.exe' -WorkingDirectory '%~dp0' -WindowStyle Hidden -RedirectStandardOutput '%~dp0api.log' -RedirectStandardError '%~dp0api.err.log'"
timeout /t 2 /nobreak >nul

echo [2/3] 포스트잇 4개 창 시작 중...
start "" "Access-포스트잇.exe"

echo [3/3] 에이전트 감시 시작 (모든 에이전트 종료 시 자동 정리)...
start "" /b powershell -WindowStyle Hidden -ExecutionPolicy Bypass -File "%~dp0Access-감시.ps1"

echo.
echo 완료! 4개 포스트잇이 바탕화면에 나타납니다.
echo.
REM 자동시작(시작프로그램)일 때는 콘솔을 바로 닫는다
if /i "%~1"=="auto" exit /b 0
pause
