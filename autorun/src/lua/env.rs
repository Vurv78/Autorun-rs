use std::ffi::CStr;
use colored::Colorize;
use rglua::prelude::*;

use crate::{configs, lua::{self, err}, logging::*};

#[lua_function]
pub fn log(l: LuaState) -> i32 {
	let s = luaL_checkstring(l, 1);
	let level = luaL_optinteger(l, 2, 3); // INFO by default

	let msg = unsafe { CStr::from_ptr(s).to_string_lossy() };
	match level {
		1 => error!("{msg}"),
		2 => warning!("{msg}"),
		3 => info!("{msg}"),
		4 => debug!("{msg}"),
		5 => trace!("{msg}"),
		_ => luaL_argerror(l, 2, err::INVALID_LOG_LEVEL)
	}

	0
}

#[lua_function]
// Works like MsgC in lua (except also adds a newline.)
pub fn print(l: LuaState) -> i32 {
	let nargs = lua_gettop(l);

	// Buffer for the whole message to be printed.
	let mut total_buf = String::new();

	// Buffer that is re-used for every color found
	let mut buf = String::new();

	// Walk through args backwards. Every color will affect all of the text prior.
	for i in (1..=nargs).rev() {
		match lua_type(l, i) {
			lua::TTABLE => {
				lua_rawgeti(l, i, 1);
				if lua_isnumber(l, -1) == 0 {
					// Not a color
					let s = lua_tostring(l, i);
					let s = unsafe { CStr::from_ptr(s).to_string_lossy() };
					buf.push_str(&s);

					lua_pop(l, 1);
					continue;
				}
				let r = luaL_optinteger(l, -1, 255) as u8;

				lua_rawgeti(l, i, 2);
				let g = luaL_optinteger(l, -1, 255) as u8;

				lua_rawgeti(l, i, 3);
				let b = luaL_optinteger(l, -1, 255) as u8;

				if !buf.is_empty() {
					let str = buf.truecolor(r, g, b);
					buf = String::new();

					total_buf.push_str( &format!("{str}") );
				}
			},
			_ => {
				let s = lua_tostring(l, i);
				let s = unsafe { CStr::from_ptr(s).to_string_lossy() };
				buf.push_str(&s);
			}
		}
	}

	println!("{total_buf}");

	0
}

// https://github.com/lua/lua/blob/eadd8c7178c79c814ecca9652973a9b9dd4cc71b/loadlib.c#L657
#[lua_function]
pub fn require(l: LuaState) -> i32 {
	use rglua::prelude::*;
	use std::{fs::File, io::prelude::*};

	let raw_path = luaL_checkstring(l, 1);
	let path = unsafe { CStr::from_ptr(raw_path) };
	let path = path.to_string_lossy();

	let path = configs::path(configs::INCLUDE_DIR)
		.join(path.to_string()); // I hate this to_string

	match File::open(&path) {
		Err(why) => error!(
			"Failed to sautorun.require path [{}] [{}]",
			path.display(),
			why
		),
		Ok(mut handle) => {
			let mut script = String::new();
			let top = lua_gettop(l);
			if let Err(why) = handle.read_to_string(&mut script) {
				error!(
					"Failed to read script from file [{}]. Reason: {}",
					path.display(),
					why
				);
			} else if let Err(why) = lua::dostring(l, &script) {
				error!("Error when requiring [{}]. [{}]", path.display(), why);
			}
			return lua_gettop(l) - top;
		}
	}

	0
}
