use std::ops::Not;

mod config;
mod error;
mod args;

fn main() {
	let args = args::Args::parse(std::env::args().skip(1));


	let config = {
		let file = std::env::var("PMM_CONFIG")
			.unwrap_or_else(|_| String::from("config.toml"));

		config::deserialize_from::<config::Config>(&file)
			.unwrap_or_else(|e| err!("'{file}': {e}"))
	};

	#[cfg(debug_assertions)]
	println!("{:#?}", config);


	let package_managers = std::fs::read_dir(&config.pmdir)
		.unwrap_or_else(|e| err!("'{}': {e}", config.pmdir.display()))
		.filter_map(|entry| {
			let path = entry.unwrap_or_else(|e| err!("{e}")).path();
			path.is_dir().not().then_some(path)
		})
		.map(|path| (
			path.file_stem().unwrap().to_string_lossy().into_owned(),
			config::deserialize_from::<config::PmConfig>(&path)
				.unwrap_or_else(|e| err!("'{}': {e}", path.display()))))
		.collect::<Vec<_>>();
	
	#[cfg(debug_assertions)]
	println!("{:#?}", package_managers);


	// FIXME: have some way to use the pkgmgrs interface
	let action = config::PmImpl::from_str(&args.verbs.first()
		.unwrap_or_else(|| err!("no action specified")));
}
