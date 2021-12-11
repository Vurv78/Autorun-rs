#![allow(non_snake_case)]
use crate::sys::{
	statics::*,
	util::{self, getClientState, getMenuState},
};
use std::ffi::CString;

use rglua::{cstr, globals::Lua::GLOBALSINDEX, lua_shared::*, rstr, types::LuaState};

const NO_LUA_STATE: &str = "Didn't run lua code, lua state is not valid/loaded!";
const INVALID_LOG_LEVEL: *const i8 =
	cstr!("Invalid log level (Should be 1-5, 1 being Error, 5 being Trace)");

pub fn runLua(realm: Realm, code: String) -> Result<(), &'static str> {
	// Check if lua state is valid for instant feedback
	match realm {
		REALM_MENU => {
			let s = getMenuState();
			match s {
				Some(L) => L,
				None => {
					return Err("Menu state hasn't been loaded. Hover over a ui icon or something")
				}
			}
		}
		REALM_CLIENT => {
			let s = getClientState();
			if s.is_null() {
				return Err("Client state has not been loaded. Join a server!");
			}
			s
		}
	};

	match &mut LUA_SCRIPTS.try_lock() {
		Ok(script_queue) => script_queue.push((realm, code)),
		Err(why) => error!("Failed to lock lua_scripts mutex. {}", why),
	};

	Ok(())
}

extern "C" fn log(L: LuaState) -> i32 {
	let s = luaL_checklstring(L, 1, 0);
	let level = luaL_optinteger(L, 2, 3); // INFO by default

	let str = rstr!(s);
	match level {
		1 => error!("{}", str),
		2 => warn!("{}", str),
		3 => info!("{}", str),
		4 => debug!("{}", str),
		5 => trace!("{}", str),
		_ => {
			luaL_argerror(L, 2, INVALID_LOG_LEVEL);
		}
	}
	0
}

// https://github.com/lua/lua/blob/eadd8c7178c79c814ecca9652973a9b9dd4cc71b/loadlib.c#L657
extern "C" fn require(L: LuaState) -> i32 {
	use rglua::prelude::*;
	use std::{fs::File, io::prelude::*, path::PathBuf};

	let raw_path = luaL_checklstring(L, 1, 0);

	let path_str = util::sanitizePath(rstr!(raw_path));
	let path = SAUTORUN_SCRIPT_DIR.join::<PathBuf>(path_str.into());

	match File::open(&path) {
		Err(why) => error!(
			"Failed to sautorun.require path [{}] [{}]",
			path.display(),
			why
		),
		Ok(mut handle) => {
			let mut script = String::new();
			let top = lua_gettop(L);
			if let Err(why) = handle.read_to_string(&mut script) {
				error!(
					"Failed to read script from file [{}]. Reason: {}",
					path.display(),
					why
				);
			} else if let Err(why) = util::lua_dostring(L, &script) {
				error!("Error when requiring [{}]. [{}]", path.display(), why);
			}
			return lua_gettop(L) - top;
		}
	}

	0
}

// Runs lua, but inside of the sautorun environment.
pub fn runLuaEnv(
	script: &str,
	identifier: *const i8,
	dumped_script: *const i8,
	ip: &str,
	startup: bool,
) -> Result<i32, String> {
	let L = getClientState();

	if L.is_null() {
		return Err(NO_LUA_STATE.to_owned());
	}

	let top = lua_gettop(L);

	if let Err(why) = util::lua_compilestring(L, script) {
		return Err(why.to_owned());
	}

	// stack = {}
	lua_createtable(L, 0, 0); // stack[1] = {}
	lua_createtable(L, 0, 0); // stack[2] = {}

	lua_pushstring(L, identifier); // stack[3] = identifier
	lua_setfield(L, -2, cstr!("NAME")); // stack[2].NAME = table.remove(stack, 3)

	lua_pushstring(L, dumped_script); // stack[3] = identifier
	lua_setfield(L, -2, cstr!("CODE")); // stack[2].CODE = table.remove(stack, 3)

	if let Ok(ip) = CString::new(ip) {
		lua_pushstring(L, ip.as_ptr()); // stack[3] = ip
	} else {
		lua_pushnil(L); // stack[3] = nil
	}
	lua_setfield(L, -2, cstr!("IP")); // stack[2].IP = table.remove(stack, 3)

	// If this is running before autorun, set SAUTORUN.STARTUP to true.
	lua_pushboolean(L, startup as i32); // stack[3] = startup
	lua_setfield(L, -2, cstr!("STARTUP")); // stack[2].STARTUP = table.remove(stack, 3)

	let fns = rglua::reg! [
		"log" => log,
		"require" => require
	];
	luaL_register(L, std::ptr::null_mut(), fns.as_ptr());

	lua_pushcfunction(L, log); // stack[3] = log
	lua_setfield(L, -2, cstr!("log")); // stack[2].log = table.remove(stack, 3)

	lua_pushcfunction(L, require); // stack[3] = require
	lua_setfield(L, -2, cstr!("require")); // stack[2].require = table.remove(stack, 3)

	lua_setfield(L, -2, cstr!("sautorun")); // stack[1].sautorun = table.remove(stack, 2)

	// Create a metatable to make the env inherit from _G
	lua_createtable(L, 0, 0); // stack[2] = {}
	lua_pushvalue(L, GLOBALSINDEX); // stack[3] = _G
	lua_setfield(L, -2, cstr!("__index")); // stack[2].__index = table.remove(stack, 3)
	lua_setmetatable(L, -2); // setmetatable(stack[1], table.remove(stack, 2))

	lua_setfenv(L, -2); // setfenv(L, table.remove(stack, 1))

	if let Err(why) = util::lua_pexec(L) {
		return Err(why.to_owned());
	}
	Ok(top)
}
