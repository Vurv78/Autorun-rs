@echo off
set file_name=autorun.dll

set target=x86_64-pc-windows-msvc
set target_dir=%cd%\target\%target%\debug
set out=%cd%\gmsv_autorun_win64.dll

rustup target add %target%
cargo build --target=%target%

move %target_dir%\%file_name% %out%
pause