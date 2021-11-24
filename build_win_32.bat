@echo off
set file_name=autorun.dll

set target=i686-pc-windows-msvc
set target_dir=%cd%\target\%target%\release
set out=%cd%\gmsv_autorun_win32.dll

rustup target add %target%
cargo build --release --target=%target%

move %target_dir%\%file_name% %out%
pause