use crate::{configs, logging::*};
use std::fs::{self, File};

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
pub fn get_handle<S: AsRef<str>>(location: &str, fmt: S) -> Result<File, HandleError> {
	if location.len() >= 500 {
		return Err(HandleError::TooLong);
	};

	let fmt = fmt.as_ref();
	let file_loc = &*configs::path(configs::DUMP_DIR)
		.join(strip_invalid(fmt));

	let mut path = file_loc.join(location);

	let dir = path.parent().unwrap_or(file_loc);
	fs::create_dir_all(&dir)?;

	path.set_extension("lua");

	Ok( File::create( path )? )
}

// Removes basic invalid filename stuff
pub fn strip_invalid(str: &str) -> String {
	let mut pat = lua_patterns::LuaPattern::new(r#"[:<>"|?*]"#);
	pat.gsub(str, "_")
}