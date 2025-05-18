// TODO: world file handling

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fs::File;
use std::io::BufRead;
use std::str::FromStr;

use colored::Colorize;

use crate::backend::{Package, Backend};

pub struct PackageEntry {
	pub backend: String,
	pub alias:   Option<String>,
	pub version: PackageVersion,
}

pub enum PackageVersion {
	Exact(String),
	LowerBound(String),
	UpperBound(String),
	Range(String, String),
	Any,
}

impl FromStr for PackageVersion {
	type Err = Infallible;
	// FIXME: if the actual version has .. you're cooked 💀
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.split_once("..") {
			Some(("", "")) => Ok(PackageVersion::Any),
			Some((s, ""))  => Ok(PackageVersion::LowerBound(String::from(s))),
			Some(("", s))  => Ok(PackageVersion::UpperBound(String::from(s))),
			Some((a, b))   => Ok(PackageVersion::Range(String::from(a), String::from(b))),
			None           => Ok(PackageVersion::Exact(String::from(s))),
		}
	}
}

impl std::fmt::Display for PackageVersion {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			PackageVersion::Exact(s)      => write!(f, "{s}"),
			PackageVersion::LowerBound(s) => write!(f, "{s}.."),
			PackageVersion::UpperBound(s) => write!(f, "..{s}"),
			PackageVersion::Range(a, b)   => write!(f, "{a}..{b}"),
			PackageVersion::Any           => write!(f, ".."),
		}
	}
}

pub struct World {
	file: File,
	data: BTreeMap<String, PackageEntry>,
}

impl World {
	pub fn new(path: impl AsRef<std::path::Path>) -> Self {
		let file = File::options()
			.read(true).write(true)
			.create(true).open(&path)
			.unwrap_or_else(|e| crate::err!("{}: {e}", path.as_ref().display()));
		
		let buf = std::io::BufReader::new(file.try_clone()
			.unwrap_or_else(|e| crate::err!("{}: {e}", path.as_ref().display())));

		Self {
			data: Self::deserialize(buf).unwrap_or_else(|e| crate::err!("{}: {e}", path.as_ref().display())),
			file,
		}
	}

	pub fn add_package(&mut self, p: Package, bname: &str) {
		let Package { name, version, alias, .. } = p;

		if self.data.contains_key(&name) {
			crate::warn!("Package `{name}` already exists in world file");

			let res = crate::util::prompt_blocking(format!("Proceed anyway? (y/n)\n{}", ":: ".blue()).bold());

			match res.as_str() {
				"y" | "Y" | "" => {},
				"n" | "N" => crate::err!("Aborting"),
				_ => crate::warn!("??? (y/n)"),
			}
		}

		self.data.insert(name, PackageEntry { backend: String::from(bname), version: version.parse().unwrap(), alias });

		self.save().unwrap_or_else(|e| crate::err!("{e}"));
	}

	fn save(&mut self) -> std::io::Result<()> {
		use std::io::Write;
		self.file.set_len(0)?;
		self.file.write_all(self.serialize().as_bytes())?;
		Ok(())
	}

	fn deserialize(b: impl BufRead) -> Result<BTreeMap<String, PackageEntry>, String> {
		b.lines()
			.filter_map(|l| l.ok())
			.enumerate()
			.filter(|(_, l)| !l.chars().all(char::is_whitespace) || !l.starts_with('#'))
			.map(|(i, l)| {
				let mut parts = l.split_whitespace();

				let backend = parts.next().unwrap().to_string();
				let name = parts.next()
					.map(String::from)
					.ok_or_else(|| format!("line {i}: Invalid entry format"))?;

				let entry = PackageEntry {
					backend,
					version: parts.next()
						.map(|s| s.parse().unwrap())
						.ok_or_else(|| format!("line {i}: Invalid entry format"))?,
					alias: parts.next().map(String::from),
				};

				Ok::<_, String>((i, name, entry))
			})
			.try_fold(BTreeMap::new(), |mut acc, i| {
				let (i, k, v) = i?;
				match acc.contains_key(&k) {
					false => {
						acc.insert(k, v);
						Ok(acc)
					},
					true  => Err(format!("line {i}: Duplicate entry for {k}")),
				}
			})
	}

	// TODO: if this is a bottleneck maybe update just the changed lines
	fn serialize(&self) -> String {
		self.data.iter().map(|(k, v)|
			format!("{} {} {}{}\n", 
				v.backend, k, v.version.to_string(),
				v.alias.as_ref().map_or(String::new(), |a| format!(" {a}"))))
			.collect()
	}

	pub fn has_package(&self, p: &Package, b: &Backend) -> bool {
		self.data.get(&p.name).is_some_and(|v| v.backend == b.name)
	}
}
