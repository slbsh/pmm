use std::collections::HashMap;
use janetrs::{JanetKeyword, TaggedJanet};

#[derive(Debug)]
pub struct Config {
	pub backend_dir: String,
	pub world_path:   String,
	pub env:          HashMap<String, String>,
	pub priority:     Vec<String>,
}

impl Config {
	pub fn eval_from_file(rt: &mut janetrs::client::JanetClient, path: &str) -> Config {
		let file = std::fs::read_to_string(path)
			.unwrap_or_else(|e| crate::err!("{path}: {e}"));

		let config = match rt.run(file).map(|v| v.unwrap()) {
			Err(e) => crate::err!("{path}: {e}"),
			Ok(TaggedJanet::Struct(m)) => m,
			Ok(t) => crate::err!("{path}: Expected `map`, got `{}`", t.kind()),
		};

		Self {
			backend_dir: config.get(JanetKeyword::new("backend-dir"))
				.unwrap_or_else(|| crate::err!("{path}: missing field `:backend-dir`"))
				.to_string(),

			world_path: config.get(JanetKeyword::new("world-path"))
				.unwrap_or_else(|| crate::err!("{path}: missing field `:world-path`"))
				.to_string(),

			env: config.get(JanetKeyword::new("env"))
				.map_or_else(|| Default::default(), |v| 
					match v.unwrap() {
					TaggedJanet::Struct(s) => s.into_iter()
						.map(|(k, v)| (k.to_string(), v.to_string()))
						.collect(),
					t => crate::err!("{path}: Expected `map`, got `{}`", t.kind()),
				}),

			priority: config.get(JanetKeyword::new("priority"))
				.map_or_else(|| Default::default(), |v| 
					match v.unwrap() {
					TaggedJanet::Tuple(a) => a.into_iter()
						.map(|v| v.to_string())
						.collect(),
					t => crate::err!("{path}: Expected `tuple`, got `{}`", t.kind()),
				}),
		}
	}
}
