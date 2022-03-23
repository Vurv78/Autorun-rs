use rglua::types::LuaState;

use crate::lua::{self, AutorunEnv, LuaEnvError};
use crate::{logging::*, configs, ui::printcol};
use crate::configs::PLUGIN_DIR;

use std::path::PathBuf;

mod serde;

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
	#[error("IO Error: {0}")]
	IO(#[from] std::io::Error),

	#[error("Lua env error: {0}")]
	LuaEnv(#[from] LuaEnvError),

	#[error("Failed to parse plugin.toml: {0}")]
	Parsing(toml::de::Error),

	#[error("Could not find plugin.toml")]
	NoToml
}

#[non_exhaustive]
pub enum PluginLanguage {
	// In the future there could be more languages like Teal or Expressive
	Lua
}

impl Default for PluginLanguage {
	fn default() -> Self { Self::Lua }
}

#[derive(Debug)]
pub struct Plugin {
	data: serde::PluginToml,
	/// Path to plugin's directory
	dir: PathBuf,
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

	// Will use later in getting Autorun.require to work relative.
	#[allow(unused)]
	pub fn get_path(&self) -> &PathBuf {
		&self.dir
	}

	pub fn has_file<N: AsRef<str>>(&self, name: N) -> bool {
		let name = name.as_ref();
		let path = self.dir.join(name);
		path.exists()
	}

	pub fn dofile<N: AsRef<str>>(&self, l: LuaState, name: N, env: &AutorunEnv) -> Result<(), PluginError> {
		let src = std::fs::read_to_string( self.dir.join(name.as_ref()) )?;

		lua::run_plugin(l, &src, env, self)?;

		Ok(())
	}
}

/// Searches for plugin and makes sure they are all valid, if not, prints errors to the user.
pub fn sanity_check() -> Result<(), PluginError> {
	let dir = std::fs::read_dir( configs::path(PLUGIN_DIR) )?;
	for d in dir {
		if let Ok(d) = d {
			let path = d.path();
			if path.is_dir() {
				let plugin_toml = path.join("plugin.toml");

				let src_autorun = path.join("src/autorun.lua");
				let src_hooks = path.join("src/hook.lua");

				let path_name = path.file_name()
					.map(|x| x.to_string_lossy())
					.unwrap_or_else(|| std::borrow::Cow::Owned( path.display().to_string() ) );

				if plugin_toml.exists() && (src_autorun.exists() || src_hooks.exists()) {
					let content = std::fs::read_to_string(plugin_toml)?;
					match toml::from_str::<serde::PluginToml>(&content) {
						Ok(_) => (),
						Err(why) => error!("Failed to load plugin {}. plugin.toml failed to parse: '{}'", path_name, why)
					}
				} else if plugin_toml.exists() {
					error!("Failed to load plugin {}. plugin.toml exists but no src/autorun.lua or src/hook.lua", path_name);
				} else {
					error!("Failed to load plugin {}. plugin.toml does not exist", path_name);
				}
			}
		} else {
			error!("autorun/plugins folder missing during sanity check!");
		}
	}

	Ok(())
}

// (Directory, PluginOrError)
type PluginFS = (String, Result<Plugin, PluginError>);

/// Finds all valid plugins (has plugin.toml, src/autorun.lua or src/hook.lua)
pub fn find() -> Result<Vec<PluginFS>, PluginError> {
	let mut plugins = vec![];

	let dir = std::fs::read_dir( configs::path(PLUGIN_DIR) )?;
	for d in dir {
		match d {
			Ok(d) => {
				let path = d.path();
				let path_name = path
					.file_name()
					.map(|x| x.to_string_lossy().to_string())
					.unwrap_or_else(|| path.display().to_string() );

				if path.is_dir() {
					let plugin_toml = path.join("plugin.toml");
					let res = if plugin_toml.exists() {
						let content = std::fs::read_to_string(plugin_toml)?;
						match toml::from_str::<serde::PluginToml>(&content) {
							Ok(toml) => Ok(
								Plugin {
									data: toml,
									dir: path
								}
							),
							Err(why) => Err(PluginError::Parsing(why))
						}
					} else {
						Err(PluginError::NoToml)
					};
					plugins.push((path_name, res));
				}
			}
			Err(why) => error!("Failed to read dir entry in autorun/plugins: {why}")
		}
	}
	Ok(plugins)
}

/// Run ``autorun.lua`` in all plugins.
pub fn call_autorun(l: LuaState, env: &AutorunEnv) -> Result<(), PluginError> {
	for (dirname, plugin) in find()? {
		match plugin {
			Ok(plugin) => {
				if plugin.has_file("src/autorun.lua") {
					if let Err(why) = plugin.dofile(l, "src/autorun.lua", env) {
						error!("Failed to run plugin '{}': {}", plugin.get_name(), why);
					};
				}
			},
			Err(why) => {
				error!("Failed to load plugin @plugins/{dirname}: {}", why);
			}
		}
	}

	Ok(())
}

/// Run ``hook.lua`` in all plugins.
/// Does not print out any errors unlike call_autorun.
pub fn call_hook(l: LuaState, env: &AutorunEnv) -> Result<(), PluginError> {
	for plugin in find()? {
		if let (_, Ok(plugin)) = plugin {
			let _ = plugin.dofile(l, "src/hook.lua", env);
		}
	}
	Ok(())
}

pub fn init() -> Result<(), PluginError> {
	let plugin_dir = configs::path( PLUGIN_DIR );
	if !plugin_dir.exists() {
		std::fs::create_dir(&plugin_dir)?;
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
			(_, Ok(plugin)) => info!("Verified plugin: {}", plugin.get_name())
		}
	}

	Ok(())
}