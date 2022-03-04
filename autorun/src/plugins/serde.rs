use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct PluginToml {
	pub plugin: PluginMetadata,
	pub settings: toml::Value,
}

#[derive(Deserialize, Debug)]
pub struct PluginMetadata {
	pub name: String,
	pub author: String,
	pub version: String,
	pub description: Option<String>,

	pub language: Option<String>,
}