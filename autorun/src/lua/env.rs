use std::ffi::CStr;
use rglua::prelude::*;

use crate::{configs, lua::{self, err}, logging::*};

#[lua_function]
pub fn log(l: LuaState) -> i32 {
	let s = luaL_checkstring(l, 1);
	let level = luaL_optinteger(l, 2, 3); // INFO by default

	let str = try_rstr!(s).unwrap_or("UTF8 - Conversion Error");
	match level {
		1 => error!("{}", str),
		2 => warn!("{}", str),
		3 => info!("{}", str),
		4 => debug!("{}", str),
		5 => trace!("{}", str),
		_ => {
			luaL_argerror(l, 2, err::INVALID_LOG_LEVEL);
		}
	}
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
