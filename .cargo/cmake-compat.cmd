@echo off
if /I "%~1"=="--build" (
    "C:\Program Files\CMake\bin\cmake.exe" %*
) else (
    "C:\Program Files\CMake\bin\cmake.exe" -DCMAKE_POLICY_VERSION_MINIMUM=3.5 %*
)
