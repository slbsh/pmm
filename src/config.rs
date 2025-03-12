use std::path::Path;
use std::collections::HashMap;

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all="kebab-case")]
pub struct Config {
	pub pmdir:      Box<Path>,
	pub world_path: Box<Path>,
	pub shell:      String,
	#[serde(default)]
	pub env:        HashMap<String, String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all="kebab-case")]
pub struct PmConfig {
	#[serde(default)]
	pub env:   HashMap<String, String>,
	#[serde(default)]
	pub args:  Box<str>,
	#[serde(rename="impl")]
	pub impls: HashMap<PmImpl, Box<str>>,
}

#[derive(Hash, PartialEq, Eq, Debug, serde::Deserialize)]
#[serde(rename_all="kebab-case")]
pub enum PmImpl {
	Query,  // list all packages with name
	Add,    // add packages, polls query before to check precedence
	Remove, // remove packages, polls query to check with to remove
}

impl PmImpl {
	pub fn from_str(s: &str) -> Option<Self> {
		match s {
			"query"  => Some(Self::Query),
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
