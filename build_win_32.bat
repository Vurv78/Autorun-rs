@echo off
rustup target add i686-pc-windows-msvc
cargo build --release --target=i686-pc-windows-msvc
move %cd%\target\i686-pc-windows-msvc\release\Autorun.dll %cd%\gmsv_autorun_win32.dll
pause