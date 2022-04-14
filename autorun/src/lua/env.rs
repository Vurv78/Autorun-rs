use colored::Colorize;
use once_cell::sync::Lazy;
use rglua::prelude::*;
use std::{
	ffi::CStr,
	mem::MaybeUninit,
	path::PathBuf,
	sync::{Arc, Mutex},
};

use crate::{
	fs::{self as afs, FSPath, BIN_DIR, INCLUDE_DIR},
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

				color = Some((r, g, b));
			}
			lua::TFUNCTION | lua::TUSERDATA | lua::TLIGHTUSERDATA | lua::TTHREAD => {
				let s = lua_topointer(l, i);
				buf.push_str(&format!("{:p}", s));
			}
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
		luaL_error(
			l,
			cstr!("no function environment for tail call at level %d"),
			level,
		);
	}
}

// Pushes the fenv onto the stack
fn push_fenv(l: LuaState) -> bool {
	get_func(l, 1);

	if lua_iscfunction(l, -1) == 0 {
		lua_getfenv(l, -1);

		return true;
	}
	lua_pop(l, 1); // pop func

	false
}

fn get_current_path(l: LuaState) -> Option<FSPath> {
	if push_fenv(l) {
		lua_getfield(l, -1, cstr!("Autorun"));

		if lua_istable(l, -1) {
			lua_getfield(l, -1, cstr!("PATH"));
			if lua_isstring(l, -1) == 1 {
				let file_path = lua_tostring(l, -1);
				let file_path = unsafe { CStr::from_ptr(file_path) };
				let file_path = file_path.to_string_lossy();

				lua_pop(l, 3); // pop PATH, Autorun and fenv
				return Some(FSPath::from(file_path.to_string()));
			} else {
				luaL_error(l, cstr!("Bad call: Autorun.PATH is not a string"));
			}
		} else {
			luaL_error(l, cstr!("Bad call: Autorun table not found"));
		}
	}

	None
}

fn get_relative<P: AsRef<std::path::Path>>(l: LuaState, path: P) -> Option<FSPath> {
	let p = path.as_ref();

	let current = get_current_path(l)?;
	Some(current.parent().unwrap_or(current).join(p))
}

// https://github.com/lua/lua/blob/eadd8c7178c79c814ecca9652973a9b9dd4cc71b/loadlib.c#L657
#[lua_function]
pub fn require(l: LuaState) -> Result<i32, RequireError> {
	use rglua::prelude::*;

	let raw_path = luaL_checkstring(l, 1);
	let path_name = unsafe { CStr::from_ptr(raw_path) };
	let path_name = path_name.to_string_lossy();

	let mut path = PathBuf::from(path_name.as_ref());

	if path.file_name().is_none() {
		luaL_error(l, cstr!("Malformed require path: '%s'"), raw_path);
	}

	// Make sure extension is always .lua or omitted
	match path.extension() {
		Some(ext) if ext == "lua" => (),
		Some(_) => {
			luaL_error(
				l,
				cstr!("Malformed require path: '%s' (needs .lua file extension)"),
				raw_path,
			);
		}
		None => {
			path.set_extension("lua");
		}
	}

	let path = get_relative(l, &path).unwrap_or_else(|| FSPath::from(INCLUDE_DIR).join(&path));

	if !path.exists() {
		luaL_error(
			l,
			cstr!("File does not exist in autorun/scripts or relative: '%s'"),
			raw_path,
		);
	}

	let script = afs::read_to_string(path)?;
	let top = lua_gettop(l);

	if let Err(why) = lua::compile(l, &script) {
		let err = format!("Compile error when requiring file {path_name}: {why}\0");
		let err_c = err.as_bytes();

		luaL_error(l, err_c.as_ptr().cast());
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

		luaL_error(l, err_c.as_ptr().cast());
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
		.with_extension(DLL_EXTENSION);

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
	} else if let Ok(lua_sym) = unsafe { lib.get::<LuaEntry>(b"lua_open\0") } {
		n_symbols = lua_sym(l);
	} else {
		return Err(RequireError::SymbolNotFound);
	}

	if let Ok(mut libs) = LOADED_LIBS.try_lock() {
		libs.push(lib);
	}

	Ok(n_symbols)
}

#[derive(Debug, thiserror::Error)]
enum ReadError {
	#[error("Cannot be called inside a C function")]
	CFunction,

	#[error("Failed to read file: {0}")]
	IO(#[from] std::io::Error),
}

#[lua_function]
pub fn read(l: LuaState) -> Result<i32, ReadError> {
	let path_name_raw = luaL_checkstring(l, 1);
	let path_name = unsafe { CStr::from_ptr(path_name_raw) };
	let path = path_name.to_string_lossy();
	let path = std::path::Path::new(path.as_ref());

	if path.extension().is_none() {
		luaL_error(
			l,
			cstr!("Malformed file name: %s (Missing extension)\0"),
			path_name_raw,
		);
	}

	if push_fenv(l) {
		lua_getfield(l, -1, cstr!("Autorun"));
		if lua_istable(l, -1) {
			lua_getfield(l, -1, cstr!("PATH"));

			let current_path = luaL_checkstring(l, -1);
			let current_path = unsafe { CStr::from_ptr(current_path) };
			let current_path = current_path.to_string_lossy();
			let mut current_path = FSPath::from(current_path.to_string());
			current_path.pop(); // Pop to current directory instead of file

			lua_pop(l, 1);

			// First try to retrieve local to current file.
			let mut total_path = current_path.join(path);
			if !total_path.exists() {
				lua_getfield(l, -1, cstr!("Plugin"));

				if lua_istable(l, -1) {
					// It's a plugin
					lua_getfield(l, -1, cstr!("DIR"));
					let plugin_dir = luaL_checkstring(l, -1);
					let plugin_dir = unsafe { CStr::from_ptr(plugin_dir) };
					let plugin_dir = plugin_dir.to_string_lossy();

					let data_path = afs::FSPath::from(afs::PLUGIN_DIR)
						.join(plugin_dir.to_string())
						.join("data")
						.join(path);

					if data_path.exists() {
						total_path = data_path;
					} else {
						luaL_error(l, cstr!("File not found: %s"), path_name_raw);
					}
				} else {
					lua_pop(l, 1);
				}
			}

			let contents = afs::read_to_string(total_path)?;
			let contents_bytes = contents.as_bytes();
			lua_pushlstring(l, contents_bytes.as_ptr() as *mut _, contents.len());
			Ok(1)
		} else {
			luaL_error(l, cstr!("Bad call: Autorun table not found"));
		}
	} else {
		Err(ReadError::CFunction)
	}
}
