use std::ffi::CStr;
use std::io::Write;
use std::{fs, sync::atomic::Ordering};

use crate::{configs, global, lua, util, logging};
use autorun_shared::{REALM_CLIENT, REALM_MENU};

use logging::*;
use rglua::interface::EngineClient;
use rglua::prelude::*;

#[macro_use]
pub mod lazy;

// Make our own static detours because detours.rs is lame and locked theirs behind nightly. :)
lazy_detour! {
	pub static LUAL_NEWSTATE_H: extern "C" fn() -> LuaState = (*LUA_SHARED_RAW.get::<extern "C" fn() -> LuaState>(b"luaL_newstate").unwrap(), newstate_h);
	pub static LUAL_LOADBUFFERX_H: extern "C" fn(LuaState, *const i8, SizeT, *const i8, *const i8) -> i32 = (*LUA_SHARED_RAW.get::<extern "C" fn(LuaState, LuaString, SizeT, LuaString, LuaString) -> i32>(b"luaL_loadbufferx").unwrap(), loadbufferx_h);
	pub static JOINSERVER_H: extern "C" fn(LuaState) -> i32;
}

#[cfg(feature = "runner")]
use rglua::interface::IPanel;

type PaintTraverseFn = extern "fastcall" fn(&'static IPanel, usize, bool, bool);

#[cfg(feature = "runner")]
lazy_detour! {
	static PAINT_TRAVERSE_H: PaintTraverseFn;
}

extern "C" fn newstate_h() -> LuaState {
	let state = unsafe { LUAL_NEWSTATE_H.call() };
	util::set_client(state);
	state
}

extern "C" fn loadbufferx_h(
	l: LuaState,
	code: LuaString,
	len: SizeT,
	identifier: LuaString,
	mode: LuaString,
) -> i32 {
	let engine: *mut rglua::interface::EngineClient = iface!("engine", "VEngineClient015")
		.expect("Couldn't get engine interface");
	let engine = unsafe { engine.as_ref() }.expect("Couldn't get engine as_ref");

	let do_run;
	if engine.IsConnected() {
		// CLIENT
		let server_ip = global::SERVER_IP.load(Ordering::Relaxed);

		let raw_path = unsafe { CStr::from_ptr(identifier) };
		let path = &raw_path.to_string_lossy()[1..]; // Remove the @ from the beginning of the path

		// There's way too many params here
		do_run = dispatch(l, path, server_ip, code, len, identifier);
		if !do_run {
			return 0
		}
	} else {
		// MENU
		if global::MENU_STATE.get().is_none() {
			if let Err(why) = crate::cross::startup_menu(l) {
				error!("Couldn't initialize menu state. {}", why);
			}

			// Should only be file dumping and hooking on clientside, so just take the menu state.
		}
	}

	unsafe {
		LUAL_LOADBUFFERX_H.call(l, code, len, identifier, mode)
	}
}

pub fn dispatch(l: LuaState, path: &str, server_ip: &str, mut code: LuaString, mut len: SizeT, identifier: LuaString) -> bool {
	let mut do_run = true;
	if global::HAS_AUTORAN
		.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
		.is_ok()
	{
		// This will only run once when HAS_AUTORAN is false, setting it to true.
		// Will be reset by JoinServer.
		let ar_path = configs::path(configs::AUTORUN_PATH);
		trace!("Running autorun script at {}", ar_path.display());
		if let Ok(script) = fs::read_to_string(&ar_path) {
			// Try to run here
			if let Err(why) = lua::run_with_env(&script, identifier, code, len, server_ip, true) {
				error!("{}", why);
			}
		} else {
			error!(
				"Couldn't read your autorun script file at [{}]",
				ar_path.display()
			);
		}
	}

	if let Ok(script) = fs::read_to_string(configs::path(configs::HOOK_PATH)) {
		match lua::run_with_env(&script, identifier, code, len, server_ip, false) {
			Ok(top) => {
				// If you return ``true`` in your sautorun/hook.lua file, then don't run the sautorun.CODE that is about to run.
				match lua_type(l, top + 1) {
					rglua::lua::TBOOLEAN => {
						do_run = lua_toboolean(l, top + 1) == 0;
					}
					rglua::lua::TSTRING => {
						// lua_tolstring sets len to new length automatically.
						let nul_str = lua_tolstring(l, top + 1, &mut len);
						code = nul_str;
					}
					_ => (),
				}
				lua_settop(l, top);
			}
			Err(_why) => (),
		}
	}

	if global::FILESTEAL_ENABLED.load(Ordering::Relaxed) {
		if let Some(mut file) = util::get_handle(path, server_ip) {
			if let Err(why) = file.write_all(unsafe { std::ffi::CStr::from_ptr(code) }.to_bytes()) {
				error!(
					"Couldn't write to file made from lua path [{}]. {}",
					path, why
				);
			}
		}
	}

	do_run
}

// Since the first lua state will always be the menu state, just keep a variable for whether joinserver has been hooked or not,
// If not, then hook it.
pub extern "C" fn joinserver_h(state: LuaState) -> i32 {
	let raw_ip = luaL_checkstring(state, 1);

	let ip = try_rstr!(raw_ip).unwrap_or("Unknown");
	info!("Joining Server with IP {}!", ip);

	// Set the IP so we know where to write files in loadbufferx.
	global::SERVER_IP.store(ip, Ordering::Relaxed);
	global::HAS_AUTORAN.store(false, Ordering::Relaxed);

	unsafe { JOINSERVER_H.get().unwrap().call(state) }
}

#[cfg(feature = "runner")]
extern "fastcall" fn paint_traverse_h(
	this: &'static IPanel,
	panel_id: usize,
	force_repaint: bool,
	force_allow: bool,
) {
	unsafe {
		PAINT_TRAVERSE_H
			.get()
			.unwrap()
			.call(this, panel_id, force_repaint, force_allow);
	}

	let script_queue = &mut *global::LUA_SCRIPTS.lock().unwrap();

	if !script_queue.is_empty() {
		let (realm, script) = script_queue.remove(0);

		let state = match realm {
			REALM_MENU => util::get_menu().expect("Menu state DNE in painttraverse. ??"), // Code will never get into the queue without a menu state already existing.
			REALM_CLIENT => util::get_client(),
		};

		if state.is_null() {
			return;
		}

		match lua::dostring(state, &script) {
			Err(why) => {
				error!("{}", why);
			}
			Ok(_) => {
				info!("Script of len #{} ran successfully.", script.len())
			}
		}
	}
}

#[cfg(feature = "runner")]
unsafe fn init_paint_traverse() -> Result<(), detour::Error> {
	use rglua::interface::*;

	let vgui: *mut IPanel = iface!("vgui2", "VGUI_Panel009").unwrap();
	let panel_interface = vgui.as_ref().unwrap();

	// Get painttraverse raw function object to detour.
	let painttraverse: PaintTraverseFn = std::mem::transmute(
		(panel_interface.vtable as *mut *mut c_void)
			.offset(41)
			.read(),
	);

	let detour = detour::GenericDetour::new(painttraverse, paint_traverse_h)?;

	assert!(PAINT_TRAVERSE_H.set(detour).is_ok());
	PAINT_TRAVERSE_H.get().unwrap().enable()?;

	Ok(())
}

pub fn init() -> Result<(), detour::Error> {
	use once_cell::sync::Lazy;

	Lazy::force(&LUAL_LOADBUFFERX_H);
	Lazy::force(&LUAL_NEWSTATE_H);

	#[cfg(feature = "runner")]
	unsafe {
		init_paint_traverse()?
	};

	Ok(())
}

pub fn cleanup() -> Result<(), detour::Error> {
	unsafe {
		LUAL_LOADBUFFERX_H.disable()?;
		LUAL_NEWSTATE_H.disable()?;
		JOINSERVER_H.get().unwrap().disable()?;

		#[cfg(feature = "runner")]
		PAINT_TRAVERSE_H.get().unwrap().disable()?;
	}

	Ok(())
}
