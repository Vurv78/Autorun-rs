use rglua::prelude::*;

use crate::fs::{self as afs, FSPath, PLUGIN_DIR};
use crate::lua::{self, AutorunEnv, LuaEnvError};
use crate::{logging::*, ui::printcol};
use fs_err as fs;

use std::borrow::Cow;
use std::ffi::{CString, OsStr};
use std::path::Path;

mod serde;
pub use self::serde::{PluginToml, PluginMetadata};

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
	#[error("IO Error: {0}")]
	IO(#[from] std::io::Error),

	#[error("{0}")]
	LuaEnv(#[from] LuaEnvError),

	#[error("Failed to parse plugin.toml: {0}")]
	Parsing(toml::de::Error),

	#[error("Could not find plugin.toml")]
	NoToml,
}

#[non_exhaustive]
pub enum PluginLanguage {
	// In the future there could be more languages like Teal or Expressive
	Lua,
}

impl Default for PluginLanguage {
	fn default() -> Self {
		Self::Lua
	}
}

#[derive(Debug)]
pub struct Plugin {
	data: serde::PluginToml,

	/// Path to plugin's directory local to autorun directory
	dir: FSPath,
}

// Result of running hook.lua
#[derive(PartialEq)]
pub enum HookRet {
	Stop,
	/// Replace running code
	Replace(LuaString, usize),
	Continue
}

impl Plugin {
	pub fn get_name(&self) -> &String {
		&self.data.plugin.name
	}

	pub fn get_author(&self) -> &String {
		&self.data.plugin.author
	}

	pub fn get_version(&self) -> &String {
		&self.data.plugin.version
	}

	pub fn get_description(&self) -> &Option<String> {
		&self.data.plugin.description
	}

	pub fn get_settings(&self) -> &toml::Value {
		&self.data.settings
	}

	pub fn get_dir(&self) -> &FSPath {
		&self.dir
	}

	pub fn has_file<N: AsRef<str>>(&self, name: N) -> bool {
		let name = name.as_ref();
		let path = self.dir.join(name);
		path.exists()
	}

	fn push_settings(&self, l: LuaState) {
		match self.get_settings().as_table() {
			Some(tbl) => {
				lua_createtable(l, 0, tbl.len() as i32);

				fn push_value(l: LuaState, v: &toml::Value) {
					match v {
						toml::Value::String(s) => {
							let bytes = s.as_bytes();
							lua_pushlstring(l, bytes.as_ptr().cast(), bytes.len());
						}
						toml::Value::Integer(n) => lua_pushinteger(l, *n as LuaInteger),
						toml::Value::Boolean(b) => lua_pushboolean(l, *b as i32),

						toml::Value::Float(f) => lua_pushnumber(l, *f),

						toml::Value::Array(arr) => {
							lua_createtable(l, arr.len() as i32, 0);

							for (i, v) in arr.iter().enumerate() {
								push_value(l, v);
								lua_rawseti(l, -2, i as i32 + 1);
							}
						}

						toml::Value::Table(tbl) => {
							lua_createtable(l, 0, tbl.len() as i32);

							for (k, v) in tbl.iter() {
								if let Ok(k) = CString::new(k.as_bytes()) {
									push_value(l, v);
									lua_setfield(l, -2, k.as_ptr());
								}
							}
						}

						toml::Value::Datetime(time) => {
							// Just pass a string, smh
							let time = time.to_string();
							let bytes = time.as_bytes();
							lua_pushlstring(l, bytes.as_ptr() as _, bytes.len());
						}
					}
				}

				for (k, v) in tbl.iter() {
					let k = match CString::new(k.as_bytes()) {
						Ok(k) => k,
						Err(_) => continue,
					};

					push_value(l, v);
					lua_setfield(l, -2, k.as_ptr());
				}
			}
			None => lua_createtable(l, 0, 0),
		}
	}

	pub fn run_lua<S: AsRef<str>, P: AsRef<Path>>(
		&self,
		l: LuaState,
		src: S,
		path: P,
		env: &AutorunEnv,
	) -> Result<i32, LuaEnvError> {
		lua::run_env_prep(
			l,
			src,
			path,
			env,
			&Some(|l| {
				lua_createtable(l, 0, 4);

				let name = self.get_name();
				if let Ok(name) = CString::new(name.as_bytes()) {
					lua_pushstring(l, name.as_ptr());
					lua_setfield(l, -2, cstr!("NAME"));
				}

				let version = self.get_version();
				if let Ok(version) = CString::new(version.as_bytes()) {
					lua_pushstring(l, version.as_ptr());
					lua_setfield(l, -2, cstr!("VERSION"));
				}

				let author = self.get_author();
				if let Ok(author) = CString::new(author.as_bytes()) {
					lua_pushstring(l, author.as_ptr());
					lua_setfield(l, -2, cstr!("AUTHOR"));
				}

				let dir = self.get_dir();
				let dirname = dir.file_name();
				if let Some(d) = dirname {
					let d = d.to_string_lossy();
					let bytes = d.as_bytes();
					lua_pushlstring(l, bytes.as_ptr() as *mut _, bytes.len());
					lua_setfield(l, -2, cstr!("DIR"));
				}

				if let Some(desc) = self.get_description() {
					if let Ok(desc) = CString::new(desc.as_bytes()) {
						lua_pushstring(l, desc.as_ptr());
						lua_setfield(l, -2, cstr!("DESCRIPTION"));
					}
				}

				self.push_settings(l);
				lua_setfield(l, -2, cstr!("Settings"));

				lua_setfield(l, -2, cstr!("Plugin"));
			}),
		)
	}

	/// dofile but if the ran code returns a boolean or string, will return that to Rust.
	pub fn dohook(&self, l: LuaState, env: &AutorunEnv) -> Result<HookRet, PluginError> {
		let path = self.dir.join("src/hook.lua");
		let src = afs::read_to_string(&path)?;
		let top = self.run_lua(l, &src, &path, env)?;

		let ret = match lua_type(l, top + 1) {
			rglua::lua::TBOOLEAN => {
				if lua_toboolean(l, -1) != 0 {
					Ok(HookRet::Stop)
				} else {
					Ok(HookRet::Continue)
				}
			},

			rglua::lua::TSTRING => {
				let mut len: usize = 0;
				let code = lua_tolstring(l, top + 1, &mut len);
				Ok( HookRet::Replace(code, len) )
			}

			_ => Ok( HookRet::Continue )
		};

		lua_settop(l, top);

		ret
	}

	pub fn dofile<P: AsRef<Path>>(
		&self,
		l: LuaState,
		path: P,
		env: &AutorunEnv,
	) -> Result<(), PluginError> {
		let path = self.dir.join(path);
		let src = afs::read_to_string(&path)?;
		self.run_lua(l, &src, &path, env)?;
		Ok(())
	}
}

/// Searches for plugin and makes sure they are all valid, if not, prints errors to the user.
pub fn sanity_check() -> Result<(), PluginError> {
	afs::traverse_dir(PLUGIN_DIR, |path, _| {
		if path.is_dir() {
			let plugin_toml = path.join("plugin.toml");

			let src_autorun = path.join("src/autorun.lua");
			let src_hooks = path.join("src/hook.lua");

			let path_name = path
				.file_name()
				.map_or_else(|| Cow::Owned(path.display().to_string()), OsStr::to_string_lossy);

			if plugin_toml.exists() && (src_autorun.exists() || src_hooks.exists()) {
				if let Ok(content) = afs::read_to_string(plugin_toml) {
					match toml::from_str::<serde::PluginToml>(&content) {
						Ok(_) => (),
						Err(why) => error!(
							"Failed to load plugin {}. plugin.toml failed to parse: '{}'",
							path_name, why
						),
					}
				} else {
					error!(
						"Failed to load plugin {}. plugin.toml could not be read.",
						path_name
					);
				}
			} else if plugin_toml.exists() {
				error!("Failed to load plugin {}. plugin.toml exists but no src/autorun.lua or src/hook.lua", path_name);
			} else {
				error!(
					"Failed to load plugin {}. plugin.toml does not exist",
					path_name
				);
			}
		}
	})?;

	Ok(())
}

// (Directory, PluginOrError)
type PluginFS = (String, Result<Plugin, PluginError>);

/// Finds all valid plugins (has plugin.toml, src/autorun.lua or src/hook.lua)
pub fn find() -> Result<Vec<PluginFS>, PluginError> {
	let mut plugins = vec![];

	afs::traverse_dir(PLUGIN_DIR, |path, _| {
		let path_name = path
			.file_name()
			.map_or_else(|| path.display().to_string(), |x| x.to_string_lossy().to_string());

		if path.is_dir() {
			let plugin_toml = path.join("plugin.toml");
			let res = if plugin_toml.exists() {
				if let Ok(content) = afs::read_to_string(plugin_toml) {
					match toml::from_str::<serde::PluginToml>(&content) {
						Ok(toml) => Ok(Plugin {
							data: toml,
							dir: path.to_owned(),
						}),
						Err(why) => Err(PluginError::Parsing(why)),
					}
				} else {
					Err(PluginError::NoToml)
				}
			} else {
				Err(PluginError::NoToml)
			};

			plugins.push((path_name, res));
		}
	})?;

	Ok(plugins)
}

/// Run ``autorun.lua`` in all plugins.
pub fn call_autorun(l: LuaState, env: &AutorunEnv) -> Result<(), PluginError> {
	for (dirname, plugin) in find()? {
		match plugin {
			Ok(plugin) => {
				if plugin.has_file("src/autorun.lua") {
					if let Err(why) = plugin.dofile(l, "src/autorun.lua", env) {
						error!("Error in plugin '{}': [{}]", plugin.get_name(), why);
					};
				}
			}
			Err(why) => {
				error!("Failed to load plugin @plugins/{dirname}: {}", why);
			}
		}
	}

	Ok(())
}

/// Run ``hook.lua`` in all plugins.
/// Does not print out any errors unlike `call_autorun`.
pub fn call_hook(l: LuaState, env: &AutorunEnv, do_run: &mut bool) -> Result<Option<(LuaString, usize)>, PluginError> {
	for plugin in find()? {
		if let (_, Ok(plugin)) = plugin {
			// All of the plugin hook.lua will still run even if the first plugin returned a string or a boolean.
			// They will however have their return values ignored.
			if let Ok(plugin_ret) = plugin.dohook(l, env) {
				match plugin_ret {
					HookRet::Continue => (),
					HookRet::Replace(code, len) => {
						// Code to edit script and have other plugins ``hook.lua`` filess still run
						// Not sure if this should be the behavior so just having it abort.
						// (code, code_len) = (loc_code, loc_len);
						// env.set_code(code, code_len);

						return Ok( Some( (code, len) ) );

					},
					HookRet::Stop => {
						*do_run = false;
					}
				}
			}
		}
	}
	Ok(None)
}

pub fn init() -> Result<(), PluginError> {
	let plugin_dir = afs::in_autorun(PLUGIN_DIR);
	if !plugin_dir.exists() {
		fs::create_dir(&plugin_dir)?;
	}

	sanity_check()?;

	let plugins = find()?;

	printcol!(WHITE, "Verifying plugins..");
	if plugins.is_empty() {
		printcol!(WHITE, on_green, "{}", "No plugins found!");
	}

	for plugin in plugins {
		match plugin {
			(name, Err(why)) => error!("Failed to verify plugin @plugins/{name}: {}", why),
			(_, Ok(plugin)) => info!("Verified plugin: {}", plugin.get_name()),
		}
	}

	Ok(())
}
