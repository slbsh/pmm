//#![feature(trivial_bounds)]

use std::{ops::Not, sync::LazyLock};

use stabby::libloading::StabbyLibrary;
use stabby::boxed::Box;
use pmm_abi::PmImplDyn;

mod config;
mod error;
mod args;

use config::PmConfig;

const DEFAULT_CONF_PATH: &str = "./config.toml";

static CONFIG: LazyLock<pmm_abi::Config> = LazyLock::new(|| {
	let file = std::env::var("PMM_CONFIG")
		.unwrap_or_else(|_| String::from(DEFAULT_CONF_PATH));

	config::deserialize_from(&file).unwrap_or_else(|e| err!("'{file}': {e}"))
});

fn main() {
	let args = args::Args::parse(std::env::args().skip(1));

	#[cfg(debug_assertions)]
	println!("{:#?}", LazyLock::force(&CONFIG));

	let package_managers = std::fs::read_dir(&*CONFIG.pmdir)
		.unwrap_or_else(|e| err!("'{}': {e}", CONFIG.pmdir))
		.filter_map(|entry| {
			let path = entry.unwrap_or_else(|e| err!("{e}")).path();
			path.is_dir().not().then_some(path)
		})
		.map(|path| match path.extension()
			.unwrap_or_else(|| err!("no extension for '{}'", path.display()))
			.to_str().unwrap() {
			"so"   => unsafe {
				let lib = libloading::Library::new(&path).unwrap_or_else(|e| err!("{e}"));

				type InitFn = extern "C" fn(&'static pmm_abi::Config) 
					-> stabby::dynptr!(Box<dyn pmm_abi::PmImpl>);

				let pmimpl = lib.get_canaried::<InitFn>(b"init")
					.unwrap_or_else(|e| crate::err!("{e}"))(&CONFIG);

				std::mem::forget(lib); // leak the lib so we never unload it

				pmimpl
			},
			"toml" => Box::new(config::deserialize_from::<PmConfig>(&path)
				.unwrap_or_else(|e| err!("'{}': {e}", path.display()))).into(),
			_ => err!("unknown filetype for '{}'", path.display()),
		})
		.collect::<Vec<_>>();


	#[cfg(debug_assertions)]
	println!("{:#?}", package_managers.len());

	let items = args.verbs.get(1..)
		.map_or(Vec::new(), |v| v.iter().map(|&s| s.into()).collect::<Vec<stabby::str::Str>>());

	match *args.verbs.first().unwrap_or_else(|| err!("no verb specified")) {
		"query" => package_managers.iter()
			.for_each(|pm| pm.query(items.as_slice().into())
				.match_owned(
					|p| p.match_owned(
						|p| {
							let name = pm.name();
							p.iter().for_each(|pkg| println!("{name}: {}", pkg.name));
						},
						|e| {
							let name = pm.name();
							warn!("{name}: {e}");
						}),
					|| (),
				)),
		v => err!("unknown verb '{v}'"),
	}

	// batch_cmd(&package_managers, &args.verbs, &config, Action::Query)
	// 	.iter().for_each(|r| println!("{}: {}({})", r.0, if r.1.is_ok() { "Ok" } else { "Err" }, r.1.as_ref().unwrap_or_else(|s|s)));
}
