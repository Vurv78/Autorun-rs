#![allow(non_snake_case)]

use std::{sync::mpsc, thread};

#[cfg(feature = "logging")]
#[macro_use] extern crate log;

#[cfg(feature = "logging")]
extern crate simplelog;

#[macro_use]
mod logging;

use once_cell::sync::OnceCell;

mod input; // Console input
mod sys;   // Configs
mod detours;

static SENDER: OnceCell< mpsc::SyncSender<()> > = OnceCell::new();
const DLL_PROCESS_ATTACH: u32 = 1;
const DLL_PROCESS_DETACH: u32 = 0;

extern "system" {
	fn AllocConsole() -> bool;
	fn FreeConsole() -> bool;
	fn GetLastError() -> u32;
}

fn init() -> anyhow::Result<()> {
	logging::init()?;

	unsafe {
		if !AllocConsole() {
			// Assume a console already exists and just log an error.
			error!("Failed to allocate console. {}", GetLastError());
		}

		detours::init()?;
	}

	debug!("Initialized.");
	println!("<---> Autorun-rs <--->");
	println!("Type [help] for the list of commands");

	let (sender, receiver) = mpsc::sync_channel(1);

	thread::spawn(move || loop {
		use mpsc::TryRecvError::*;
		if input::try_process_input().is_ok() {
			// Got a command
			continue;
		}
		match receiver.try_recv() {
			Ok(_) | Err(Disconnected) => break,
			Err(Empty) => ()
		}
	});

	if SENDER.set(sender).is_err() {
		anyhow::bail!("Failed to set sender.");
	}

	Ok(())
}

fn cleanup() -> anyhow::Result<()> {
	unsafe { detours::cleanup()? };

	if let Some(sender) = SENDER.get() {
		sender.send(())?;
	}

	unsafe {
		FreeConsole();
	};

	Ok(())
}

// Windows Only. I'm not going to half-ass Linux support (And don't even get me to try and work with OSX..)
#[no_mangle]
pub extern "system" fn DllMain(_: *const u8, reason: u32, _: *const u8) -> u32 {
	match reason {
		DLL_PROCESS_ATTACH => {
			if let Err(why) = init() {
				error!("Failed to inject Autorun. [{}]", why);
			}
		},
		DLL_PROCESS_DETACH => {
			if let Err(why) = cleanup() {
				error!("Failed to inject Autorun. [{}]", why);
			}
		},
		_ => ()
	}
	1
}

use rglua::types::LuaState;

#[no_mangle]
pub extern "C" fn gmod13_open(state: LuaState) -> i32 {
	use crate::sys::util::initMenuState;
	if let Err(why) = init() {
		error!("Failed to open Autorun module. [{}]", why);
		return 0;
	}

	if let Err(why) = initMenuState(state) {
		error!("Couldn't initialize menu state! [{}]", why);
	}
	0
}

#[no_mangle]
pub extern "C" fn gmod13_close(_state: LuaState) -> i32 {
	if let Err(why) = cleanup() {
		error!("Failed to close Autorun module. [{}]", why);
	}
	0
}