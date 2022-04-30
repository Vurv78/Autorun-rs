use fs_err as fs;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize, Default, PartialEq)]
pub struct Settings {
	pub autorun: AutorunSettings,
	pub filesteal: FileSettings,
	pub logging: LoggerSettings,
	pub plugins: PluginSettings,
}

impl Settings {
	pub fn color_enabled(&self) -> bool {
		#[allow(deprecated)]
		!self.autorun.no_color
			.unwrap_or_else(|| self.autorun.nocolor.unwrap_or(false) )
	}
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct AutorunSettings {
	pub hide: bool,
	#[deprecated(since = "1.2.3", note = "Use `no_color` instead")]
	pub nocolor: Option<bool>,
	pub no_color: Option<bool>,
	pub check_version: bool
}

impl Default for AutorunSettings {
	fn default() -> Self {
		#[allow(deprecated)]
		Self {
			hide: false,

			nocolor: None,

			no_color: Some(false),
			check_version: true
		}
	}
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct FileSettings {
	pub enabled: bool,
	pub format: String,
}

impl Default for FileSettings {
	fn default() -> Self {
		Self {
			enabled: true,
			format: "<ip>".to_owned(),
		}
	}
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct LoggerSettings {
	pub enabled: bool,
}

impl Default for LoggerSettings {
	fn default() -> Self {
		Self { enabled: true }
	}
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct PluginSettings {
	pub enabled: bool,
}

impl Default for PluginSettings {
	fn default() -> Self {
		Self { enabled: true }
	}
}

use crate::fs::SETTINGS_PATH;

pub static SETTINGS: Lazy<Settings> = Lazy::new(|| {
	let settings_file = crate::fs::in_autorun(SETTINGS_PATH);
	let default_settings = include_str!("settings.toml");

	if settings_file.exists() {
		match fs::read_to_string(&settings_file) {
			Ok(content) => match toml::from_str(&content) {
				Ok(settings) => settings,
				Err(why) => {
					eprintln!("Failed to parse your autorun/settings.toml file ({why}). Using default settings.");
					Settings::default()
				}
			},
			Err(why) => {
				eprintln!("Failed to read your settings file ({why}). Using default settings!");
				Settings::default()
			}
		}
	} else {
		// No settings file, create file with default settings, and use that.
		if let Err(why) = fs::write(settings_file, default_settings) {
			eprintln!("Failed to create default settings file file ({why})");
		}
		Settings::default()
	}
});