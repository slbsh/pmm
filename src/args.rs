use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct Args {
	args:  HashMap<&'static str, Option<&'static str>>,
	verbs: Vec<&'static str>,
}

impl Args {
	pub fn get_with_opt(&self, key: &str) -> Option<&'static str> {
		self.args.get(key).map(|v| v.unwrap_or_else(||
			crate::err!("Missing value for arg '{key}'\nRun with {} for usage information", "--help".bold())))
	}

	pub fn parse<I: std::iter::Iterator<Item = String>>(args: I) -> Self {
		args.fold(Args::default(), |mut acc, arg| {
			match arg.starts_with('-') {
				true => {
					let arg = Box::leak(arg.into_boxed_str()).trim_start_matches("-");
					let (key, val) = arg.split_once("=").map_or((arg, None), |(k, v)| (k, Some(v)));
					acc.args.insert(key, val);
				},
				_ => acc.verbs.push(Box::leak(arg.into_boxed_str())),
			}; acc
		})
	}
}
