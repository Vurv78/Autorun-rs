use colored::Colorize;
use once_cell::sync::Lazy;
use rglua::prelude::*;
use std::{
	ffi::CStr,
	sync::{Arc, Mutex}, mem::MaybeUninit, path::PathBuf,
};

use crate::{
	fs::{self as afs, INCLUDE_DIR, BIN_DIR, FSPath},
	logging::*,
	lua::{self, err},
};

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
		_ => luaL_argerror(l, 2, err::INVALID_LOG_LEVEL),
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
	let mut color: Option<(u8, u8, u8)> = None;
	for i in 1..=nargs {
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

				if let Some(col) = color {
					// Take all previous text
					let str = buf.truecolor(col.0, col.1, col.2);
					buf = String::new();
					total_buf.push_str(&format!("{str}"));
				}

				color = Some( (r, g, b) );
			},
			lua::TFUNCTION | lua::TUSERDATA | lua::TLIGHTUSERDATA | lua::TTHREAD => {
				let s = lua_topointer(l, i);
				buf.push_str( &format!("{:p}", s) );
			},
			_ => {
				let s = lua_tostring(l, i);
				let s = unsafe { CStr::from_ptr(s).to_string_lossy() };
				buf.push_str(&s);
			}
		}
	}

	if let Some(col) = color {
		let str = buf.truecolor(col.0, col.1, col.2);
		total_buf.push_str(&format!("{str}"));
	}

	println!("{total_buf}");

	0
}

#[derive(Debug, thiserror::Error)]
enum RequireError {
	#[error("Failed to require file: {0}")]
	IO(#[from] std::io::Error),

	#[error("Failed to load dynamic library: {0}")]
	Libloading(#[from] libloading::Error),

	#[error("Failed to find gmod13_open or autorun_open symbols in library")]
	SymbolNotFound,

	#[error("File does not exist: {0}")]
	DoesNotExist(String),
}

// Gets function at the stack level given (assuming there is one there..)
fn get_func(l: LuaState, level: u32) {
	let mut ar = MaybeUninit::uninit();

	if lua_getstack(l, level as i32, ar.as_mut_ptr()) == 0 {
		luaL_argerror(l, 1, cstr!("invalid level"));
	}

	lua_getinfo(l, cstr!("f"), ar.as_mut_ptr());

	if lua_isnil(l, -1) {
		luaL_error(l, cstr!("no function environment for tail call at level %d"), level);
	}
}

// https://github.com/lua/lua/blob/eadd8c7178c79c814ecca9652973a9b9dd4cc71b/loadlib.c#L657
#[lua_function]
pub fn require(l: LuaState) -> Result<i32, RequireError> {
	use rglua::prelude::*;

	let raw_path = luaL_checkstring(l, 1);
	let path_name = unsafe { CStr::from_ptr(raw_path) };
	let path_name = path_name.to_string_lossy();

	let mut path = PathBuf::from( path_name.as_ref() );

	if path.file_name().is_none() {
		luaL_error(l, cstr!("Malformed require path: '%s'"), raw_path);
	}

	// Make sure extension is always .lua
	match path.extension() {
		Some(ext) if ext == "lua" => (),
		Some(_) => {
			luaL_error(l, cstr!("Malformed require path: '%s' (needs .lua file extension)"), raw_path);
		},
		_ => {
			path.set_extension("lua");
		}
	}

	let script;
	get_func(l, 1); // Get the function calling this to get the fenv
	if lua_iscfunction(l, -1) == 0 {
		// Not a C function. Can get the fenv.
		lua_getfenv(l, -1);

		lua_getfield(l, -1, cstr!("Autorun"));
		if lua_istable(l, -1) {
			lua_getfield(l, -1, cstr!("PATH"));
			if lua_isstring(l, -1) == 1 {
				let file_path = lua_tostring(l, -1);
				let file_path = unsafe { CStr::from_ptr(file_path) };
				let file_path = file_path.to_string_lossy();

				let local_path = FSPath::from( file_path.as_ref() );
				let local_path = local_path
					.parent()
					.unwrap_or(local_path) // pop to directory atop running lua file, e.g. to /src/. Using unwrap_or to avoid panic (just in case)
					.join(path);

				if local_path.exists() {
					script = afs::read_to_string(&local_path)?;
				} else {
					let local_path = FSPath::from(INCLUDE_DIR)
						.join(path_name.as_ref());
					script = afs::read_to_string(&local_path)?;
				}

				lua_pop(l, 3); // pop PATH, Autorun and fenv
			} else {
				lua_pop(l, 1);
				luaL_error(l, cstr!("Bad require: Autorun.PATH is not a string"));
			}
		} else {
			lua_pop(l, 1);
			luaL_error(l, cstr!("Bad require: Autorun table not found"));
		}

		lua_pop(l, 1); // Pop the function
	} else {
		luaL_error(l, cstr!("Cannot use `require` in a C function"));
	}

	let top = lua_gettop(l);

	if let Err(why) = lua::compile(l, &script) {
		let err = format!("Compile error when requiring file {path_name}: {why}\0");
		let err_c = err.as_bytes();

		luaL_error(l, err_c.as_ptr() as *const _);
	}

	get_func(l, 1);
	if lua_iscfunction(l, -1) == 0 {
		lua_getfenv(l, -1);
		lua_remove(l, -2);

		lua_setfenv(l, -2);
	}

	if let Err(why) = lua::pcall(l) {
		let err = format!("Error when requiring file {path_name}: {why}\0");
		let err_c = err.as_bytes();

		luaL_error(l, err_c.as_ptr() as *const _);
	}

	Ok(lua_gettop(l) - top)
}

pub static LOADED_LIBS: Lazy<Arc<Mutex<Vec<libloading::Library>>>> =
	Lazy::new(|| Arc::new(Mutex::new(vec![])));

#[lua_function]
/// Example usage: require("vistrace") (No extensions or anything.)
pub fn requirebin(l: LuaState) -> Result<i32, RequireError> {
	use std::env::consts::{DLL_EXTENSION, DLL_PREFIX};

	let dlname = luaL_checkstring(l, 1);
	let dlname = unsafe { CStr::from_ptr(dlname) };
	let dlname = dlname.to_string_lossy();

	let binpath = afs::in_autorun(BIN_DIR);
	let mut path = binpath
		.join(DLL_PREFIX)
		.join(dlname.as_ref())
		.with_extension( DLL_EXTENSION );

	if !path.exists() {
		let os_prefix = if cfg!(windows) {
			"win"
		} else if cfg!(target_os = "macos") {
			"osx"
		} else {
			"linux"
		};

		let arch = if cfg!(target_pointer_width = "32") {
			"32"
		} else {
			"64"
		};

		let altpath = binpath.join(format!("gmcl_{dlname}_{os_prefix}{arch}.{DLL_EXTENSION}"));

		if altpath.exists() {
			path = altpath;
		} else {
			let altpath = binpath.join(format!("gmsv_{dlname}_{os_prefix}{arch}.{DLL_EXTENSION}"));
			if altpath.exists() {
				path = altpath;
			} else {
				return Err(RequireError::DoesNotExist(path.display().to_string()));
			}
		}
	}

	let lib = unsafe { libloading::Library::new(path)? };

	// Api may be changed.
	type AutorunEntry = extern "C" fn(l: LuaState) -> c_int;
	type Gmod13Entry = extern "C" fn(l: LuaState) -> c_int;
	type LuaEntry = extern "C" fn(l: LuaState) -> c_int;

	let n_symbols;
	if let Ok(autorun_sym) = unsafe { lib.get::<AutorunEntry>(b"autorun_open\0") } {
		n_symbols = autorun_sym(l);
	} else if let Ok(gmod13_sym) = unsafe { lib.get::<Gmod13Entry>(b"gmod13_open\0") } {
		n_symbols = gmod13_sym(l);
	} else {
		let lua_sym = format!("luaopen_{}\0", dlname);
		if let Ok(lua_sym) = unsafe { lib.get::<LuaEntry>(lua_sym.as_bytes()) } {
			n_symbols = lua_sym(l);
		} else {
			return Err(RequireError::SymbolNotFound);
		}
	}

	if let Ok(mut libs) = LOADED_LIBS.try_lock() {
		libs.push(lib);
	}

	Ok(n_symbols)
}