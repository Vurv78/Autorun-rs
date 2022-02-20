// Rust-Analyzer has false positives with detours-rs
// https://github.com/rust-analyzer/rust-analyzer/issues/9576
#![allow(unused_unsafe)]
use rglua::prelude::*;

mod configs;
#[macro_use]
mod logging;
mod ui;
mod cross;
mod global;
mod hooks;
mod lua;
mod util;

use logging::*;

#[no_mangle]
#[cfg(feature = "inject")]
extern "system" fn DllMain(_: *const u8, reason: u32, _: *const u8) -> u32 {
	use winapi::um::winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};

	match reason {
		DLL_PROCESS_ATTACH => {
			if let Err(why) = cross::startup() {
				error!("Failed to start: {}", why);
			}
		}
		DLL_PROCESS_DETACH => {
			if let Err(why) = cross::cleanup() {
				error!("Failed to cleanup: {}", why);
			}
		}
		_ => (),
	}

	1
}

#[gmod_open]
pub fn main(l: LuaState) -> i32 {
	// DllMain is called prior to this even if Autorun is used as a binary module.
	// So only initialize what we haven't already.
	#[cfg(not(feature = "inject"))]
	if let Err(why) = cross::startup() {
		printgm!(l, "Failed to start Autorun: `{}`", why);
		error!("Failed to start Autorun: `{}`", why)
	}

	0
}

#[gmod_close]
pub fn close(_l: LuaState) -> i32 {
	if let Err(why) = cross::cleanup() {
		error!("Failed to cleanup at gmod13_close: {}", why);
	}
	0
}
