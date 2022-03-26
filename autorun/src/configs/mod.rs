use fs_err as fs;

// I know I could just derive / impl Default for all of these settings,
// but then there wouldn't be comments to explain what each setting is for.
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize)]
pub struct Settings {
	pub autorun: AutorunSettings,
	pub filesteal: FileSettings,
	pub logging: LoggerSettings,
	pub plugins: PluginSettings,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AutorunSettings {
	pub hide: bool,
	pub nocolor: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileSettings {
	pub enabled: bool,
	pub format: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoggerSettings {
	pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PluginSettings {
	pub enabled: bool,
}

use crate::{logging::{error, info}, fs::SETTINGS_PATH};

use once_cell::sync::Lazy;
pub static SETTINGS: Lazy<Settings> = Lazy::new(|| {
	let settings_file = crate::fs::in_autorun(SETTINGS_PATH);
	let default_settings = include_str!("settings.toml");

	if settings_file.exists() {
		match fs::read_to_string(&settings_file) {
			Ok(content) => match toml::from_str(&content) {
				Ok(settings) => settings,
				Err(why) => {
					error!("Failed to parse your autorun/settings.toml file: {why}");

					toml::from_str(default_settings).expect("Failed to parse default settings")
				}
			},
			Err(why) => {
				error!("Failed to read your settings file '{why}'. Using default settings!");

				toml::from_str(default_settings).expect("Failed to parse default settings")
			}
		}
	} else {
		// No settings file, create file with default settings, and use that.
		match fs::File::create(settings_file) {
			Err(why) => {
				error!("Couldn't create settings file: {why}");
			}
			Ok(mut handle) => {
				use std::io::Write;
				match handle.write_all(default_settings.as_bytes()) {
					Err(why) => {
						error!("Couldn't write default settings: {why}");
					}
					Ok(_) => {
						info!("No settings found, created default settings file!");
					}
				}
			}
		};

		toml::from_str(default_settings).expect("Failed to parse default settings")
	}
});
