use std::cell::LazyCell;

use janetrs::{Janet, TaggedJanet};
use colored::Colorize;

mod config;
mod error;
mod args;
mod prelude;
mod backend;
mod world;
mod util;

use world::World;
use util::JanetInto;

// TODO: make absolute :)
const DEFAULT_CONF_PATH: &str = "./config.janet";

pub struct PmmExec {
	rt:        janetrs::client::JanetClient,
	args:      args::Args,
	config:    config::Config,
	frontends: backend::Backends,
	world:     LazyCell<World, Box<dyn FnOnce() -> World>>, // fucking closures
}

impl PmmExec {
	fn init(args: args::Args) -> Self {
		let mut rt = janetrs::client::JanetClient::init()
			.unwrap_or_else(|e| err!("{e}"))
			.load_env_default();

		prelude::append(&mut rt);

		// TODO: maybe have config set a var?
		let config = config::Config::eval_from_file(&mut rt, 
			&std::env::var("PMM_CONFIG")
				.unwrap_or_else(|_| String::from(DEFAULT_CONF_PATH)));

		let world_path = config.world_path.clone();
		Self { 
			frontends: backend::Backends::get_from_dir(&mut rt, &config.backend_dir),
			world:     LazyCell::new(Box::new(move || World::new(&world_path))),
			config, rt, args,
		}
	}

	fn call_all_threaded(&mut self, name: &str, args: impl AsRef<[Janet]>) -> Vec<(backend::Backend, Janet)> {
		self.rt.add_def(janetrs::env::DefOptions::new("pmm-chan", 
			self.rt.run(format!("(ev/thread-chan {})", self.frontends.len()))
				.unwrap_or_else(|e| err!("{e}"))));

		self.frontends.iter().for_each(|ns| {
			let mut f = match self.rt.run(format!("(fn [& x] (ev/spawn-thread (ev/give pmm-chan [\"{ns}\" (apply {ns}/{name} x)])))"))
				.unwrap_or_else(|e| err!("{ns}: {e}")).unwrap() {
				TaggedJanet::Function(f) => f,
				t => err!("{ns}/{name}: expected `function`, got `{}`", t.kind()),
			};

			f.call(&args).unwrap_or_else(|e| err!("{ns}: {e}"));
		});

		// FIXME: 🤮
		match self.rt.run(format!("(seq [_ :range [0 {}]] (ev/take pmm-chan))", self.frontends.len()))
			.unwrap_or_else(|e| err!("{e}"))
			.unwrap() {
			TaggedJanet::Array(a) => a.into_iter().map(|e| {
				let TaggedJanet::Tuple(t) = e.unwrap() else { unreachable!() };
				let mut t = t.into_iter();
				(t.next().unwrap().to_string(), t.next().unwrap())
			}).collect::<Vec<_>>(),
			t => err!("Expected `array`, got `{}`", t.kind()),
		}
	}

	fn sort_by_priority(&self, v: &mut Vec<(backend::Backend, Janet)>) {
		// TODO: diff flag + document
		match self.args.get("bottomup") {
			true => v.sort_unstable_by_key(|(k, _)| self.config.priority.iter().rev()
				.position(|p| p == k).unwrap_or(usize::MIN)),
			false => v.sort_unstable_by_key(|(k, _)| self.config.priority.iter()
				.position(|p| p == k).unwrap_or(usize::MAX)),
		}
	}

	fn cmd(&mut self, act: Action) {
		match act {
			Action::Search(arg) => {
				let mut res = self.call_all_threaded("search", &[Janet::wrap(&*arg)]);
				self.sort_by_priority(&mut res);

				res.into_iter().for_each(|(name, v)| {
					let a = match v.unwrap() {
						TaggedJanet::Array(a) => a,
						t => err!("{name}: expected `array`, got `{}`", t.kind()),
					};

					a.into_iter().for_each(|e| {
						let pkg: backend::Package = e.janet_into();
						// TODO: unique colours per backend
						println!("{}{}{}", name.cyan().bold(), "/".bold(), pkg)
					})
				})
			},
			Action::Info(arg) => {
				self.call_all_threaded("info", &[Janet::wrap(&*arg)])
					.into_iter().for_each(|(name, o)| {
						let p: backend::PackageInfo = o.janet_into();
						println!("Backend:      {}\n{p}", name.cyan().bold())
					});
			},
			_ => todo!(),
		}
	}
}

enum Action {
	Search(String),   // Array<Package>
	Info(String),     // PackageInfo
	Add(Vec<String>), // Array<Package>
	Del(Vec<String>), // Array<Package>
}

fn main() {
	let (args, mut verbs) = args::Args::parse(std::env::args().skip(1));
	args.handle_base_flags(); // TODO: maybe move into Args::parse

	let mut pmm = PmmExec::init(args);

	let action = match verbs.first().map(|s| s.as_str()) {
		Some("search") => Action::Search(verbs[1..].join(" ")),

		Some("info") if verbs.len() > 2 => err!("action `info` expected only one argument"),
		Some("info") if verbs.len() < 2 => err!("action `info` expected an argument"),
		Some("info") => Action::Info(std::mem::take(&mut verbs[1])),

		Some(a) => err!("Unknown action `{a}`"),
		None => err!("No verbs provided"), // TODO: better error message
	};

	pmm.cmd(action);
}
