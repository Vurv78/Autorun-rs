#![allow(non_snake_case)]
use std::ffi::CString;

use crate::sys::{
	statics::*,
	funcs::getLuaState
};

use rglua::{
	lua_shared::*,
	types::{
		CharBuf,
		LuaState
	},
	rstring
};

// Runs lua code through loadbufferx. Returns whether it successfully ran.
pub fn runLua(code: &str) -> Result<(), String> {
	let state = getLuaState();

	if state == std::ptr::null_mut() {
		return Err( "Invalid lua state".to_owned() );
	}

	let buf_code = match CString::new(code) {
		Ok(code) => code,
		Err(why) => {
			return Err( format!("Couldn't convert code into a CString {}", why) );
		}
	};

	let status = LUAL_LOADBUFFERX.call(
		state,
		buf_code.as_ptr(),
		std::mem::size_of_val(code),
		b"@RunString\0".as_ptr() as CharBuf,
		b"bt\0".as_ptr() as CharBuf
	);

	if status != 0 {
		// Compile Error
		let err = lua_tolstring(state, -1, 0);
		lua_settop(state, -2);
		return Err( rstring!(err).to_owned() );
	}

	if lua_pcall(state, 0, rglua::globals::Lua::MULTRET, 0) != 0 {
		let err_runtime = rstring!( lua_tolstring(state, -1, 0) );
		lua_settop(state, -2);
		return Err( err_runtime.to_owned() );
	}

	Ok(())
}

extern fn log(state: LuaState) -> i32 {
	let s = luaL_checklstring(state, 1, 0);
	let mut level = simplelog::Level::Info as i32;
	if lua_type(state, 2) == rglua::globals::Lua::Type::Number as i32 {
		level = lua_tointeger(state, 2) as i32;
	}

	let str = rstring!(s);
	match level {
		1 => error!("{}", str),
		2 => warn!("{}", str),
		3 => info!("{}", str),
		4 => debug!("{}", str),
		5 => trace!("{}", str),
		_ => {
			luaL_argerror( state, 2, b"Invalid log level (Should be 1-5, 1 being Error, 5 being Trace)\0".as_ptr() as *const i8 );
		}
	}
	0
}

// Runs lua, but inside of the sautorun environment.
pub fn runLuaEnv(script: &str, identifier: CharBuf, dumped_script: CharBuf, ip: &str, startup: bool) -> Result<(), String> {
	let state = getLuaState();

	if state == std::ptr::null_mut() {
		return Err( "Didn't run lua code, make sure the lua state is valid!".to_owned() );
	}

	let loadbufx_hook = &*LUAL_LOADBUFFERX;

	let cscript = match std::ffi::CString::new(script) {
		Err(why) => {
			return Err( format!("Couldn't transform script into CString. {}", why) );
		}
		Ok(b) => b
	};

	let status = loadbufx_hook.call(state,
		cscript.as_ptr(),
		std::mem::size_of_val(script),
		b"@RunString\0".as_ptr() as CharBuf,
		b"bt\0".as_ptr() as CharBuf
	);

	if status != 0 {
		// Compile Error
		let err = lua_tolstring(state, -1, 0);
		lua_settop(state, -2);
		return Err( rstring!(err).to_owned() );
	}

	lua_createtable( state, 0, 0 ); // Create our custom environment

	lua_createtable( state, 0, 0 ); // Create the  'sautorun' table

	lua_pushstring( state, identifier );
		lua_setfield( state, -2, b"NAME\0".as_ptr() as CharBuf );

		lua_pushstring( state, dumped_script );
		lua_setfield( state, -2, b"CODE\0".as_ptr() as CharBuf );

		if let Ok(ip) = CString::new(ip) {
			lua_pushstring( state, ip.as_ptr() );
		} else {
			lua_pushnil(state);
		}
		lua_setfield( state, -2, b"IP\0".as_ptr() as CharBuf );

		// If this is running before autorun, set SAUTORUN.STARTUP to true.
		lua_pushboolean( state, startup as i32 );
		lua_setfield( state, -2, b"STARTUP\0".as_ptr() as CharBuf );

		lua_pushcfunction( state, log );
		lua_setfield( state, -2, b"log\0".as_ptr() as CharBuf );

	lua_setfield( state, -2, b"sautorun\0".as_ptr() as CharBuf );

	lua_createtable( state, 0, 0 ); // Create a metatable to make the env inherit from _G
		lua_pushvalue( state, rglua::globals::Lua::GLOBALSINDEX );
		lua_setfield( state, -2, b"__index\0".as_ptr() as CharBuf );
	lua_setmetatable( state, -2 );

	lua_setfenv( state, -2 );

	if lua_pcall(state, 0, rglua::globals::Lua::MULTRET, 0) != 0 {
		let err_runtime = rstring!( lua_tolstring(state, -1, 0) );
		lua_settop(state, -2);
		return Err( err_runtime.to_owned() );
	}

	Ok(())
}