use crate::{configs, global, logging::*};
use std::fs::{self, File};
use std::path::PathBuf;

use rglua::types::LuaState;

use atomic::Ordering;

#[inline(always)]
pub fn get_menu() -> Option<LuaState> {
	Some(global::MENU_STATE.get()?.load(Ordering::Acquire))
}

#[inline(always)]
pub fn get_client() -> LuaState {
	global::CLIENT_STATE.load(Ordering::Acquire)
}

#[inline(always)]
pub fn set_client(state: LuaState) {
	global::CLIENT_STATE.store(state, Ordering::Release)
}

/// Recursively creates folders based off of a directory from your HOME dir + the lua path made from the currently running file.
/// # Parameters
/// * `location` - Id returned by loadbuffer, loadstring, etc. Ex: "lua/init/foo.lua"
/// * `server_ip` - The ip of the server. This will be used to create the folder structure of HOME/sautorun-rs/lua_dumps/IP/...
/// # Returns
/// File created at the final dir.
pub fn get_handle(location: &str, server_ip: &str) -> Option<File> {
	if location.len() > 500 {
		return None;
	};

	let mut lua_run_path = PathBuf::from(location);

	let extension = match lua_run_path.extension() {
		Some(ext) => {
			match ext.to_str() {
				Some(ext) if ext == "lua" => "lua", // Using guards check if the extension is lua, else it will fall under _.
				_ => "txt",
			}
		}
		None => "txt",
	};
	lua_run_path.set_extension(extension);

	let file_loc = &*configs::path(configs::DUMP_DIR)
		.join(strip_invalid(server_ip))
		.join(location);

	match file_loc.parent() {
		Some(dirs) => match fs::create_dir_all(dirs) {
			Err(why) => {
				error!(
					"Couldn't create sautorun-rs dirs with path [{}]. [{}]",
					dirs.display(),
					why
				);
				None
			}
			Ok(_) => match File::create(file_loc) {
				Ok(file) => Some(file),
				Err(why) => {
					error!(
						"Couldn't create sautorun-rs file with path [{}]. [{}]",
						why,
						file_loc.display()
					);
					None
				}
			},
		},
		None => None,
	}
}

// Removes basic invalid filename stuff
pub fn strip_invalid(str: &str) -> String {
	let mut pat = lua_patterns::LuaPattern::new(r#"[:<>"|?*]"#);
	pat.gsub(str, "_")
}