use std::fmt::{self, Display, Formatter};
use janetrs::{Janet, JanetKeyword, JanetStruct};

use colored::Colorize;
use crate::util::JanetInto;

pub type Backend = String;

pub struct Backends(Vec<Backend>);

impl Backends {
	pub fn get_from_dir(rt: &mut janetrs::client::JanetClient, path: impl AsRef<std::path::Path>) -> Self {
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

				ns.to_string()
			})
			.collect::<Vec<_>>();

		if ns.is_empty() {
			crate::err!("{}: No frontends found", path.as_ref().display());
		}

		Self(ns)
	}
}

impl std::ops::Deref for Backends {
	type Target = Vec<Backend>;
	fn deref(&self) -> &Self::Target { &self.0 }
}

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

impl Display for Package {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{} {}{}\n{}", 
			self.name.bold(), 
			self.alias.as_ref().map_or(String::new(), |a| format!("{} {} ", "as".bold().cyan(), a.bold())),
			self.version.to_string().green().bold(),
			self.desc.trim(),
		)
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

		writeln!(f, "Name:         {}", self.pkg.name)?;
		writeln!(f, "Version:      {}", self.pkg.version)?;
		writeln!(f, "Description:  {}", self.pkg.desc)?;
		writeln!(f, "Authors:      {}", self.pkg.authors.as_ref()
			.map_or_else(|| String::from("Unknown"), |a| a.join(", ")))?;
		writeln!(f, "Url:          {}", self.pkg.url)?;
		
		writeln!(f, "Dependencies: {}", self.deps.join(", "))?;
		writeln!(f, "License:      {}", self.license)?;
		writeln!(f, "Release-Date: {}", self.release_date)?;
		writeln!(f, "Source:       {}", self.source.as_ref().map_or("Unknown or n/a", |v| v))?;
		writeln!(f, "Groups:       {}", self.groups.as_ref()
			.map_or_else(|| String::from("None"), |g| g.join(", ")))?;
		writeln!(f, "Downloads:    {}", self.downloads.as_ref()
			.map_or_else(|| String::from("Unknown"), ToString::to_string))?;
		writeln!(f, "Homepage:     {}", self.homepage.as_ref()
			.map_or_else(|| String::from("Unknown"), ToString::to_string))?;
		write!(f, "Size:         {}", self.size.as_ref()
			.map_or_else(|| String::from("Unknown"), fmt_bytes))?;

		Ok(())
	}
}

// impl<T: TryFrom<Janet, Error = JanetConversionError>> JanetInto<T> for Janet {
// 	fn janet_into(self) -> T {
// 		self.try_into().unwrap_or_else(|e| crate::err!("{e}"))
// 	}
// }
