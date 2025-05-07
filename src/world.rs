// TODO: world file handling

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fs::File;
use std::io::BufRead;
use std::str::FromStr;

pub struct PackageEntry {
	pub name:    String,
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

	fn deserialize(b: impl BufRead) -> Result<BTreeMap<String, PackageEntry>, String> {
		b.lines()
			.filter_map(|l| l.ok())
			.enumerate()
			.filter(|(_, l)| !l.chars().all(char::is_whitespace) || !l.starts_with('#'))
			.map(|(i, l)| {
				let mut parts = l.split_whitespace();

				let backend = parts.next().unwrap().to_string();
				let entry = PackageEntry {
					name: parts.next()
						.map(String::from)
						.ok_or_else(|| format!("line {i}: Invalid entry format"))?,
					version: parts.next()
						.map(|s| s.parse().unwrap())
						.ok_or_else(|| format!("line {i}: Invalid entry format"))?,
					alias: parts.next().map(String::from),
				};

				Ok::<_, String>((i, backend, entry))
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
}
