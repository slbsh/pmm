
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
	#[cfg_attr(feature = "serde", serde(default))]
	pub shell:      Option<String>,
	#[cfg_attr(feature = "serde", serde(default, deserialize_with="config_deser_env"))]
	pub env:        Vec<Tuple2<String, String>>,
}

#[cfg(feature = "serde")]
fn config_deser_env<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Vec<Tuple2<String, String>>, D::Error> {
	let map: std::collections::HashMap<String, String> = serde::Deserialize::deserialize(d)?;
	Ok(map.into_iter().map(Into::into).collect())
}

#[derive(Debug, Default)]
#[stabby::stabby]
pub struct Package {
	pub name:        String,
	pub installed:   Option<bool>,
	pub version:     Option<String>,
	pub repo:        Option<String>,
	pub description: Option<String>,
}

impl std::fmt::Display for Package {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.repo.match_ref(|r| write!(f, "{r}/"), || Ok(()))?;
		write!(f, "{}", self.name)?;
		self.version.match_ref(|v| write!(f, " {v}"), || Ok(()))?;
		self.installed.match_ref(|&i| if i { write!(f, " [installed]") } else { Ok(()) }, || Ok(()))?;
		self.description.match_ref(|d| write!(f, " {d}"), || Ok(()))?;
		Ok(())
	}
}

#[stabby::stabby]
pub trait PmImpl {
	extern "C" fn name(&self) -> String;
	extern "C" fn query(&self, item: Str) -> Option<Result<Package, String>>;
	extern "C" fn list(&self, items: Slice<Str>) -> Option<Result<Vec<Package>, String>>;
}
