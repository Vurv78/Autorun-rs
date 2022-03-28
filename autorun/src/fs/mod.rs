use std::path::{Path, PathBuf};

use fs_err as fs;

pub const DUMP_DIR: &str = "lua_dumps";
pub const LOG_DIR: &str = "logs";
pub const INCLUDE_DIR: &str = "scripts";
pub const PLUGIN_DIR: &str = "plugins";
pub const BIN_DIR: &str = "bin";

pub const AUTORUN_PATH: &str = "autorun.lua";
pub const HOOK_PATH: &str = "hook.lua";
pub const SETTINGS_PATH: &str = "settings.toml";

mod path;
pub use path::FSPath;

pub fn in_autorun<S: AsRef<Path>>(path: S) -> PathBuf {
	home::home_dir()
		.expect("Couldn't get your home directory!")
		.join("autorun")
		.join(path.as_ref())
}

pub fn base() -> PathBuf {
	home::home_dir()
		.expect("Couldn't get your home directory!")
		.join("autorun")
}

pub fn read_to_string<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
	use std::io::Read;

	let mut file = fs::File::open(in_autorun(path.as_ref()))?;
	let mut contents = String::new();
	file.read_to_string(&mut contents)?;

	Ok(contents)
}

// Reads a directory at a path local to the 'autorun' directory,
// And then returns results *also* truncated to be local to the 'autorun' directory
pub fn traverse_dir<P: AsRef<Path>, F: FnMut(&FSPath, fs::DirEntry)>(
	path: P,
	mut rt: F,
) -> std::io::Result<()> {
	let p = in_autorun(path.as_ref());
	let ar_base = base();

	for entry in fs::read_dir(&p)?.flatten() {
		let path = entry.path();
		let path = path.strip_prefix(&ar_base).unwrap_or(&path);

		rt(&FSPath::from(path), entry);
	}

	Ok(())
}
