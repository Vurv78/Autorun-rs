#![allow(non_snake_case)]
use std::{
	thread,
	sync::mpsc
};

#[macro_use] extern crate log;
extern crate simplelog;

use once_cell::sync::OnceCell;

mod input; // Console input
mod sys;   // Configs
mod hooks; // Hook functions for detours
mod logging; // You guessed it, logging

use sys::statics::*;

const SENDER: OnceCell< mpsc::Sender<()> > = OnceCell::new();
const DLL_PROCESS_ATTACH: u32 = 1;
const DLL_PROCESS_DETACH: u32 = 0;

extern "system" {
	fn AllocConsole() -> i32;
	fn FreeConsole() -> i32;
	fn GetLastError() -> u32;
}

fn init() {
	if let Err(why) = logging::init() {
		eprintln!("Couldn't start logging module. [{}]", why);
		return;
	}

	unsafe {
		if AllocConsole() == 0 {
			error!("Couldn't allocate console. {}", GetLastError());
		}
	}
	//assert_eq!( unsafe { AllocConsole() }, 1, "Couldn't allocate console" );

	debug!("Initialized.");
	println!("<---> Autorun-rs <--->");
	println!("Type [help] for the list of commands");

	unsafe {
		LUAL_LOADBUFFERX.enable().expect("Couldn't enable luaL_loadbufferx hook");
		LUAL_NEWSTATE.enable().expect("Couldn't enable luaL_newstate hook");
	}

	let (sender, receiver) = mpsc::channel();

	thread::spawn(move || loop {
		if let Ok(_) = input::try_process_input() {
			// Got a command
			continue;
		}
		match receiver.try_recv() {
			Ok(_) => {
				break;
			},
			Err( mpsc::TryRecvError::Disconnected ) => {
				break;
			},
			Err( mpsc::TryRecvError::Empty ) => ()
		}
	});

	SENDER.set(sender).expect("Couldn't set mpsc kill channel!");
}

fn cleanup() {
	// Detour cleanups
	unsafe {
		LUAL_LOADBUFFERX.disable().unwrap();
		if let Some(hook) = JOIN_SERVER.get() {
			hook.disable().unwrap();
		}
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
	init();
	CURRENT_LUA_STATE.store(state, atomic::Ordering::SeqCst);

	0
}

#[no_mangle]
pub extern "C" fn gmod13_close(_state: LuaState) -> i32 {
	cleanup();
	0
}