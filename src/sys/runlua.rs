#![allow(non_snake_case)]
use std::ffi::CString;
use crate::{
	detours::luaL_loadbufferx_h,
	sys::{
		statics::*,
		funcs::{getClientState, getMenuState}
	}
};

use rglua::{
	lua_shared::*,
	types::{
		CharBuf,
		LuaState
	},
	globals::Lua::{
		MULTRET as LUA_MULTRET,
		GLOBALSINDEX as LUA_GLOBALSINDEX
	},
	rstring
};

const LUA_OK: i32 = 0;
const NO_LUA_STATE: &str = "Didn't run lua code, lua state is not valid/loaded!";
const INVALID_LOG_LEVEL: *const i8 = "Invalid log level (Should be 1-5, 1 being Error, 5 being Trace)\0".as_ptr() as *const i8;

pub fn runLua(realm: Realm, code: String) -> Result<(), &'static str>{
	// Check if lua state is valid for instant feedback
	match realm {
		REALM_MENU => {
			let s = getMenuState();
			match s {
				Some(state) => state,
				None => { return Err("Menu state hasn't been loaded. Hover over a ui icon or something") }
			}
		},
		REALM_CLIENT => {
			let s = getClientState();
			if s == std::ptr::null_mut() { return Err("Client state has not been loaded. Join a server!"); }
			s
		}
	};

	match &mut LUA_SCRIPTS.try_lock() {
		Ok(script_queue) => script_queue.push( (realm, code) ),
		Err(why) => error!("Failed to lock lua_scripts mutex. {}", why)
	};

	Ok(())
}

extern fn log(state: LuaState) -> i32 {
	let s = luaL_checklstring(state, 1, 0);
	let level = luaL_optinteger(state, 2, simplelog::Level::Info as isize);

	let str = rstring!(s);
	match level {
		1 => error!("{}", str),
		2 => warn!("{}", str),
		3 => info!("{}", str),
		4 => debug!("{}", str),
		5 => trace!("{}", str),
		_ => {
			luaL_argerror( state, 2, INVALID_LOG_LEVEL );
		}
	}
	0
}

// Runs lua, but inside of the sautorun environment.
pub fn runLuaEnv(script: &str, identifier: CharBuf, dumped_script: CharBuf, ip: &str, startup: bool) -> Result<(), String> {
	let state = getClientState();

	if state == std::ptr::null_mut() {
		return Err( NO_LUA_STATE.to_owned() );
	}

	let cscript = CString::new(script).map_err(|x| format!("Couldn't convert script to CString. [{}]", x))?;

	if luaL_loadbufferx_h.call(
		state,
		cscript.as_ptr(),
		script.len(),
		"@RunString\0".as_ptr() as CharBuf,
		"bt\0".as_ptr() as CharBuf
	) != LUA_OK {
		// Compile Error
		let err = lua_tolstring(state, -1, 0);
		lua_pop(state, 1);
		return Err( rstring!(err).to_owned() );
	}

	lua_createtable( state, 0, 0 ); // Create our custom environment

	lua_createtable( state, 0, 0 ); // Create the  'sautorun' table

	lua_pushstring( state, identifier );
		lua_setfield( state, -2, "NAME\0".as_ptr() as CharBuf );

		lua_pushstring( state, dumped_script );
		lua_setfield( state, -2, "CODE\0".as_ptr() as CharBuf );

		if let Ok(ip) = CString::new(ip) {
			lua_pushstring( state, ip.as_ptr() );
		} else {
			lua_pushnil(state);
		}
		lua_setfield( state, -2, "IP\0".as_ptr() as CharBuf );

		// If this is running before autorun, set SAUTORUN.STARTUP to true.
		lua_pushboolean( state, startup as i32 );
		lua_setfield( state, -2, "STARTUP\0".as_ptr() as CharBuf );

		lua_pushcfunction( state, log );
		lua_setfield( state, -2, "log\0".as_ptr() as CharBuf );

	lua_setfield( state, -2, "sautorun\0".as_ptr() as CharBuf );

	lua_createtable( state, 0, 0 ); // Create a metatable to make the env inherit from _G
		lua_pushvalue( state, LUA_GLOBALSINDEX );
		lua_setfield( state, -2, "__index\0".as_ptr() as CharBuf );
	lua_setmetatable( state, -2 );

	lua_setfenv( state, -2 );

	if lua_pcall(state, 0, LUA_MULTRET, 0) != 0 {
		let err_runtime = lua_tolstring(state, -1, 0);
		lua_pop(state, 1);
		return Err( rstring!(err_runtime).to_owned() );
	}

	Ok(())
}