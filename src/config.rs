use std::path::Path;
use std::collections::HashMap;

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
	pub pmdir:      Box<Path>,
	pub world_path: Box<Path>,
	#[serde(default)]
	pub env:        HashMap<String, Box<str>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmConfig {
	#[serde(default)]
	pub env:   HashMap<String, Box<str>>,
	#[serde(default)]
	pub args:  Box<str>,
	#[serde(rename="impl")]
	pub impls: HashMap<PmImpl, Box<str>>,
}

#[derive(Hash, PartialEq, Eq, Debug, serde::Deserialize)]
#[serde(rename_all="lowercase")]
pub enum PmImpl {
	Add,
	Remove,
}

impl PmImpl {
	pub fn from_str(s: &str) -> Option<Self> {
		match s {
			"add"    => Some(Self::Add),
			"remove" => Some(Self::Remove),
			_        => None,
		}
	}
}

pub fn deserialize_from<T>(path: impl AsRef<Path>) -> Result<T, Box<dyn std::error::Error>>
	where T: for<'de> serde::Deserialize<'de> {
	Ok(toml::from_str(&std::fs::read_to_string(&path)?)?)
}
