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

use winapi::{
	shared::minwindef::{DWORD, FALSE, TRUE},
	um::{
		consoleapi::{AllocConsole, SetConsoleCtrlHandler},
		errhandlingapi::GetLastError,
		wincon::{
			FreeConsole, GetConsoleWindow, CTRL_CLOSE_EVENT, CTRL_C_EVENT, CTRL_SHUTDOWN_EVENT,
		},
		winuser::{
			CreateMenu, DeleteMenu, GetSystemMenu, ShowWindow, MF_BYCOMMAND, SC_CLOSE, SW_HIDE,
			SW_SHOW,
		},
	},
};

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
		logging::init()?;
		detours::init()?;
	}

	indoc::printdoc! {"
		[Autorun-rs] v{version}
		Type 'help' for the list of commands
	", version = env!("CARGO_PKG_VERSION")};

	let (sender, receiver) = mpsc::sync_channel(1);

	// Disable close button and shortcuts
	// If you want to hide the console use the ``hide`` command.
	// Currently no way to halt Autorun w/o closing gmod
	unsafe {
		let window = GetConsoleWindow();
		let menu = GetSystemMenu(window, FALSE);

		DeleteMenu(menu, SC_CLOSE as u32, MF_BYCOMMAND);

		SetConsoleCtrlHandler(None, TRUE);
	}

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

// Returns if successfully initialized
fn attach() -> bool {
	unsafe {
		if GetConsoleWindow().is_null() {
			if AllocConsole() != TRUE {
				// Console didn't exist and couldn't allocate one now, hard error
				error!("Failed to allocate console. {}", GetLastError());
				return false;
			}
		} else {
			debug!("Found existing console!");
		}
	}

	if let Err(why) = init() {
		match why {
			InitializeError::LogInitError(y) => eprintln!("Failed to initialize logging. [{}]", y),
			_ => error!("Failed to initialize Autorun. [{}]", why),
		}
		false
	} else {
		true
	}
}

fn cleanup() {
	fn try_cleanup() -> Result<(), ExitError> {
		unsafe { detours::cleanup()? };

		if let Some(sender) = SENDER.get() {
			sender.send(()).map_err(|_| ExitError::MPSCFailure)?;
		}

		unsafe {
			FreeConsole();
		};

		Ok(())
	}

	if let Err(why) = try_cleanup() {
		error!("Failed to inject Autorun. [{}]", why);
	}
}

#[no_mangle]
extern "system" fn DllMain(_: *const u8, reason: u32, _: *const u8) -> u32 {
	use winapi::um::winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};

	match reason {
		DLL_PROCESS_ATTACH => {
			attach();
		}
		DLL_PROCESS_DETACH => {
			cleanup();
		}
		_ => (),
	}

	1
}

use rglua::types::LuaState;

#[no_mangle]
extern "C" fn gmod13_open(state: LuaState) -> i32 {
	// DllMain is called prior to this even if Autorun is used as a binary module.
	// So only initialize what we haven't already.

	use crate::sys::util::initMenuState;

	if let Err(why) = initMenuState(state) {
		error!("Failed to initialize menu state. [{}]", why);
	}
	0
}

#[no_mangle]
extern "C" fn gmod13_close(_state: LuaState) -> i32 {
	cleanup();
	0
}
