use std::path::Path;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::io::Write;

use crate::CONFIG;

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all="kebab-case")]
pub struct PmConfig {
	pub name:  String,
	#[serde(default)]
	pub run_cond: Option<String>,
	#[serde(default)]
	pub strat: Option<String>,
	#[serde(default)]
	pub env:   HashMap<String, String>,
	#[serde(rename="impl")]
	pub impls: HashMap<Action, Box<str>>,
}

#[derive(Hash, PartialEq, Debug, Eq, serde::Deserialize)]
#[serde(rename_all="kebab-case")]
pub enum Action {
	Query,  // check if package exists
	Add,    // add packages, polls query before to check precedence
	Remove, // remove packages, polls query to check with to remove
	List,   // list all packages with name
}

impl std::fmt::Display for Action {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", match self {
			Action::Query  => "query",
			Action::Add    => "add",
			Action::Remove => "remove",
			Action::List   => "list",
		})
	}
}

use stabby::{
	slice::Slice as sSlice,
	string::String as sString,
	option::Option as sOption,
	result::Result as sResult,
	vec::Vec as sVec,
	str::Str as sStr,
	tuple::Tuple2,
};

impl pmm_abi::PmImpl for PmConfig {
	extern "C" fn name(&self) -> sString {
		self.name.clone().into()
	}

	extern "C" fn query(&self, item: sStr) -> sOption<sResult<pmm_abi::Package, sString>> {
		if !self.test_cond() { return sOption::None(); }

		todo!("{:?}", self.run([item][..].into(), Action::Query))
	}

	extern "C" fn list(&self, items: sSlice<sStr>) -> sOption<sResult<sVec<pmm_abi::Package>, sString>> {
		if !self.test_cond() { return sOption::None(); }

		// TODO: collect all pkg info
		self.run(items, Action::List).map(|s| s.map(|s| 
			s.split('\n')
				.filter(|s| !s.is_empty())
				.map(|s| pmm_abi::Package { name: s.into(), ..Default::default() }).collect())).into()
	}
}

impl PmConfig {
	fn run(&self, items: sSlice<sStr>, action: Action) -> Option<sResult<sString, sString>> {
		let Some(cmd) = self.impls.get(&action) else {
			crate::warn!("action '{action}' not implemented for '{}'", self.name);
			return None;
		};

		let mut child = Command::new(CONFIG.shell.as_ref()
			.unwrap_or_else(|| crate::err!("a shell must be specified to run non `.so` package manegers"))
			.as_ref())
			.envs(&self.env)
			.envs(CONFIG.env.iter().map(|Tuple2(k, v)| (k.as_str(), v.as_str())))
			.env("items", items.iter().fold(String::new(), |acc, s| acc + s + " "))
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()
			.map_err(|e| crate::warn!("failed to run action '{action}' for '{}': {e}", self.name)).ok()?;

		let cmd = match self.strat {
			Some(ref strat) => format!("strat -r {strat} {cmd}"),
			None => cmd.to_string(),
		};

		child.stdin.as_mut().unwrap()
			.write_all(cmd.as_bytes()).unwrap();

		let out = child.wait_with_output().unwrap();

		let stdout = String::from_utf8(out.stdout)
			.map_err(|_| crate::warn!("invalid utf8 in stdout for '{}'", self.name)).ok()?
			.into();

		let stderr = String::from_utf8(out.stderr)
			.map_err(|_| crate::warn!("invalid utf8 in stderr for '{}'", self.name)).ok()?
			.into();

		Some(out.status.success().then_some(stdout).ok_or(stderr).into())
	}

	fn test_cond(&self) -> bool {
		if self.run_cond.is_none() { return true; }

		let child = Command::new(CONFIG.shell.as_ref()
			.unwrap_or_else(|| crate::err!("a shell must be specified to run non `.so` package manegers"))
			.as_ref())
			.envs(&self.env)
			.envs(CONFIG.env.iter().map(|Tuple2(k, v)| (k.as_str(), v.as_str())))
			.stdin(Stdio::piped())
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.spawn()
			.map_err(|e| crate::warn!("failed to run condition check for '{}': {e}", self.name));

		let mut child = match child {
			Ok(child) => child,
			Err(_) => return false,
		};

		let cmd = self.run_cond.as_ref().unwrap();
		let cmd = match self.strat {
			Some(ref strat) => format!("strat -r {strat} {cmd}"),
			None => cmd.to_string(),
		};

		child.stdin.as_mut().unwrap()
			.write_all(cmd.as_bytes()).unwrap();

		child.wait().unwrap().success()
	}
}

pub fn deserialize_from<T>(path: impl AsRef<Path>) -> Result<T, Box<dyn std::error::Error>>
	where T: for<'de> serde::Deserialize<'de> {
	Ok(toml::from_str(&std::fs::read_to_string(&path)?)?)
}
