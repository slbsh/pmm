use std::fmt::{self, Display, Formatter};
use janetrs::{Janet, JanetArray, JanetKeyword, JanetStruct};

use colored::Colorize;
use crate::util::JanetInto;
use crate::keyword;

pub struct Backend {
	pub name:   String,
	pub colour: (u8, u8, u8),
}

impl Display for Backend {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "\x1b[38;2;{};{};{}m{}\x1b[0m", 
			self.colour.0, self.colour.1, self.colour.2, 
			self.name.bold())
	}
}

pub struct Backends(Vec<Backend>);

impl Backends {
	pub fn from_dir(rt: &mut janetrs::client::JanetClient, path: impl AsRef<std::path::Path>) -> Self {
		let ns = std::fs::read_dir(&path)
			.unwrap_or_else(|e| crate::err!("{}: {e}", path.as_ref().display()))
			.filter_map(|entry| {
				let path = entry.unwrap_or_else(|e| crate::err!("{e}")).path();
				(!path.is_dir()).then_some(path)
			})
			.map(|path| {
				let mut path = std::path::absolute(&path).unwrap();
				path.set_extension("");
				let ns = path.file_stem().unwrap().to_string_lossy();

				rt.run(format!("(import @{} :as {ns})", path.to_str().unwrap()))
					.unwrap_or_else(|e| crate::err!("{e}"));

				let env = rt.env().unwrap();

				Backend {
					name:   ns.to_string(),
					colour: match env.resolve(format!("{ns}/COLOUR")) {
						None => {
							crate::warn!("{ns}: `COLOUR` not specified, set to `nil` to use the default");
							(u8::MAX, u8::MAX, u8::MAX)
						},
						Some(c) => c.janet_into(),
					},
				}
			})
			.collect::<Vec<_>>();

		if ns.is_empty() {
			crate::err!("{}: No frontends found", path.as_ref().display());
		}

		Self(ns)
	}

	// linear search is fine 0 way this is gonna be a bottleneck
	pub fn get(&self, name: &str) -> &Backend {
		self.0.iter().find(|b| b.name == name)
			.unwrap_or_else(|| crate::err!("{}: Backend not found", name))
	}
}

impl std::ops::Deref for Backends {
	type Target = Vec<Backend>;
	fn deref(&self) -> &Self::Target { &self.0 }
}

// deriving default so we can std::mem:take it later
#[derive(Default)]
pub struct Package {
	pub name:       String,
	pub version:    String,
	pub alias:      Option<String>,
	pub desc:       String,
	pub authors:    Option<Vec<String>>,
	pub url:        String,
}

impl JanetInto<Package> for Janet {
	fn janet_into(self) -> Package {
		let j: JanetStruct = self.janet_into();

		Package { 
			name:    j.get(JanetKeyword::from("name"))
				.unwrap_or_else(|| crate::err!("missing field `:name`")).janet_into(),
			version: j.get(JanetKeyword::from("version"))
				.unwrap_or_else(|| crate::err!("missing field `:version`")).janet_into(),
			alias:   None, // TODO: alias???
			desc:    j.get(JanetKeyword::from("description"))
				.unwrap_or_else(|| crate::err!("missing field `:description`")).janet_into(),
			authors: j.get(JanetKeyword::from("authors")).and_then(|a| a.janet_into()),
			url:     j.get(JanetKeyword::from("url"))
				.unwrap_or_else(|| crate::err!("missing field `:url`")).janet_into(),
		}
	}
}

impl Into<Janet> for &Package {
	fn into(self) -> Janet {
		Janet::from(janetrs::structs! {
			keyword![name]        => self.name.as_str(),
			keyword![version]     => self.version.as_str(),
			keyword![alias]       => self.alias.as_ref()
				.map_or_else(Janet::nil, |a| a.as_str().into()),
			keyword![description] => self.desc.as_str(),
			keyword![authors]     => self.authors.as_ref()
				.map_or_else(Janet::nil, |a| Janet::array(a.iter().map(|a| Janet::from(a.as_str())).collect())),
			keyword![url]         => self.url.as_str(),
		})
	}
}

pub struct PackageInfo {
	pub pkg:          Package,
	pub deps:         Vec<String>,
	pub license:      String,
	pub release_date: String,
	pub source:       Option<String>,
	pub groups:       Option<Vec<String>>,
	pub downloads:    Option<u64>,
	pub homepage:     Option<String>,
	pub size:         Option<u64>,
}

impl JanetInto<PackageInfo> for Janet {
	fn janet_into(self) -> PackageInfo {
		let j: JanetStruct = self.janet_into();

		PackageInfo {
			pkg: j.get(JanetKeyword::from("pkg"))
				.unwrap_or_else(|| crate::err!("missing field `:package`"))
				.janet_into(),
			deps: j.get(JanetKeyword::from("deps"))
				.unwrap_or_else(|| crate::err!("missing field `:deps`"))
				.janet_into(),
			license: j.get(JanetKeyword::from("license"))
				.unwrap_or_else(|| crate::err!("missing field `:license`"))
				.janet_into(),
			release_date: j.get(JanetKeyword::from("release-date"))
				.unwrap_or_else(|| crate::err!("missing field `:release-date`"))
				.janet_into(),
			source: j.get(JanetKeyword::from("source"))
				.and_then(|s| s.janet_into()),
			groups: j.get(JanetKeyword::from("groups"))
				.and_then(|g| g.janet_into()),
			downloads: j.get(JanetKeyword::from("downloads"))
				.and_then(|d| d.janet_into()),
			homepage: j.get(JanetKeyword::from("homepage"))
				.and_then(|h| h.janet_into()),
			size: j.get(JanetKeyword::from("size"))
				.and_then(|s| s.janet_into()),
		}
	}
}

impl Display for PackageInfo {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let fmt_bytes = |&n| {
			const KB: u64 = 1000;
			const MB: u64 = 1000 * KB;
			const GB: u64 = 1000 * MB;
			const TB: u64 = 1000 * GB;
			const PB: u64 = 1000 * TB;
			match n {
				0 ..KB => format!("{}b", n),
				KB..MB => format!("{:.2}kb", n as f64 / KB as f64),
				MB..GB => format!("{:.2}Mb", n as f64 / MB as f64),
				GB..TB => format!("{:.2}Gb", n as f64 / GB as f64),
				TB..PB => format!("{:.2}Tb", n as f64 / TB as f64),
				PB..   => format!("{:.2}Pb", n as f64 / PB as f64), 
				// 0 way any package reaches beyond :L
			}
		};

		writeln!(f, "name:         {}", self.pkg.name)?;
		writeln!(f, "version:      {}", self.pkg.version)?;
		writeln!(f, "description:  {}", self.pkg.desc)?;
		writeln!(f, "authors:      {}", self.pkg.authors.as_ref()
			.map_or_else(|| String::from("Unknown"), |a| a.join(", ")))?;
		writeln!(f, "url:          {}", self.pkg.url)?;
		
		writeln!(f, "dependencies: {}", self.deps.join(", "))?;
		writeln!(f, "license:      {}", self.license)?;
		writeln!(f, "release-Date: {}", self.release_date)?;
		writeln!(f, "source:       {}", self.source.as_ref().map_or("Unknown or n/a", |v| v))?;
		writeln!(f, "groups:       {}", self.groups.as_ref()
			.map_or_else(|| String::from("None"), |g| g.join(", ")))?;
		writeln!(f, "downloads:    {}", self.downloads.as_ref()
			.map_or_else(|| String::from("Unknown"), ToString::to_string))?;
		writeln!(f, "homepage:     {}", self.homepage.as_ref()
			.map_or_else(|| String::from("Unknown"), ToString::to_string))?;
		write!(f, "size:         {}", self.size.as_ref()
			.map_or_else(|| String::from("Unknown"), fmt_bytes))?;

		Ok(())
	}
}
