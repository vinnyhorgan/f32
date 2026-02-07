@echo off
SETLOCAL

:: 1. Clear conflicts
SET ANTHROPIC_API_KEY=
SET ANTHROPIC_AUTH_TOKEN=
SET ANTHROPIC_BASE_URL=

:: 2. Check if already running (to prevent port 4000 errors)
netstat -ano | findstr :4000 > nul && (
    echo [INFO] proxy already running on 4000.
    exit /b
)

echo [PROXY] starting universal gateway...
echo [INFO] press ctrl+c to stop the proxy.
echo ---------------------------------------

:: 3. Run directly in this window (no 'start' command)
litellm --config litellm.yaml --port 4000
