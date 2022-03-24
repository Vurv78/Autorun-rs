use std::{borrow::Cow, ffi::CString};

use crate::{hooks, logging::*, lua, plugins::Plugin};

use autorun_shared::Realm;
use rglua::prelude::*;

mod env;
mod err;
mod fs;

#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
use std::sync::{Arc, Mutex};

#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
use once_cell::sync::Lazy;

#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
type LuaScript = Vec<(autorun_shared::Realm, String)>;

// Scripts waiting to be ran in painttraverse
#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
pub static SCRIPT_QUEUE: Lazy<Arc<Mutex<LuaScript>>> =
	Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

pub use env::LOADED_LIBS;

pub struct AutorunEnv {
	// Whether this is the autorun.lua file
	pub is_autorun_file: bool,

	// Whether this is running before autorun
	pub startup: bool,

	pub ip: LuaString,

	// Name/Path of the file being run
	pub identifier: LuaString,

	pub code: LuaString,
	pub code_len: usize,

	pub plugin: Option<crate::plugins::Plugin>,
}

// Functions to interact with lua without triggering the detours
pub fn compile<S: AsRef<str>>(l: LuaState, code: S) -> Result<(), Cow<'static, str>> {
	let s = code.as_ref();
	unsafe {
		if hooks::LUAL_LOADBUFFERX_H.call(
			l,
			s.as_ptr() as _,
			s.len(),
			cstr!("@RunString"),
			cstr!("bt"),
		) != OK
		{
			return Err(get_lua_error(l));
		}
	}

	Ok(())
}

// Helpers

pub unsafe fn get_lua_error(l: LuaState) -> Cow<'static, str> {
	let err = lua_tostring(l, -1);
	lua_pop(l, 1);

	let err = std::ffi::CStr::from_ptr(err);
	err.to_string_lossy()
}

pub fn dostring<S: AsRef<str>>(l: LuaState, script: S) -> Result<(), Cow<'static, str>> {
	compile(l, script)?;
	pcall(l)?;
	Ok(())
}

pub fn pcall(l: LuaState) -> Result<(), Cow<'static, str>> {
	if lua_pcall(l, 0, lua::MULTRET, 0) != OK {
		unsafe {
			return Err(get_lua_error(l));
		}
	}
	Ok(())
}

#[inline(always)]
pub fn get_state(realm: Realm) -> Result<LuaState, rglua::interface::Error> {
	let engine = iface!(LuaShared)?;

	let iface = unsafe { engine.GetLuaInterface(realm.into()).as_mut() }
		.ok_or(rglua::interface::Error::AsMut)?;

	Ok(iface.base as LuaState)
}

#[derive(Debug, thiserror::Error)]
pub enum LuaEnvError {
	#[error("Failed to get lua interface")]
	Interface(#[from] rglua::interface::Error),

	#[error("Failed to compile lua code '{0}'")]
	Compile(String),

	#[error("Error during lua runtime '{0}'")]
	Runtime(String),
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
	#[error("Failed to get LUASHARED003 interface")]
	NoInterface(#[from] rglua::interface::Error),

	#[error("Failed to get lua interface")]
	#[cfg(feature = "runner")]
	#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
	NoLuaInterface,
}

#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
pub fn run(realm: Realm, code: String) -> Result<(), RunError> {
	// Check if lua state is valid for instant feedback
	let lua = iface!(LuaShared)?;
	let cl = lua.GetLuaInterface(realm.into());
	if !cl.is_null() {
		debug!("Got {realm} interface for run");
	} else {
		return Err(RunError::NoLuaInterface);
	}

	match &mut SCRIPT_QUEUE.try_lock() {
		Ok(script_queue) => script_queue.push((realm, code)),
		Err(why) => error!("Failed to lock script queue mutex. {why}"),
	};

	Ok(())
}

/// Runs a lua script after running the provided preparation closure (to add variables to the env, etc)
pub fn run_prepare<S: AsRef<str>, F: Fn(LuaState)>(
	l: LuaState,
	script: S,
	func: F,
) -> Result<i32, LuaEnvError> {
	let top = lua_gettop(l);

	if let Err(why) = lua::compile(l, script) {
		return Err(LuaEnvError::Compile(why.to_string()));
	}

	// stack = {}
	lua_createtable(l, 0, 0); // stack[1] = {}
	lua_createtable(l, 0, 0); // stack[2] = {}

	func(l);

	lua_setfield(l, -2, cstr!("Autorun")); // stack[1].Autorun = table.remove(stack, 2)

	// Create a metatable to make the env inherit from _G
	lua_createtable(l, 0, 1); // stack[2] = {}
	lua_pushvalue(l, GLOBALSINDEX); // stack[3] = _G
	lua_setfield(l, -2, cstr!("__index")); // stack[2].__index = table.remove(stack, 3)
	lua_setmetatable(l, -2); // setmetatable(stack[1], table.remove(stack, 2))

	lua_setfenv(l, -2); // setfenv(l, table.remove(stack, 1))

	if let Err(why) = lua::pcall(l) {
		return Err(LuaEnvError::Runtime(why.to_string()));
	}

	Ok(top)
}

// Runs lua, but inside of the `autorun` environment.
pub fn run_env_prep<S: AsRef<str>, F: Fn(LuaState)>(
	l: LuaState,
	script: S,
	env: &AutorunEnv,
	prep: Option<F>,
) -> Result<i32, LuaEnvError> {
	run_prepare(l, script, |l| {
		// Autorun located at -2

		lua_pushstring(l, env.identifier); // stack[3] = identifier
		lua_setfield(l, -2, cstr!("NAME")); // stack[2].NAME = table.remove(stack, 3)

		lua_pushinteger(l, env.code_len as LuaInteger); // stack[3] = code_len
		lua_setfield(l, -2, cstr!("CODE_LEN")); // stack[2].CODE_LEN = table.remove(stack, 3)

		lua_pushlstring(l, env.code, env.code_len); // stack[3] = identifier
		lua_setfield(l, -2, cstr!("CODE")); // stack[2].CODE = table.remove(stack, 3)

		lua_pushstring(l, env.ip);
		lua_setfield(l, -2, cstr!("IP")); // stack[2].IP = table.remove(stack, 3)

		// If this is running before autorun, set SAUTORUN.STARTUP to true.
		lua_pushboolean(l, env.startup as i32); // stack[3] = startup
		lua_setfield(l, -2, cstr!("STARTUP")); // stack[2].STARTUP = table.remove(stack, 3)

		let fns = reg! [
			"log" => env::log,
			"require" => env::require,
			"requirebin" => env::requirebin,
			"print" => env::print
		];

		luaL_register(l, std::ptr::null_mut(), fns.as_ptr());

		if let Some(f) = &prep {
			f(l);
		}
	})
}

pub fn run_env<S: AsRef<str>>(
	l: LuaState,
	script: S,
	env: &AutorunEnv,
) -> Result<i32, LuaEnvError> {
	run_env_prep::<S, fn(LuaState)>(l, script, env, None)
}

pub fn run_plugin<S: AsRef<str>>(
	l: LuaState,
	script: S,
	env: &AutorunEnv,
	plugin: &Plugin,
) -> Result<i32, LuaEnvError> {
	run_env_prep(
		l,
		script,
		env,
		Some(|l| {
			// print(Autorun.Plugin.NAME, Autorun.Plugin.VERSION)

			lua_createtable(l, 0, 4);

			let name = plugin.get_name();
			if let Ok(name) = CString::new(name.as_bytes()) {
				lua_pushstring(l, name.as_ptr());
				lua_setfield(l, -2, cstr!("NAME"));
			}

			let version = plugin.get_version();
			if let Ok(version) = CString::new(version.as_bytes()) {
				lua_pushstring(l, version.as_ptr());
				lua_setfield(l, -2, cstr!("VERSION"));
			}

			let author = plugin.get_author();
			if let Ok(author) = CString::new(author.as_bytes()) {
				lua_pushstring(l, author.as_ptr());
				lua_setfield(l, -2, cstr!("AUTHOR"));
			}

			if let Some(desc) = plugin.get_description() {
				if let Ok(desc) = CString::new(desc.as_bytes()) {
					lua_pushstring(l, desc.as_ptr());
					lua_setfield(l, -2, cstr!("DESCRIPTION"));
				}
			}

			match plugin.get_settings().as_table() {
				Some(tbl) => {
					lua_createtable(l, 0, tbl.len() as i32);

					fn push_value(l: LuaState, v: &toml::Value) {
						match v {
							toml::Value::String(s) => {
								let bytes = s.as_bytes();
								lua_pushlstring(l, bytes.as_ptr() as _, bytes.len());
							}
							toml::Value::Integer(n) => lua_pushinteger(l, *n as LuaInteger),
							toml::Value::Boolean(b) => lua_pushboolean(l, *b as i32),

							toml::Value::Float(f) => lua_pushnumber(l, *f),

							toml::Value::Array(arr) => {
								lua_createtable(l, arr.len() as i32, 0);

								for (i, v) in arr.iter().enumerate() {
									push_value(l, v);
									lua_rawseti(l, -2, i as i32 + 1);
								}
							}

							toml::Value::Table(tbl) => {
								lua_createtable(l, 0, tbl.len() as i32);

								for (k, v) in tbl.iter() {
									if let Ok(k) = CString::new(k.as_bytes()) {
										push_value(l, v);
										lua_setfield(l, -2, k.as_ptr());
									}
								}
							}

							toml::Value::Datetime(time) => {
								// Just pass a string, smh
								let time = time.to_string();
								let bytes = time.as_bytes();
								lua_pushlstring(l, bytes.as_ptr() as _, bytes.len());
							}
						}
					}

					for (k, v) in tbl.iter() {
						let k = match CString::new(k.as_bytes()) {
							Ok(k) => k,
							Err(_) => continue,
						};

						push_value(l, v);
						lua_setfield(l, -2, k.as_ptr());
					}
				}
				None => lua_createtable(l, 0, 0),
			}

			lua_setfield(l, -2, cstr!("Settings"));
			lua_setfield(l, -2, cstr!("Plugin"));
		}),
	)
}
