use crate::{hooks, lua, util, logging::*, global};
use autorun_shared::{Realm, REALM_CLIENT, REALM_MENU};

use rglua::prelude::*;

use std::ffi::CString;

mod env;
mod err;

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

	#[error("Failed to compile lua code '{0}'")]
	Compile(String),

	#[error("Error during lua runtime '{0}'")]
	Runtime(String)
}

pub fn run(realm: Realm, code: String) -> Result<(), &'static str> {
	// Check if lua state is valid for instant feedback
	match realm {
		REALM_MENU => {
			let s = util::get_menu();
			match s {
				Some(l) => l,
				None => {
					return Err("Menu state hasn't been loaded. Hover over a ui icon or something")
				}
			}
		}
		REALM_CLIENT => {
			let s = util::get_client();
			if s.is_null() {
				return Err("Client state has not been loaded. Join a server!");
			}
			s
		}
	};

	match &mut global::LUA_SCRIPTS.try_lock() {
		Ok(script_queue) => script_queue.push((realm, code)),
		Err(why) => error!("Failed to lock lua_scripts mutex. {}", why),
	};

	Ok(())
}

// Runs lua, but inside of the `autorun` environment.
pub fn run_with_env(
	script: &str,
	identifier: LuaString,
	dumped_script: LuaString,
	len: SizeT,
	ip: &str,
	startup: bool,
) -> Result<i32, LuaEnvError> {
	let l = util::get_client();
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

	lua_pushstring(l, identifier); // stack[3] = identifier
	lua_setfield(l, -2, cstr!("NAME")); // stack[2].NAME = table.remove(stack, 3)

	lua_pushlstring(l, dumped_script, len); // stack[3] = identifier
	lua_setfield(l, -2, cstr!("CODE")); // stack[2].CODE = table.remove(stack, 3)

	if let Ok(ip) = CString::new(ip) {
		lua_pushstring(l, ip.as_ptr()); // stack[3] = ip
	} else {
		lua_pushnil(l); // stack[3] = nil
	}
	lua_setfield(l, -2, cstr!("IP")); // stack[2].IP = table.remove(stack, 3)

	// If this is running before autorun, set SAUTORUN.STARTUP to true.
	lua_pushboolean(l, startup as i32); // stack[3] = startup
	lua_setfield(l, -2, cstr!("STARTUP")); // stack[2].STARTUP = table.remove(stack, 3)

	let fns = reg! [
		"log" => env::log,
		"require" => env::require
	];

	luaL_register(l, std::ptr::null_mut(), fns.as_ptr());

	lua_setfield(l, -2, cstr!("sautorun")); // stack[1].sautorun = table.remove(stack, 2)

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
