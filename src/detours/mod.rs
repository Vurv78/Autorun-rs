use std::{fs, io::prelude::*, sync::atomic::Ordering};

use crate::sys::{
	funcs::{getAutorunHandle, getClientState, setClientState},
	runlua::runLuaEnv, statics::*
};

use rglua::{
	lua_shared::{self, *},
	types::*,
	rstring,
	interface::IPanel
};

use detour::static_detour;

static_detour! {
	pub static luaL_newstate_h: extern "C" fn() -> LuaState;
	pub static luaL_loadbufferx_h: extern "C" fn(LuaState, CharBuf, SizeT, CharBuf, CharBuf) -> CInt;
	pub static joinserver_h: extern "C" fn(LuaState) -> CInt;
	pub static paint_traverse_h: extern "thiscall" fn(&'static IPanel, usize, bool, bool);
}

fn luaL_newstate() -> LuaState {
	let state = luaL_newstate_h.call();
	info!("Got client state through luaL_newstate");
	setClientState(state);
	state
}

fn luaL_loadbufferx(state: LuaState, code: CharBuf, size: SizeT, identifier: CharBuf, mode: CharBuf) -> CInt {
	use crate::sys::funcs::initMenuState;
	if MENU_STATE.get().is_none() {
		initMenuState(state)
			.expect("Couldn't initialize menu state");
	}

	// Todo: Check if you're in menu state (Not by checking MENU_DLL because that can be modified by lua) and if so, don't dump files.
	// Dump the file to sautorun-rs/lua_dumps/IP/...
	let raw_path = &rstring!(identifier)[1 ..]; // Remove the @ from the beginning of the path.
	let server_ip = CURRENT_SERVER_IP.load( Ordering::Relaxed );

	let mut autoran = false;
	let mut do_run = true;
	if raw_path == "lua/includes/init.lua" {
		if HAS_AUTORAN.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
			// This will only run once when HAS_AUTORAN is false, setting it to true.
			// Will be reset by JoinServer.
			if let Ok(script) = fs::read_to_string(&*AUTORUN_SCRIPT_PATH) {
				// Try to run here
				if let Err(why) = runLuaEnv(&script, identifier, code, server_ip, true) {
					error!("{}", why);
				}
				autoran = true;
			} else {
				error!( "Couldn't read your autorun script file at {}/{}", SAUTORUN_DIR.display(), AUTORUN_SCRIPT_PATH.display() );
			}
		}
	}

	if !autoran {
		if let Ok(script) = fs::read_to_string(&*HOOK_SCRIPT_PATH) {
			match runLuaEnv(&script, identifier, code, server_ip, false) {
				Ok(_) => {
					// If you return ``true`` in your sautorun/hook.lua file, then don't run the sautorun.CODE that is about to run.
					if lua_type(state, 1) == rglua::globals::Lua::Type::Bool as i32 {
						do_run = lua_toboolean(state, 1) == 0;
						lua_pop(state, 1);
					}
				},
				Err(why) => error!("{}", why)
			}
		}

	}

	if let Some(mut file) = getAutorunHandle(raw_path, server_ip) {
		if let Err(why) = file.write_all( unsafe { std::ffi::CStr::from_ptr(code) }.to_bytes() ) {
			error!("Couldn't write to file made from lua path [{}]. {}", raw_path, why);
		}
	}

	if do_run {
		// Call the original function and return the value.
		return luaL_loadbufferx_h.call( state, code, size, identifier, mode );
	}
	0
}

// Since the first lua state will always be the menu state, just keep a variable for whether joinserver has been hooked or not,
// If not, then hook it.
pub fn joinserver(state: LuaState) -> CInt {
	let ip = rstring!( lua_tolstring(state, 1, 0) );
	info!("Joining Server with IP {}!", ip);

	CURRENT_SERVER_IP.store(ip, Ordering::Relaxed); // Set the IP so we know where to write files in loadbufferx.
	HAS_AUTORAN.store(false, Ordering::Relaxed);

	joinserver_h.call(state)
}

fn paint_traverse(this: &'static IPanel, panel_id: usize, force_repaint: bool, force_allow: bool) {
	paint_traverse_h.call(this, panel_id, force_repaint, force_allow);

	let script_queue = &mut *LUA_SCRIPTS
		.lock()
		.unwrap();

	if script_queue.len() > 0 {
		let (realm, script) = script_queue.remove(0);

		let state = match realm {
			REALM_MENU => MENU_STATE.get().unwrap().load(Ordering::Acquire), // Code will never get into the queue without a menu state already existing.
			REALM_CLIENT => getClientState()
		};

		if state == std::ptr::null_mut() { return; }

		if luaL_loadbufferx_h.call(
			state,
			script.as_ptr() as *const i8,
			script.len(),
			"@RunString\0".as_ptr() as CharBuf,
			"bt\0".as_ptr() as CharBuf
		) != 0 || lua_pcall(state, 0, 0, 0) != 0 {
			let err = lua_tostring(state, -1);
			lua_pop(state, 1);
			error!("{}", rstring!(err));
		} else {
			info!("Code [len {}] ran successfully.", script.len())
		}
	}
}

pub unsafe fn init() -> Result<(), detour::Error> {
	luaL_loadbufferx_h
		.initialize(*lua_shared::luaL_loadbufferx, luaL_loadbufferx)?
		.enable()?;

	luaL_newstate_h
		.initialize(*lua_shared::luaL_newstate, luaL_newstate)?
		.enable()?;

	use rglua::interface::*;

	let vgui_interface = get_from_interface( "VGUI_Panel009", get_interface_handle("vgui2.dll").unwrap() )
		.unwrap() as *mut IPanel;

	let panel_interface = vgui_interface.as_ref().unwrap();

	type PaintTraverseFn = extern "thiscall" fn(&'static IPanel, usize, bool, bool);
	// Get painttraverse raw function object to detour.
	let painttraverse: PaintTraverseFn = std::mem::transmute(
		(panel_interface.vtable as *mut *mut CVoid)
			.offset(41)
			.read()
	);

	paint_traverse_h
		.initialize( painttraverse, paint_traverse )?
		.enable()?;

	Ok(())
}

pub unsafe fn cleanup() -> Result<(), detour::Error>{
	luaL_loadbufferx_h.disable()?;
	luaL_newstate_h.disable()?;
	joinserver_h.disable()?;
	paint_traverse_h.disable()?;

	Ok(())
}