use std::{
	path::PathBuf,
	fs::{self, File},
	sync::atomic::Ordering
};

use rglua::{
	lua_shared::*,
	types::LuaState,
	rstring
};

const LUA_OK: i32 = 0;

use crate::detours::LUAL_LOADBUFFERX_H;
use crate::sys::{
	statics::*
};

// Recursively creates folders based off of a directory from your HOME dir + the lua path made from the currently running file.
// &str garry_dir = Not necessarily a directory, can be anything, but this is the id returned by loadbuffer, loadstring, etc. Ex: "lua/init/bruh.lua"
// &str server_ip = The ip of the server. This will be used to create the folder structure of HOME/sautorun-rs/lua_dumps/IP/...
// Returns Option<File> that was created at the final dir.
pub fn getAutorunHandle(garry_dir: &str, server_ip: &str) -> Option<File> {
	if garry_dir.len() > 500 { return None };
	let mut lua_run_path = PathBuf::from(garry_dir);

	let extension = match lua_run_path.extension() {
		Some(ext) => {
			match ext.to_str() {
				Some(ext) if ext=="lua" => "lua", // Using guards check if the extension is lua, else it will fall under _.
				_ => "txt"
			}
		}
		None => "txt"
	};
	lua_run_path.set_extension(extension);

	let file_loc = &*SAUTORUN_DIR
		.join("lua_dumps")
		.join( sanitizePath(server_ip) )
		.join( sanitizePath(garry_dir) );

	match file_loc.parent() {
		Some(dirs) => {
			match fs::create_dir_all(dirs) {
				Err(why) => {
					error!("Couldn't create sautorun-rs dirs with path [{}]. [{}]", dirs.display(), why);
					None
				}
				Ok(_) => match File::create(file_loc) {
					Ok(file) => Some(file),
					Err(why) => {
						error!("Couldn't create sautorun-rs file with path [{}]. [{}]", why, file_loc.display());
						None
					}
				}
			}
		}
		None => None
	}
}

pub fn initMenuState(state: LuaState) -> Result<(), detour::Error> {
	use rglua::lua_shared::*;
	use std::sync::atomic::AtomicPtr;
	use crate::detours::{JOINSERVER_H, joinserver};

	if MENU_STATE.set( AtomicPtr::from(state) ).is_err() {
		error!("MENU_STATE was occupied in gmod13_open. Shouldn't happen.");
	}

	info!("Loaded into menu state.");

	lua_getglobal( state, "JoinServer\0".as_ptr() as *const i8 );
	let joinserver_fn = lua_tocfunction(state, -1);
	lua_pop(state, 1);

	unsafe {
		let hook = detour::GenericDetour::new(joinserver_fn, joinserver)?;
		JOINSERVER_H
			.set(hook).unwrap();
		JOINSERVER_H.get().unwrap().enable().unwrap();
	}

	Ok(())
}

// Creating this function for the future where accessing the lua state doesn't directly need an unsafe block.
pub fn getClientState() -> LuaState {
	CLIENT_STATE.load( Ordering::Acquire )
}

pub fn getMenuState() -> Option<LuaState> {
	Some( MENU_STATE.get()?.load(Ordering::Acquire) )
}

pub fn setClientState(state: LuaState) {
	CLIENT_STATE.store( state, Ordering::Release);
}

// https://github.com/parshap/node-sanitize-filename/blob/master/index.js

use regex::Regex;
use once_cell::sync::Lazy;

// ? < > \ : * | " minus /, since we want to allow directories here
static ILLEGAL: Lazy<Regex> = Lazy::new(|| Regex::new( r#"[\x{003F}\x{003C}\x{003E}\x{005C}\x{003A}\x{002A}\x{007C}\x{0022}]"#).unwrap() );
static CONTROL_RESERVED: Lazy<Regex> = Lazy::new(|| Regex::new( r"\x00-\x1f\x80-\x9f" ).unwrap() );
static RESERVED: Lazy<Regex> = Lazy::new(|| Regex::new( r"^\.+$" ).unwrap() );
static WINDOWS_RESERVED: Lazy<Regex> = Lazy::new(|| Regex::new( r"^(?i)(con|prn|aux|nul|com[0-9]|lpt[0-9])(\..*)?$" ).unwrap() );
static WINDOWS_TRAILING: Lazy<Regex> = Lazy::new(|| Regex::new( r"[\. ]+$" ).unwrap() );
static TRAVERSE: Lazy<Regex> = Lazy::new(|| Regex::new( r"\.{2,}\x{002F}?" ).unwrap() ); // We need this since we allow / (directories)

const REPL: &str = "_";

// Returns a string that is safe to use as a file path
pub fn sanitizePath<T: AsRef<str>>(input: T) -> String  {
	let path = ILLEGAL.replace_all(input.as_ref(), REPL);
	let path = CONTROL_RESERVED.replace_all(&path, REPL);
	let path = RESERVED.replace_all(&path, REPL);
	let path = WINDOWS_RESERVED.replace_all(&path, REPL);
	let path = WINDOWS_TRAILING.replace_all(&path, REPL);
	let path = TRAVERSE.replace_all(&path, REPL);

	path.into()
}


pub fn lua_compilestring(state: LuaState, code: &str) -> Result<(), &'static str> {
	if LUAL_LOADBUFFERX_H.call(
		state,
		code.as_ptr() as *const i8,
		code.len(),
		"@RunString\0".as_ptr() as *const i8,
		"bt\0".as_ptr() as *const i8
	) != LUA_OK {
		let err = lua_tolstring(state, -1, 0);
		lua_pop(state, 1);
		return Err( rstring!(err) );
	}

	Ok(())
}

pub fn lua_pexec(state: LuaState) -> Result<(), &'static str> {
	if lua_pcall( state, 0, -1, 0) != LUA_OK {
		let err = lua_tolstring(state, -1, 0);
		lua_pop(state, 1);
		return Err( rstring!(err) );
	}
	Ok(())
}

pub fn lua_dostring(state: LuaState, code: &str) -> Result<(), &'static str> {
	lua_compilestring(state, code)?;
	lua_pexec(state)?;
	Ok(())
}