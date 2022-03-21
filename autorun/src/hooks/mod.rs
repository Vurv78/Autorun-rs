use std::ffi::CStr;
use std::io::Write;
use std::{fs, sync::atomic::Ordering};
use std::sync::atomic::AtomicU64;

use crate::plugins;
use crate::{configs::{self, SETTINGS}, lua::{self, AutorunEnv}, util, logging};

use logging::*;
use rglua::interface;
use rglua::prelude::*;

pub mod lazy;
use lazy::lazy_detour;

// Make our own static detours because detours.rs is lame and locked theirs behind nightly. :)
lazy_detour! {
	pub static LUAL_LOADBUFFERX_H: extern "C" fn(LuaState, *const i8, SizeT, *const i8, *const i8) -> i32 = (
		{
			*LUA_SHARED_RAW.get::<extern "C" fn(LuaState, LuaString, SizeT, LuaString, LuaString) -> i32>(b"luaL_loadbufferx")
				.expect("Failed to get luaL_loadbufferx")
		},
		loadbufferx_h
	);

	#[cfg(feature = "runner")]
	#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
	pub static PAINT_TRAVERSE_H: PaintTraverseFn = (
		{
			let vgui = iface!(Panel).expect("Failed to get Panel interface");
			std::mem::transmute::<_, PaintTraverseFn>(
				(vgui.vtable as *mut *mut c_void)
					.offset(41)
					.read(),
			)
		},
		paint_traverse_h
	);
}

#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
use rglua::interface::Panel;

#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
type PaintTraverseFn = extern "fastcall" fn(&'static Panel, usize, bool, bool);

static CONNECTED: AtomicU64 = AtomicU64::new(99999);

pub struct DispatchParams<'a> {
	ip: LuaString,
	code: LuaString,
	code_len: usize,
	identifier: LuaString,

	startup: bool,
	path: &'a str,
	#[allow(unused)]
	engine: &'a mut interface::EngineClient,
	net: &'a mut interface::NetChannelInfo,
}

fn loadbufferx_hook(l: LuaState, code: LuaString, code_len: usize, identifier: LuaString, mode: LuaString) -> Result<i32, interface::Error> {
	let mut engine = iface!(EngineClient)?;

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
			let params = DispatchParams {
				ip,
				code,
				code_len,
				identifier,
				startup,
				path,

				engine: &mut engine,
				net
			};

			do_run = dispatch(l, params);
			if !do_run {
				return Ok(0);
			}
		}
	}

	unsafe {
		Ok(LUAL_LOADBUFFERX_H.call(l, code, code_len, identifier, mode))
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

pub fn dispatch(l: LuaState, mut params: DispatchParams) -> bool {
	let mut do_run = true;

	if params.startup {
		let env = AutorunEnv {
			is_autorun_file: true,
			startup: params.startup,

			identifier: params.identifier,
			code: params.code,
			code_len: params.code_len,

			ip: params.ip,
			plugin: None
		};

		if let Err(why) = plugins::call_autorun(l, &env) {
			error!("Failed to call plugins (autorun): {why}");
		}
		// This will only run once when HAS_AUTORAN is false, setting it to true.
		// Will be reset by JoinServer.
		let ar_path = configs::path(configs::AUTORUN_PATH);
		trace!("Running autorun script at {}", ar_path.display());

		if let Ok(script) = fs::read_to_string(&ar_path) {
			// Try to run here
			if let Err(why) = lua::run_env(l, &script, &env) {
				error!("{why}");
			}
		} else {
			debug!(
				"Couldn't read your autorun script file at [{}]",
				ar_path.display()
			);
		}
	}

	{
		// Calling hook.lua
		let env = AutorunEnv {
			is_autorun_file: false,
			startup: params.startup,

			identifier: params.identifier,
			code: params.code,
			code_len: params.code_len,

			ip: params.ip,
			plugin: None
		};

		if SETTINGS.plugins.enabled {
			if let Err(why) = plugins::call_hook(l, &env) {
				error!("Failed to call plugins (hook): {why}");
			}
		}

		if let Ok(script) = fs::read_to_string(configs::path(configs::HOOK_PATH)) {
			match lua::run_env(l, &script, &env) {
				Ok(top) => {
					// If you return ``true`` in your sautorun/hook.lua file, then don't run the sautorun.CODE that is about to run.
					match lua_type(l, top + 1) {
						rglua::lua::TBOOLEAN => {
							do_run = lua_toboolean(l, top + 1) == 0;
						}
						rglua::lua::TSTRING => {
							// lua_tolstring sets len to new length automatically.
							let nul_str = lua_tolstring(l, top + 1, &mut params.code_len);
							params.code = nul_str;
						}
						_ => (),
					}
					lua_settop(l, top);
				}
				Err(_why) => (),
			}
		}
	}

	if SETTINGS.filesteal.enabled {
		let mut fmt = SETTINGS.filesteal.format.clone();
		if fmt.contains("<ip>") {
			let ip = unsafe { CStr::from_ptr(params.ip) };
			let ip = ip.to_string_lossy();

			fmt = fmt.replace("<ip>", &ip);
		}

		if fmt.contains("<hostname>") {
			let hostname = params.net.GetName();
			let hostname = unsafe { CStr::from_ptr(hostname) };
			let hostname = hostname.to_string_lossy();

			fmt = fmt.replace("<hostname>", &hostname);
		}

		if let Ok(mut file) = util::get_handle(params.path, fmt) {
			let code = unsafe { CStr::from_ptr(params.code) };
			if let Err(why) = file.write_all(code.to_bytes()) {
				error!(
					"Couldn't write to file made from lua path [{}]. {}",
					params.path, why
				);
			}
		}
	}

	do_run
}

#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
extern "fastcall" fn paint_traverse_h(
	this: &'static Panel,
	panel_id: usize,
	force_repaint: bool,
	force_allow: bool,
) {
	unsafe {
		PAINT_TRAVERSE_H.call(this, panel_id, force_repaint, force_allow);
	}

	match lua::SCRIPT_QUEUE.try_lock() {
		Ok(ref mut queue) => {
			if !queue.is_empty() {
				let (realm, script) = queue.remove(0);

				match lua::get_state(realm) {
					Ok(state) => {
						debug!("Got {realm} iface!");
						match lua::dostring(state, &script) {
							Err(why) => error!("{why}"),
							Ok(_) => info!("Script of len #{} ran successfully.", script.len())
						}
					},
					Err(why) => error!("{why}")
				}
			}
		},
		Err(_) => return,
	}
}

#[derive(Debug, thiserror::Error)]
pub enum HookingError {
	#[error("Failed to hook function: {0}")]
	Detour(#[from] detour::Error),

	#[error("Failed to get interface")]
	Interface(#[from] rglua::interface::Error)
}

pub fn init() -> Result<(), HookingError> {
	use once_cell::sync::Lazy;

	Lazy::force(&LUAL_LOADBUFFERX_H);

	#[cfg(feature = "runner")]
	#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
	Lazy::force(&PAINT_TRAVERSE_H);

	Ok(())
}

pub fn cleanup() -> Result<(), detour::Error> {
	unsafe {
		LUAL_LOADBUFFERX_H.disable()?;

		#[cfg(feature = "runner")]
		#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
		PAINT_TRAVERSE_H.disable()?;
	}

	Ok(())
}
