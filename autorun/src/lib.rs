// Rust-Analyzer has false positives with detours-rs
// https://github.com/rust-analyzer/rust-analyzer/issues/9576
#![allow(unused_unsafe)]
use rglua::prelude::*;

mod configs;
mod fs;

#[macro_use]
mod logging;

mod cross;
mod hooks;
mod lua;

#[cfg(plugins)]
mod plugins;
mod ui;

#[cfg(http)]
mod version;

use logging::error;

#[no_mangle]
#[cfg(inject)]
extern "system" fn DllMain(_: *const u8, reason: u32, _: *const u8) -> u32 {
	use winapi::um::winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};

	match reason {
		DLL_PROCESS_ATTACH => {
			if let Err(why) = cross::startup() {
				error!("Failed to start: {why}");
			}
		}
		DLL_PROCESS_DETACH => {
			if let Err(why) = cross::cleanup() {
				error!("Failed to cleanup: {why}");
			}
		}
		_ => (),
	}

	1
}

#[gmod_open]
#[allow(unused)]
pub fn main(l: LuaState) -> i32 {
	// DllMain is called prior to this even if Autorun is used as a binary module.
	// So only initialize what we haven't already.
	#[cfg(not(feature = "inject"))]
	if let Err(why) = cross::startup() {
		printgm!(l, "Failed to start Autorun: `{why}`");
		error!("Failed to start Autorun: `{why}`");
	}

	0
}

#[gmod_close]
pub fn close(_l: LuaState) -> i32 {
	#[cfg(not(feature = "inject"))]
	if let Err(why) = cross::cleanup() {
		error!("Failed to cleanup at gmod13_close: {why}");
	}
	0
}
