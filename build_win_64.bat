@echo off
cargo build
move %cd%\target\debug\Autorun.dll %cd%\gmsv_autorun_win64.dll
pause