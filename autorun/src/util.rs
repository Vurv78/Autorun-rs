use crate::{configs, logging::*};
use std::fs::{self, File};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum HandleError {
	#[error("IO Error: `{0}`")]
	IO(#[from] std::io::Error),

	#[error("Location is too long!")]
	TooLong
}

/// Recursively creates folders based off of a directory from your HOME dir + the lua path made from the currently running file.
/// # Parameters
/// * `location` - Id returned by loadbuffer, loadstring, etc. Ex: "lua/init/foo.lua"
/// * `server_ip` - The ip of the server. This will be used to create the folder structure of HOME/autorun/lua_dumps/IP/...
/// # Returns
/// File created at the final dir.
pub fn get_handle<S: AsRef<str>>(location: &str, server_ip: S) -> Result<File, HandleError> {
	if location.len() >= 500 {
		return Err(HandleError::TooLong);
	};

	let server_ip = server_ip.as_ref();
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
		.join(strip_invalid(server_ip));

	fs::create_dir_all(file_loc)?;

	Ok( File::create( file_loc.join(location) )? )
}

// Removes basic invalid filename stuff
pub fn strip_invalid(str: &str) -> String {
	let mut pat = lua_patterns::LuaPattern::new(r#"[:<>"|?*]"#);
	pat.gsub(str, "_")
}