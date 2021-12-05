use std::{fs, io::prelude::*, sync::atomic::Ordering};

use crate::sys::{
	runlua::runLuaEnv,
	statics::*,
	util::{getAutorunHandle, setClientState},
};

use rglua::{
	lua_shared::{self, *},
	rstr,
	types::*,
};

const LUA_BOOL: i32 = rglua::globals::Lua::Type::Bool as i32;
const LUA_STRING: i32 = rglua::globals::Lua::Type::String as i32;

#[macro_use]
pub mod lazy;

// Make our own static detours because detours.rs is lame and locked theirs behind nightly. :)
lazy_detour! {
	pub static LUAL_NEWSTATE_H: extern "C" fn() -> LuaState = (*lua_shared::luaL_newstate, luaL_newstate);
	pub static LUAL_LOADBUFFERX_H: extern "C" fn(LuaState, *const i8, SizeT, *const i8, *const i8) -> i32 = (*lua_shared::luaL_loadbufferx, luaL_loadbufferx);
	pub static JOINSERVER_H: extern "C" fn(LuaState) -> i32;
}

#[cfg(feature = "runner")]
use rglua::interface::IPanel;

type PaintTraverseFn = extern "fastcall" fn(&'static IPanel, usize, bool, bool);

#[cfg(feature = "runner")]
lazy_detour! {
	static PAINT_TRAVERSE_H: PaintTraverseFn;
}

extern "C" fn luaL_newstate() -> LuaState {
	let state = LUAL_NEWSTATE_H.call();
	setClientState(state);
	state
}

extern "C" fn luaL_loadbufferx(
	state: LuaState,
	mut code: *const i8,
	mut size: SizeT,
	identifier: *const i8,
	mode: *const i8,
) -> i32 {
	use crate::sys::util::initMenuState;
	if MENU_STATE.get().is_none() {
		if let Err(why) = initMenuState(state) {
			error!("Couldn't initialize menu state. {}", why);
		}
	}

	// Todo: Check if you're in menu state (Not by checking MENU_DLL because that can be modified by lua) and if so, don't dump files.
	// Dump the file to sautorun-rs/lua_dumps/IP/...
	let raw_path = &rstr!(identifier)[1..]; // Remove the @ from the beginning of the path.
	let server_ip = CURRENT_SERVER_IP.load(Ordering::Relaxed);

	let mut do_run = true;
	if raw_path == "lua/includes/init.lua"
		&& HAS_AUTORAN
			.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
			.is_ok()
	{
		// This will only run once when HAS_AUTORAN is false, setting it to true.
		// Will be reset by JoinServer.
		if let Ok(script) = fs::read_to_string(&*AUTORUN_SCRIPT_PATH) {
			// Try to run here
			if let Err(why) = runLuaEnv(&script, identifier, code, server_ip, true) {
				error!("{}", why);
			}
		} else {
			error!(
				"Couldn't read your autorun script file at [{}]",
				AUTORUN_SCRIPT_PATH.display()
			);
		}
	}

	if let Ok(script) = fs::read_to_string(&*HOOK_SCRIPT_PATH) {
		match runLuaEnv(&script, identifier, code, server_ip, false) {
			Ok(top) => {
				// If you return ``true`` in your sautorun/hook.lua file, then don't run the sautorun.CODE that is about to run.
				match lua_type(state, top + 1) {
					LUA_BOOL => {
						do_run = lua_toboolean(state, top + 1) == 0;
					}
					LUA_STRING => {
						let nul_str = lua_tostring(state, top + 1);
						let cstr = unsafe { std::ffi::CStr::from_ptr(nul_str) };
						let cutoff = cstr.to_bytes(); // String without the null char at the end (lua strings are always nul terminated)

						code = cutoff.as_ptr() as *const i8;
						size = cutoff.len();
					}
					_ => (),
				}
				lua_settop(state, top);
			}
			Err(_why) => (),
		}
	}

	if let Some(mut file) = getAutorunHandle(raw_path, server_ip) {
		if let Err(why) = file.write_all(unsafe { std::ffi::CStr::from_ptr(code) }.to_bytes()) {
			error!(
				"Couldn't write to file made from lua path [{}]. {}",
				raw_path, why
			);
		}
	}

	if do_run {
		// Call the original function and return the value.
		return LUAL_LOADBUFFERX_H.call(state, code, size, identifier, mode);
	}
	0
}

// Since the first lua state will always be the menu state, just keep a variable for whether joinserver has been hooked or not,
// If not, then hook it.
pub extern "C" fn joinserver(state: LuaState) -> i32 {
	let ip = rstr!(lua_tolstring(state, 1, 0));
	info!("Joining Server with IP {}!", ip);

	CURRENT_SERVER_IP.store(ip, Ordering::Relaxed); // Set the IP so we know where to write files in loadbufferx.
	HAS_AUTORAN.store(false, Ordering::Relaxed);

	JOINSERVER_H.get().unwrap().call(state)
}

#[cfg(feature = "runner")]
extern "fastcall" fn paint_traverse(
	this: &'static IPanel,
	panel_id: usize,
	force_repaint: bool,
	force_allow: bool,
) {
	use crate::sys::util::{self, getClientState};

	PAINT_TRAVERSE_H
		.get()
		.unwrap()
		.call(this, panel_id, force_repaint, force_allow);

	let script_queue = &mut *LUA_SCRIPTS.lock().unwrap();

	if !script_queue.is_empty() {
		let (realm, script) = script_queue.remove(0);

		let state = match realm {
			REALM_MENU => MENU_STATE.get().unwrap().load(Ordering::Acquire), // Code will never get into the queue without a menu state already existing.
			REALM_CLIENT => getClientState(),
		};

		if state.is_null() {
			return;
		}

		match util::lua_dostring(state, &script) {
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

	let vgui_interface =
		get_from_interface("VGUI_Panel009", get_interface_handle("vgui2.dll").unwrap()).unwrap()
			as *mut IPanel;

	let panel_interface = vgui_interface.as_ref().unwrap();

	// Get painttraverse raw function object to detour.
	let painttraverse: PaintTraverseFn = std::mem::transmute(
		(panel_interface.vtable as *mut *mut c_void)
			.offset(41)
			.read(),
	);

	let detour = detour::GenericDetour::new(painttraverse, paint_traverse)?;

	assert!(PAINT_TRAVERSE_H.set(detour).is_ok());
	PAINT_TRAVERSE_H.get().unwrap().enable()?;

	Ok(())
}

pub unsafe fn init() -> Result<(), detour::Error> {
	use once_cell::sync::Lazy;

	Lazy::force(&LUAL_LOADBUFFERX_H);
	Lazy::force(&LUAL_NEWSTATE_H);

	#[cfg(feature = "runner")]
	init_paint_traverse()?;

	Ok(())
}

pub unsafe fn cleanup() -> Result<(), detour::Error> {
	LUAL_LOADBUFFERX_H.disable()?;
	LUAL_NEWSTATE_H.disable()?;
	JOINSERVER_H.get().unwrap().disable()?;

	#[cfg(feature = "runner")]
	PAINT_TRAVERSE_H.get().unwrap().disable()?;

	Ok(())
}
