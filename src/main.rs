use janetrs::{env::DefOptions, Janet, JanetKeyword, JanetString, TaggedJanet};

mod config;
mod error;
mod args;
mod prelude;

const DEFAULT_CONF_PATH: &str = "./config.janet";

fn main() {
	let args = args::Args::parse(std::env::args().skip(1));

	args.handle_base_flags();

	let mut rt = janetrs::client::JanetClient::init()
		.unwrap_or_else(|e| crate::err!("{e}"))
		.load_env_default();

	prelude::append(&mut rt);

	let config = {
		let filename = std::env::var("PMM_CONFIG")
			.unwrap_or_else(|_| String::from(DEFAULT_CONF_PATH));

		let file = std::fs::read_to_string(&filename)
			.unwrap_or_else(|e| err!("{filename}: {e}"));

		let config = match rt.run(file).map(|v| v.unwrap()) {
			Err(e) => err!("{e}"),
			Ok(TaggedJanet::Struct(m)) => m,
			Ok(t) => err!("Expected `map`, got `{}`", t.kind()),
		};

		config::Config {
			frontend_dir: config.get(JanetKeyword::new("frontend-dir"))
				.unwrap_or_else(|| err!("config missing field `:frontend-dir`"))
				.to_string(),

			world_path: config.get(JanetKeyword::new("world-path"))
				.unwrap_or_else(|| err!("config missing field `:world-path`"))
				.to_string(),

			env: config.get(JanetKeyword::new("env"))
				.map_or_else(|| Default::default(), |v| 
					match v.unwrap() {
					TaggedJanet::Struct(s) => s.into_iter()
						.map(|(k, v)| (k.to_string(), v.to_string()))
						.collect(),
					t => err!("Expected `map`, got `{}`", t.kind()),
				}),
		}
	};

	#[cfg(debug_assertions)]
	println!("{:#?}", config);

	let mut frontends = std::fs::read_dir(&config.frontend_dir)
		.unwrap_or_else(|e| err!("'{}': {e}", config.frontend_dir))
		.filter_map(|entry| {
			let path = entry.unwrap_or_else(|e| err!("{e}")).path();
			(!path.is_dir()).then_some(path)
		})
		.map(|path| {
			let mut path = std::path::absolute(&path).unwrap();
			path.set_extension("");
			let ns = path.file_stem().unwrap().to_string_lossy();

			rt.run(format!("(import @{} :as {ns})", path.to_str().unwrap()))
				.unwrap_or_else(|e| err!("{e}"));

			ns.to_string()
		})
		.collect::<Vec<_>>();

	#[cfg(debug_assertions)]
	println!("{:?}", frontends);

	if frontends.is_empty() {
		err!("No frontends found in `{}`", config.frontend_dir);
	}

	if args.verbs.is_empty() {
		err!("No verbs provided"); // TODO: better error message
	}

	let janet_args = args.verbs.get(1..)
		.unwrap_or_default()
		.iter().copied()
		.map(Janet::from)
		.collect::<Vec<_>>();

	let call_all_threaded = |rt: &mut janetrs::client::JanetClient, f: &mut Vec<String>, name: &str| {
		rt.add_def(DefOptions::new("pmm-chan", 
			rt.run(format!("(ev/thread-chan {})", f.len()))
				.unwrap_or_else(|e| err!("{e}"))));

		f.iter_mut().for_each(|ns| {
			let mut f = match rt.run(format!("(fn [& x] (ev/spawn-thread (ev/give pmm-chan [\"{ns}\" (apply {ns}/{name} x)])))"))
				.unwrap_or_else(|e| err!("{e}")).unwrap() {
				TaggedJanet::Function(f) => f,
				t => err!("{ns}: expected `function`, got `{}`", t.kind()),
			};

			f.call(&janet_args).unwrap_or_else(|e| err!("{e}"));
		});

		// FIXME: 🤮
		match rt.run(format!("(seq [_ :range [0 {}]] (ev/take pmm-chan))", f.len()))
			.unwrap_or_else(|e| err!("{e}"))
			.unwrap() {
			TaggedJanet::Array(a) => a.into_iter().map(|e| {
				let TaggedJanet::Tuple(t) = e.unwrap() else { unreachable!() };
				let mut t = t.into_iter();
				(t.next().unwrap().to_string(), t.next().unwrap())
			}).collect::<Vec<_>>(),
			t => err!("Expected `array`, got `{}`", t.kind()),
		}
	};

	match *args.verbs.first().unwrap() {
		"list" => {
			call_all_threaded(&mut rt, &mut frontends, "list")
				.into_iter()
				.for_each(|(name, v)| {
					let a = match v.unwrap() {
						TaggedJanet::Array(a) => a,
						t => err!("{name}: expected `array`, got `{}`", t.kind()),
					};

					a.into_iter().for_each(|e| println!("{}: {}", name, e.to_string()))
				})
		},
		"query" => {
			println!("{:?}", call_all_threaded(&mut rt, &mut frontends, "query"));
		},
		_ => todo!(),
	}
}
