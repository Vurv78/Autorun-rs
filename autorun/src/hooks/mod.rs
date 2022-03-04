use std::ffi::CStr;
use std::io::Write;
use std::{fs, sync::atomic::Ordering};
use std::sync::atomic::AtomicU64;

use crate::plugins;
use crate::{configs, global, lua::{self, AutorunEnv}, util, logging};

use logging::*;
use rglua::interface;
use rglua::prelude::*;

#[macro_use]
pub mod lazy;

// Make our own static detours because detours.rs is lame and locked theirs behind nightly. :)
lazy_detour! {
	pub static LUAL_LOADBUFFERX_H: extern "C" fn(LuaState, *const i8, SizeT, *const i8, *const i8) -> i32 =
		(*LUA_SHARED_RAW.get::<extern "C" fn(LuaState, LuaString, SizeT, LuaString, LuaString) -> i32>(b"luaL_loadbufferx").unwrap(), loadbufferx_h);
}

#[cfg(feature = "runner")]
use rglua::interface::Panel;

type PaintTraverseFn = extern "fastcall" fn(&'static Panel, usize, bool, bool);

#[cfg(feature = "runner")]
lazy_detour! {
	static PAINT_TRAVERSE_H: PaintTraverseFn;
}

static CONNECTED: AtomicU64 = AtomicU64::new(99999);

fn loadbufferx_hook(l: LuaState, code: LuaString, len: usize, identifier: LuaString, mode: LuaString) -> Result<i32, interface::Error> {
	let engine = iface!(EngineClient)?;

	let do_run;
	if engine.IsConnected() {
		let net = engine.GetNetChannelInfo();

		if let Some(net) = unsafe { net.as_mut() } {
			let ip = net.GetAddress();
			let mut startup = false;

			// TODO: It'd be great to hook net connections instead of doing this.
			// However, this works fine for now.
			let curtime = net.GetTimeConnected() as u64;
			if curtime < CONNECTED.load(Ordering::Relaxed) {
				debug!("Curtime is less than last time connected, assuming startup");
				startup = true;
			}

			// Awful
			CONNECTED.store(curtime, Ordering::Relaxed);

			let raw_path = unsafe { CStr::from_ptr(identifier) };
			let path = &raw_path.to_string_lossy()[1..]; // Remove the @ from the beginning of the path

			// There's way too many params here
			do_run = dispatch(l, startup, path, ip, code, len, identifier);
			if !do_run {
				return Ok(0);
			}
		}
	}

	unsafe {
		Ok(LUAL_LOADBUFFERX_H.call(l, code, len, identifier, mode))
	}
}

extern "C" fn loadbufferx_h(
	l: LuaState,
	code: LuaString,
	len: SizeT,
	identifier: LuaString,
	mode: LuaString,
) -> i32 {
	match loadbufferx_hook(l, code, len, identifier, mode) {
		Ok(x) => x,
		Err(why) => {
			error!("Failed to run loadbufferx hook: {}", why);

			unsafe {
				LUAL_LOADBUFFERX_H.call(l, code, len, identifier, mode)
			}
		}
	}
}

pub fn dispatch(l: LuaState, startup: bool, path: &str, ip: LuaString, mut code: LuaString, mut len: SizeT, identifier: LuaString) -> bool {
	let mut do_run = true;

	if startup {
		let env = AutorunEnv {
			is_autorun_file: true,
			startup,

			identifier,
			code,
			code_len: len,

			ip,
			plugin: None
		};

		if let Err(why) = plugins::call_autorun(&env) {
			error!("Failed to call autorun plugins: {why}");
		}

		// This will only run once when HAS_AUTORAN is false, setting it to true.
		// Will be reset by JoinServer.
		let ar_path = configs::path(configs::AUTORUN_PATH);
		trace!("Running autorun script at {}", ar_path.display());

		if let Ok(script) = fs::read_to_string(&ar_path) {
			// Try to run here
			if let Err(why) = lua::run_env(&script, &env) {
				error!("{why}");
			}
		} else {
			debug!(
				"Couldn't read your autorun script file at [{}]",
				ar_path.display()
			);
		}
	}

	if let Ok(script) = fs::read_to_string(configs::path(configs::HOOK_PATH)) {
		let env = AutorunEnv {
			is_autorun_file: false,
			startup,

			identifier,
			code,
			code_len: len,

			ip: ip,
			plugin: None
		};

		if let Err(why) = plugins::call_hook(&env) {
			error!("{why}");
		}

		match lua::run_env(&script, &env) {
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
		let ip = unsafe { CStr::from_ptr(ip) };

		if let Ok(mut file) = util::get_handle(path, ip.to_string_lossy()) {
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

#[cfg(feature = "runner")]
extern "fastcall" fn paint_traverse_h(
	this: &'static Panel,
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

		let lua = iface!(LuaShared);
		match lua {
			Ok(lua) => {
				let iface = lua.GetLuaInterface( realm.into() );
				if let Some(iface) = unsafe { iface.as_mut() } {
					debug!("Got {realm} iface!");
					let state = iface.base as _;

					match lua::dostring(state, &script) {
						Err(why) => {
							error!("{}", why);
						}
						Ok(_) => {
							info!("Script of len #{} ran successfully.", script.len())
						}
					}
				} else {
					error!("Lua interface was null in painttraverse.");
				}
			},
			Err(why) => {
				error!("Failed to get LUASHARED003 interface in painttraverse {why}");
			}
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum HookingError {
	#[error("Failed to hook function: {0}")]
	Detour(#[from] detour::Error),

	#[error("Failed to get interface")]
	Interface(#[from] rglua::interface::Error),

	#[error("Failed to set hook")]
	SetHook,
}

#[cfg(feature = "runner")]
unsafe fn init_paint_traverse() -> Result<(), HookingError> {
	let vgui = iface!(Panel)?;
	// Get painttraverse raw function object to detour.
	let painttraverse: PaintTraverseFn = std::mem::transmute(
		(vgui.vtable as *mut *mut c_void)
			.offset(41)
			.read(),
	);

	let detour = detour::GenericDetour::new(painttraverse, paint_traverse_h)?;

	if let Err(_) = PAINT_TRAVERSE_H.set(detour) {
		return Err(HookingError::SetHook);
	}

	PAINT_TRAVERSE_H.get().unwrap().enable()?;

	Ok(())
}

pub fn init() -> Result<(), HookingError> {
	use once_cell::sync::Lazy;

	Lazy::force(&LUAL_LOADBUFFERX_H);

	#[cfg(feature = "runner")]
	unsafe { init_paint_traverse() }?;

	Ok(())
}

pub fn cleanup() -> Result<(), detour::Error> {
	unsafe {
		LUAL_LOADBUFFERX_H.disable()?;

		#[cfg(feature = "runner")]
		PAINT_TRAVERSE_H.get().unwrap().disable()?;
	}

	Ok(())
}
