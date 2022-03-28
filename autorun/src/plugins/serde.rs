use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct PluginToml {
	pub plugin: PluginMetadata,
	pub settings: toml::Value,
}

#[derive(Deserialize, Debug)]
pub struct PluginMetadata {
	pub name: String,   // Name of the plugin to be displayed to the user
	pub author: String, // TODO: Maybe make this a list?
	pub version: String,
	pub description: Option<String>,

	pub language: Option<String>,
	pub version_required: Option<String>, // Required version of Autorun for the plugin to run
}
