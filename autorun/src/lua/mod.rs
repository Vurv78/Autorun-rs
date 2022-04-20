use std::{borrow::Cow, path::Path};

use crate::{hooks, logging::*, lua};

use autorun_shared::Realm;
use rglua::prelude::*;

mod env;
mod err;

#[cfg(executor)] use std::sync::{Arc, Mutex};
#[cfg(executor)] use once_cell::sync::Lazy;
#[cfg(executor)] type LuaScript = Vec<(autorun_shared::Realm, String)>;

// Scripts waiting to be ran in painttraverse
#[cfg(executor)]
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

	#[cfg(plugins)]
	pub plugin: Option<crate::plugins::Plugin>,
}

// Functions to interact with lua without triggering the detours
pub fn compile<S: AsRef<str>>(l: LuaState, code: S) -> Result<(), Cow<'static, str>> {
	let s = code.as_ref();
	unsafe {
		if hooks::LUAL_LOADBUFFERX_H.call(
			l,
			s.as_ptr().cast(),
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

	Ok(iface.base.cast())
}

#[derive(Debug, thiserror::Error)]
pub enum LuaEnvError {
	#[error("Failed to compile lua code '{0}'")]
	Compile(String),

	#[error("Error during lua runtime '{0}'")]
	Runtime(String),
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
	#[error("Failed to get LUASHARED003 interface")]
	NoInterface(#[from] rglua::interface::Error),

	#[cfg(executor)]
	#[error("Failed to get lua interface")]
	NoLuaInterface,
}

#[cfg(executor)]
pub fn run(realm: Realm, code: String) -> Result<(), RunError> {
	// Check if lua state is valid for instant feedback
	let lua = iface!(LuaShared)?;
	let cl = lua.GetLuaInterface(realm.into());
	if cl.is_null() {
		return Err(RunError::NoLuaInterface);
	} else {
		debug!("Got {realm} interface for run");
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
pub fn run_env_prep<S: AsRef<str>, F: Fn(LuaState), P: AsRef<Path>>(
	l: LuaState,
	script: S,
	path: P,
	env: &AutorunEnv,
	prep: &Option<F>,
) -> Result<i32, LuaEnvError> {
	let path = path.as_ref();
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

		// If this is running before autorun, set Autorun.STARTUP to true.
		lua_pushboolean(l, i32::from(env.startup)); // stack[3] = startup
		lua_setfield(l, -2, cstr!("STARTUP")); // stack[2].STARTUP = table.remove(stack, 3)

		let path_str = path.display().to_string();
		let path_bytes = path_str.as_bytes();
		lua_pushlstring(l, path_bytes.as_ptr().cast(), path_str.len());
		lua_setfield(l, -2, cstr!("PATH")); // stack[2].PATH = table.remove(stack, 3)

		let fns = reg! [
			"log" => env::log,
			"require" => env::require,
			"requirebin" => env::requirebin,
			"print" => env::print,
			"readFile" => env::read
		];

		luaL_register(l, std::ptr::null_mut(), fns.as_ptr());

		if let Some(f) = &prep {
			f(l);
		}
	})
}

pub fn run_env<S: AsRef<str>, P: AsRef<Path>>(
	l: LuaState,
	script: S,
	path: P,
	env: &AutorunEnv,
) -> Result<i32, LuaEnvError> {
	run_env_prep::<S, fn(LuaState), P>(l, script, path, env, &None)
}
