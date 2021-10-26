#![allow(non_snake_case)]
use std::ffi::CString;
use crate::{
	sys::{
		statics::*,
		util::{self, getClientState, getMenuState}
	}
};

use rglua::{globals::Lua::{self, GLOBALSINDEX}, lua_shared::*, rstring, types::LuaState};

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
			if s.is_null() { return Err("Client state has not been loaded. Join a server!"); }
			s
		}
	};

	match &mut LUA_SCRIPTS.try_lock() {
		Ok(script_queue) => script_queue.push( (realm, code) ),
		Err(why) => error!("Failed to lock lua_scripts mutex. {}", why)
	};

	Ok(())
}

extern "C" fn log(state: LuaState) -> i32 {
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

// https://github.com/lua/lua/blob/eadd8c7178c79c814ecca9652973a9b9dd4cc71b/loadlib.c#L657
extern "C" fn sautorun_require(state: LuaState) -> i32 {
	use std::path::PathBuf;

	let raw_path = luaL_checklstring(state, 1, 0);

	lua_getfield(state, rglua::globals::Lua::GLOBALSINDEX, "sautorun\0".as_ptr() as *const i8 );
	lua_getfield( state, rglua::globals::Lua::GLOBALSINDEX, "package\0".as_ptr() as *const i8 );
	lua_getfield( state, rglua::globals::Lua::GLOBALSINDEX, "loaded\0".as_ptr() as *const i8 );
	lua_getfield( state, 2, raw_path );
	if lua_toboolean(state, -1) != 0 {
		return 1;
	}
	lua_pop(state, 1);

	let path_str = util::sanitizePath( rstring!(raw_path) );
	let path = SAUTORUN_SCRIPT_DIR.join::<PathBuf>( path_str.into() );

	match std::fs::File::open(&path) {
		Err(why) => error!("Failed to sautorun.require path [{}] [{}]", path.display(), why),
		Ok(mut handle) => {
			use std::io::prelude::*;

			let mut script = String::new();
			if let Err(why) = handle.read_to_string(&mut script) {
				error!( "Failed to read script from file [{}]. Reason: {}", path.display(), why );
			} else if let Err(why) = util::lua_dostring(state, &script) {
				error!("Error when requiring [{}]. [{}]", path.display(), why);
			} else if lua_type(state, -1) == Lua::Type::Nil as i32 {
				println!("nil");
			}
		}
	}

	0
}

// Runs lua, but inside of the sautorun environment.
pub fn runLuaEnv(script: &str, identifier: *const i8, dumped_script: *const i8, ip: &str, startup: bool) -> Result<i32, String> {
	let state = getClientState();

	if state.is_null() {
		return Err( NO_LUA_STATE.to_owned() );
	}

	let top = lua_gettop(state);

	if let Err(why) = util::lua_compilestring(state, script) {
		return Err(why.to_owned());
	}

	lua_createtable( state, 0, 0 ); // local t = {}
	lua_createtable( state, 0, 0 ); // local t2 = {}

		lua_pushstring( state, identifier );
		lua_setfield( state, -2, "NAME\0".as_ptr() as *const i8 ); // t2.NAME = ...

		lua_pushstring( state, dumped_script );
		lua_setfield( state, -2, "CODE\0".as_ptr() as *const i8 ); // t2.CODE = ...

		if let Ok(ip) = CString::new(ip) {
			lua_pushstring( state, ip.as_ptr() );
		} else {
			lua_pushnil(state);
		}
		lua_setfield( state, -2, "IP\0".as_ptr() as *const i8 );

		// If this is running before autorun, set SAUTORUN.STARTUP to true.
		lua_pushboolean( state, startup as i32 );
		lua_setfield( state, -2, "STARTUP\0".as_ptr() as *const i8 );

		lua_pushcfunction( state, log );
		lua_setfield( state, -2, "log\0".as_ptr() as *const i8 );

		/*lua_createtable( state, 0, 0 ); // local t = {}
			lua_createtable( state, 0, 0 ); // local t2 = {}
			lua_setfield( state, -2, "loaded\0".as_ptr() as *const i8 ); // package.loaded = t2
		lua_setfield( state, -2, "package\0".as_ptr() as *const i8 ); // package = t
		*/

		lua_pushcfunction( state, sautorun_require );
		lua_setfield( state, -2, "require\0".as_ptr() as *const i8 );

	lua_setfield( state, -2, "sautorun\0".as_ptr() as *const i8 );

	lua_createtable( state, 0, 0 ); // Create a metatable to make the env inherit from _G
		lua_pushvalue( state, GLOBALSINDEX );
		lua_setfield( state, -2, "__index\0".as_ptr() as *const i8 );
	lua_setmetatable( state, -2 );

	lua_setfenv( state, -2 );

	if let Err(why) = util::lua_pexec(state) {
		return Err(why.to_owned());
	}
	Ok(top)
}