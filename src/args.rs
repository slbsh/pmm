use std::collections::HashMap;

const HELP_MSG: &str = 
"Bedrock Linux \x1b[1mP\x1b[0mackage \x1b[1mM\x1b[0manager \x1b[1mM\x1b[0manager

Usage: pmm [options] [command] [args]

\x1b[1mOPTIONS\x1b[0m
	-h, --help       Show this help message
	-v, --version    Show version information";

#[derive(Default, Debug)]
pub struct Args(HashMap<&'static str, Option<&'static str>>);

impl Args {
	pub fn get(&self, key: &str) -> bool {
		self.0.get(key).is_some_and(|v| v.is_none())
	}

	pub fn get_with_opt(&self, key: &str) -> Option<&'static str> {
		self.0.get(key).map(|v| v.unwrap_or_else(||
			crate::err!("Missing value for arg '{key}'\nRun with {} for usage information", "--help".bold())))
	}

	pub fn parse<I: std::iter::Iterator<Item = String>>(args: I) -> (Self, Vec<String>) {
		args.fold((Args::default(), Vec::new()), |mut acc, arg| {
			match arg.starts_with('-') {
				true => {
					let arg = Box::leak(arg.into_boxed_str()).trim_start_matches("-");
					let (key, val) = arg.split_once("=").map_or((arg, None), |(k, v)| (k, Some(v)));
					acc.0.0.insert(key, val);
				},
				_ => acc.1.push(arg),
			}; acc
		})
	}

	pub fn handle_base_flags(&self) {
		if self.get("h") || self.get("help") {
			println!("{HELP_MSG}");
			std::process::exit(0);
		}

		if self.get("v") || self.get("version") {
			println!("{}", env!("CARGO_PKG_VERSION"));
			std::process::exit(0);
		}
	}
}
