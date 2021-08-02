use std::{
	path::PathBuf,
	fs::{self, File},
	sync::atomic::Ordering
};


use rglua::types::LuaState;

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
		.join(server_ip.replace(":","."))
		.join(&garry_dir.to_owned().replace(":", "."));

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
	use crate::detours::{joinserver_h, joinserver};

	MENU_STATE
		.set( AtomicPtr::from(state) )
		.expect("MENU_STATE was occupied in gmod13_open. Shouldn't happen.");

	info!("Init menu state");

	lua_getglobal( state, "JoinServer\0".as_ptr() as *const i8 );
	let joinserver_fn = lua_tocfunction(state, -1);
	lua_pop(state, 1);

	unsafe {
		joinserver_h
			.initialize(joinserver_fn, joinserver)?
			.enable()?;
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