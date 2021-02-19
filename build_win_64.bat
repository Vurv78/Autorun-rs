@echo off
cargo build --release
move %cd%\target\release\Autorun.dll %cd%\Autorun_Win_64.dll
pause