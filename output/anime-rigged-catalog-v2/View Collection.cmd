@echo off
setlocal
set "MIYORA_PYTHON=%USERPROFILE%\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe"
if not exist "%MIYORA_PYTHON%" (
  echo Python runtime unavailable. Run serve_collection.py with Python 3.
  pause
  exit /b 1
)
"%MIYORA_PYTHON%" "%~dp0serve_collection.py" --open
