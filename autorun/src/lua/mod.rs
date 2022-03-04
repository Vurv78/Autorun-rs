use std::ffi::CString;

use crate::{hooks, lua, logging::*, global, plugins::Plugin};
use autorun_shared::Realm;
use rglua::prelude::*;

mod env;
mod err;

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

impl AutorunEnv {
	pub fn from_lua(state: LuaState) -> Option<Self> {
		lua_getglobal(state, cstr!("sautorun"));
		if lua_type(state, -1) == rglua::lua::TTABLE {
			lua_getfield(state, -1, cstr!("STARTUP"));
			let startup = lua_toboolean(state, -1) != 0;

			lua_getfield(state, -1, cstr!("NAME"));
			let identifier = lua_tostring(state, -1);

			lua_getfield(state, -1, cstr!("CODE_LEN"));
			let mut code_len = lua_tointeger(state, -1) as usize;

			lua_getfield(state, -1, cstr!("CODE"));
			let code = lua_tolstring(state, -1, &mut code_len);

			lua_getfield(state, -1, cstr!("IP"));
			let ip = lua_tostring(state, -1);

			lua_pop(state, 6);

			return Some(AutorunEnv {
				is_autorun_file: false,
				startup,
				code,
				identifier,
				code_len,
				ip,
				plugin: None,
			});
		}

		lua_pop(state, 1);
		None
	}
}

// Functions to interact with lua without triggering the detours
pub fn compile<S: AsRef<str>>(l: LuaState, code: S) -> Result<(), &'static str> {
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
			return Err(get_lua_error(l).expect("Couldn't get lua error"));
		}
	}

	Ok(())
}

// Helpers

pub unsafe fn get_lua_error(l: LuaState) -> Option<&'static str> {
	let err = lua_tostring(l, -1);

	lua_pop(l, 1);
	Some(try_rstr!(err).unwrap_or("UTF8 Error"))
}

pub fn dostring<S: AsRef<str>>(l: LuaState, script: S) -> Result<(), &'static str> {
	compile(l, script)?;
	pcall(l)?;
	Ok(())
}

pub fn pcall(l: LuaState) -> Result<(), &'static str> {
	if lua_pcall(l, 0, lua::MULTRET, 0) != OK {
		unsafe {
			return Err(get_lua_error(l).expect("Failed to get lua error in pcall"));
		}
	}
	Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum LuaEnvError {
	#[error("Failed to get lua state")]
	NoState,

	#[error("Failed to get lua interface")]
	Interface(#[from] rglua::interface::Error),

	#[error("Failed to compile lua code '{0}'")]
	Compile(String),

	#[error("Error during lua runtime '{0}'")]
	Runtime(String)
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
	#[error("Failed to get LUASHARED003 interface")]
	NoInterface(#[from] rglua::interface::Error),

	#[error("Failed to get lua interface")]
	NoLuaInterface,
}

// TODO: Might be able to make this synchronous by using LuaInterface.RunString (But that might also trigger detours..)
pub fn run(realm: Realm, code: String) -> Result<(), RunError> {
	// Check if lua state is valid for instant feedback
	let lua = rglua::iface!(LuaShared)?;
	let cl = lua.GetLuaInterface(realm.into());
	if !cl.is_null() {
		debug!("Got {realm} interface for run");
	} else {
		return Err(RunError::NoLuaInterface);
	}

	match &mut global::LUA_SCRIPTS.try_lock() {
		Ok(script_queue) => script_queue.push((realm, code)),
		Err(why) => error!("Failed to lock lua_scripts mutex. {}", why),
	};

	Ok(())
}

/// Runs a lua script after running the provided preparation closure (to add variables to the env, etc)
pub fn run_prepare<S: AsRef<str>, F: Fn(LuaState)>(script: S, func: F) -> Result<i32, LuaEnvError> {
	let lua = iface!(LuaShared)?;
	let iface = lua.GetLuaInterface(Realm::Client.into());
	let iface = unsafe { iface.as_mut() }.ok_or(LuaEnvError::NoState)?;

	let l = iface.base as LuaState;

	if l.is_null() {
		return Err(LuaEnvError::NoState);
	}

	let top = lua_gettop(l);

	if let Err(why) = lua::compile(l, script) {
		return Err(LuaEnvError::Compile(why.to_string()));
	}

	// stack = {}
	lua_createtable(l, 0, 0); // stack[1] = {}
	lua_createtable(l, 0, 0); // stack[2] = {}

	func(l);

	// Backwards compatibility
	lua_pushvalue(l, -2);
	lua_setfield(l, -3, cstr!("sautorun")); // stack[1].sautorun = table.remove(stack, 2)

	lua_setfield(l, -2, cstr!("Autorun"));

	// Create a metatable to make the env inherit from _G
	lua_createtable(l, 0, 0); // stack[2] = {}
	lua_pushvalue(l, GLOBALSINDEX); // stack[3] = _G
	lua_setfield(l, -2, cstr!("__index")); // stack[2].__index = table.remove(stack, 3)
	lua_setmetatable(l, -2); // setmetatable(stack[1], table.remove(stack, 2))

	lua_setfenv(l, -2); // setfenv(l, table.remove(stack, 1))

	if let Err(why) = lua::pcall(l) {
		return Err(LuaEnvError::Runtime(why.to_owned()));
	}

	Ok(top)
}

// Runs lua, but inside of the `autorun` environment.
pub fn run_env_prep<S: AsRef<str>, F: Fn(LuaState)>(script: S, env: &AutorunEnv, prep: Option<F>) -> Result<i32, LuaEnvError> {
	run_prepare(script, |l| {
		// sautorun located at -2

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
			"require" => env::require
		];

		luaL_register(l, std::ptr::null_mut(), fns.as_ptr());

		if let Some(f) = &prep {
			f(l);
		}
	})
}

pub fn run_env<S: AsRef<str>>(script: S, env: &AutorunEnv) -> Result<i32, LuaEnvError> {
	run_env_prep::<S, fn(LuaState)>(script, env, None)
}

pub fn run_plugin<S: AsRef<str>>(script: S, env: &AutorunEnv, plugin: &Plugin) -> Result<i32, LuaEnvError> {
	run_env_prep(script, env, Some(|l| {
		// print(Autorun.Plugin.NAME, Autorun.Plugin.VERSION)

		lua_createtable(l, 0, 4);

		let name = plugin.get_name();
		if let Ok(name) = CString::new( name.as_bytes() ) {
			lua_pushstring(l, name.as_ptr());
			lua_setfield(l, -2, cstr!("NAME"));
		}

		let version = plugin.get_version();
		if let Ok(version) = CString::new( version.as_bytes() ) {
			lua_pushstring(l, version.as_ptr());
			lua_setfield(l, -2, cstr!("VERSION"));
		}

		let author = plugin.get_author();
		if let Ok(author) = CString::new( author.as_bytes() ) {
			lua_pushstring(l, author.as_ptr());
			lua_setfield(l, -2, cstr!("AUTHOR"));
		}

		// Todo
		lua_createtable(l, 0, 0);
		match plugin.get_settings().as_table() {
			Some(tbl) => {
				for (k, v) in tbl.iter() {
					match CString::new( k.as_bytes() ) {
						Ok(k) => {
							lua_pushstring(l, k.as_ptr());
						}
						Err(_) => { continue }
					};

					let success = match v {
						toml::Value::String(s) => {
							if let Ok(s) = CString::new( s.as_bytes() ) {
								lua_pushstring(l, s.as_ptr());
								true
							} else {
								false
							}
						},
						toml::Value::Integer(n) => {
							lua_pushinteger(l, *n as LuaInteger);
							true
						},
						toml::Value::Boolean(b) => {
							lua_pushboolean(l, *b as i32);
							true
						},

						toml::Value::Float(f) => {
							lua_pushnumber(l, *f);
							true
						}

						// Array, table, etc currently not supported.
						_ => false,
					};

					if success {
						lua_settable(l, -3);
					} else {
						// Pop unused key
						lua_pop(l, 1);
					}
				}
			},
			None => ()
		}

		lua_setfield(l, -2, cstr!("Settings"));

		lua_setfield(l, -3, cstr!("Plugin"));
	}))
}
