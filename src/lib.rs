#![allow(non_snake_case)]
#![allow(clippy::borrow_interior_mutable_const)]

#![feature(abi_thiscall)]

use std::{sync::mpsc, thread};

#[macro_use] extern crate log;
extern crate simplelog;

use once_cell::sync::OnceCell;

mod input; // Console input
mod sys;   // Configs
mod detours;
mod logging;

static SENDER: OnceCell< mpsc::SyncSender<()> > = OnceCell::new();
const DLL_PROCESS_ATTACH: u32 = 1;
const DLL_PROCESS_DETACH: u32 = 0;

extern "system" {
	fn AllocConsole() -> bool;
	fn FreeConsole() -> bool;
	fn GetLastError() -> u32;
}

fn init() {
	if let Err(why) = logging::init() {
		eprintln!("Couldn't start logging module. [{}]", why);
		return;
	}

	unsafe {
		if !AllocConsole() {
			error!("Couldn't allocate console. [{}]", GetLastError());
		}
	}

	if let Err(why) = unsafe { detours::init() } {
		error!("Fatal error when setting up detours. {}", why);
		return;
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

	SENDER.set(sender).expect("Couldn't set mpsc kill channel!");
}

fn cleanup() {
	// Detour cleanups
	if let Err(why) = unsafe { detours::cleanup() } {
		error!("Failed to cleanup all detours. {}", why);
	}

	unsafe {
		FreeConsole();
	};

	if let Some(sender) = SENDER.get() {
		sender.send(()).expect("Couldn't send mpsc kill message");
	}
}

// Windows Only. I'm not going to half-ass Linux support (And don't even get me to try and work with OSX..)
#[no_mangle]
pub extern "stdcall" fn DllMain(_: *const u8, reason: u32, _: *const u8) -> u32 {
	match reason {
		DLL_PROCESS_ATTACH => init(),
		DLL_PROCESS_DETACH => cleanup(),
		_ => ()
	}
	1
}

use rglua::types::LuaState;

#[no_mangle]
pub extern "C" fn gmod13_open(state: LuaState) -> i32 {
	use crate::sys::util::initMenuState;

	init();

	if let Err(why) = initMenuState(state) {
		error!("Couldn't initialize menu state!");
	}

	0
}

#[no_mangle]
pub extern "C" fn gmod13_close(_state: LuaState) -> i32 {
	cleanup();
	0
}