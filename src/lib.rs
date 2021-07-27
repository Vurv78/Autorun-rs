#![allow(non_snake_case)]
#![feature(abi_thiscall)]

use std::{
	thread,
	sync::mpsc
};

use detour::static_detour;
use rglua::interface::*;

#[macro_use] extern crate log;
extern crate simplelog;

use once_cell::sync::{Lazy, OnceCell};

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

// You need rust nightly to use thiscall because ("fuck you" - 4 year old feature that hasn't been stabilized https://github.com/rust-lang/rust/issues/422026)
type PaintTraverseFn = unsafe extern "thiscall" fn(&'static IPanel, usize, bool, bool);

// Might as well use static detours if we're gonna be on nightly now I guess.
static_detour! {
	static PaintTraverseHook: unsafe extern "thiscall" fn(&'static IPanel, usize, bool, bool);
}

fn painttraverse_detour(this: &'static IPanel, panel_id: usize, force_repaint: bool, allow_force: bool) {
	debug!("test");
	unsafe {
		PaintTraverseHook.call(this, panel_id, force_repaint, allow_force);
	}
}

fn init() {
	if let Err(why) = logging::init() {
		eprintln!("Couldn't start logging module. [{}]", why);
		return;
	}

	unsafe {
		assert_eq!(
			AllocConsole(), 1,
			"Couldn't allocate console. Error id: [{}]", GetLastError()
		);

		let iface = get_interface_handle("engine.dll").unwrap();
		let iface = get_from_interface("VEngineClient015", iface)
			.unwrap() as *mut EngineClient;
		let enginecl = iface.as_ref().unwrap();

		let iface = get_interface_handle("vgui2").unwrap();
		let iface = get_from_interface("VGUI_Panel009", iface)
			.unwrap() as *mut IPanel;

		let vgui = iface.as_ref().unwrap();

		let painttraverse_obj: PaintTraverseFn = std::mem::transmute(
			(vgui.vtable as *const *const i8).offset(41)
		);

		PaintTraverseHook
			.initialize(painttraverse_obj, painttraverse_detour)
			.unwrap()
			.enable()
			.unwrap();

		//println!("Time. {}", enginecl.Time());
	}

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