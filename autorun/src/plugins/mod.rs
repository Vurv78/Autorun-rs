use crate::configs::PLUGIN_DIR;

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
	#[error("IO Error: {0}")]
	IO(#[from] std::io::Error)
}

#[non_exhaustive]
pub enum PluginLanguage {
	// In the future there could be more languages like Teal or Expressive
	Lua
}

impl Default for PluginLanguage {
	fn default() -> Self { Self::Lua }
}

#[derive(Default)]
pub struct PluginSettings {
	language: PluginLanguage
}

pub struct Plugin {
	name: String,
	path: std::path::PathBuf,

	// Path to entrypoint file.
	autorun: std::path::PathBuf,

	// Retrieved from plugin.toml
	settings: PluginSettings
}

// Plugin system
pub fn find() -> Result<(), PluginError> {
	let dir = std::fs::read_dir( PLUGIN_DIR )?;

	Ok(())
}

pub fn exec() -> Result<(), PluginError> {
	Ok(())
}