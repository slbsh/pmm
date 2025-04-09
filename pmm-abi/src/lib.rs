use std::collections::HashMap;

use stabby::{
	slice::Slice,
	string::String,
	option::Option,
	result::Result,
	vec::Vec,
	str::Str,
	tuple::Tuple2,
};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all="kebab-case", deny_unknown_fields))]
#[stabby::stabby]
pub struct Config {
	pub pmdir:      String,
	pub world_path: String,
	#[serde(default)]
	pub shell:      Option<String>,
	#[serde(default, deserialize_with="config_deser_env")]
	pub env:        Vec<Tuple2<String, String>>,
}

fn config_deser_env<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Vec<Tuple2<String, String>>, D::Error> {
	let map: HashMap<String, String> = serde::Deserialize::deserialize(d)?;
	Ok(map.into_iter().map(Into::into).collect())
}

#[derive(Debug)]
#[stabby::stabby]
pub struct Package {
	pub name: String,
}

impl std::fmt::Display for Package {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.name)
	}
}

#[stabby::stabby]
pub trait PmImpl {
	extern "C" fn name(&self) -> String;
	extern "C" fn query(&self, items: Slice<Str>) -> Option<Result<Package, String>>;
	extern "C" fn list(&self, items: Slice<Str>) -> Option<Result<Vec<Package>, String>>;
}
