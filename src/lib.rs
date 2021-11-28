#![allow(non_snake_case)]

use std::{sync::mpsc, thread};
use thiserror::Error;

#[cfg(feature = "logging")]
#[macro_use]
extern crate log;

#[cfg(feature = "logging")]
extern crate simplelog;

#[macro_use]
mod logging;

use once_cell::sync::OnceCell;

mod detours;
mod input; // Console input
mod sys; // Configs

static SENDER: OnceCell<mpsc::SyncSender<()>> = OnceCell::new();
const DLL_PROCESS_ATTACH: u32 = 1;
const DLL_PROCESS_DETACH: u32 = 0;

extern "system" {
	fn AllocConsole() -> bool;
	fn FreeConsole() -> bool;
	fn GetLastError() -> u32;
}

#[derive(Error, Debug)]
enum InitializeError {
	#[error("Failed to create thread channel")]
	MPSCFailure,
	#[error("{0}")]
	LogInitError(#[from] logging::LogInitError),
	#[error("Failed to initialize detours")]
	DetoursInitError(#[from] detour::Error),
}

#[derive(Error, Debug)]
enum ExitError {
	#[error("Failed to send exit signal to thread")]
	MPSCFailure,
	#[error("Failed to initialize detours")]
	DetoursInitError(#[from] detour::Error),
}

fn init() -> Result<(), InitializeError> {
	unsafe {
		println!("Init!");
		logging::init()?;
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
			Err(Empty) => (),
		}
	});

	// Literally impossible for it to be full. But whatever
	SENDER
		.set(sender)
		.map_err(|_| InitializeError::MPSCFailure)?;

	Ok(())
}

fn cleanup() -> Result<(), ExitError> {
	unsafe { detours::cleanup()? };

	if let Some(sender) = SENDER.get() {
		sender.send(()).map_err(|_| ExitError::MPSCFailure)?;
	}

	unsafe {
		FreeConsole();
	};

	Ok(())
}

// Windows Only. I'm not going to half-ass Linux support (And don't even get me to try and work with OSX..)
#[no_mangle]
extern "system" fn DllMain(_: *const u8, reason: u32, _: *const u8) -> u32 {
	match reason {
		DLL_PROCESS_ATTACH => {
			unsafe {
				if !AllocConsole() {
					// Assume a console already exists and just log an error.
					eprintln!("Failed to allocate console. {}", GetLastError());
				}
			}

			if let Err(why) = init() {
				error!("Failed to inject Autorun. [{}]", why);
			}
		}
		DLL_PROCESS_DETACH => {
			if let Err(why) = cleanup() {
				error!("Failed to inject Autorun. [{}]", why);
			}
		}
		_ => (),
	}
	1
}

use rglua::types::LuaState;

#[no_mangle]
extern "C" fn gmod13_open(state: LuaState) -> i32 {
	use crate::sys::util::initMenuState;
	unsafe {
		if !AllocConsole() {
			// Assume a console already exists and just log an error.
			eprintln!("Failed to allocate console. {}", GetLastError());
		}
	}

	if let Err(why) = init() {
		match why {
			InitializeError::LogInitError(y) => eprintln!("Failed to initialize logging. [{}]", y),
			_ => error!("Failed to initialize Autorun. [{}]", why),
		}
	} else if let Err(why) = initMenuState(state) {
		error!("Failed to initialize menu state. [{}]", why);
	}
	0
}

#[no_mangle]
extern "C" fn gmod13_close(_state: LuaState) -> i32 {
	if let Err(why) = cleanup() {
		error!("Failed to close Autorun module. [{}]", why);
	}
	0
}
