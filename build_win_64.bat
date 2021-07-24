@echo off
cargo build --release
move %cd%\target\release\Autorun.dll %cd%\gmsv_autorun_win64.dll
pause