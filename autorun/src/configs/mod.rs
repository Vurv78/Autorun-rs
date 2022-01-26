// concat! only takes literals.
// autorun dir has been changed from sautorun-rs to ``autorun``
macro_rules! adir {
	() => {
		"autorun"
	};
}

pub static DUMP_DIR: &str = concat!(adir!(), "/lua_dumps");
#[cfg(feature = "logging")]
pub static LOG_DIR: &str = concat!(adir!(), "/logs");
pub static INCLUDE_DIR: &str = concat!(adir!(), "/scripts");

pub static AUTORUN_PATH: &str = concat!(adir!(), "/autorun.lua");
pub static HOOK_PATH: &str = concat!(adir!(), "/hook.lua");

pub fn path(path: &str) -> std::path::PathBuf {
	let home = home::home_dir().expect("Couldn't get your home directory!");
	home.join(path)
}
