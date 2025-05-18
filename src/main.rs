use std::cell::LazyCell;
use std::ops::Deref;

use janetrs::{Janet, JanetKeyword, TaggedJanet};
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
use backend::{Package, PackageInfo};

// TODO: make absolute :)
const DEFAULT_CONF_PATH: &str = "./config.janet";

pub struct PmmExec {
	rt:       janetrs::client::JanetClient,
	args:     args::Args,
	config:   config::Config,
	backends: backend::Backends,
	term_col: Option<usize>,
	world:    LazyCell<World, Box<dyn FnOnce() -> World>>, // fucking closures
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
			backends: backend::Backends::from_dir(&mut rt, &config.backend_dir),
			world:    LazyCell::new(Box::new(move || World::new(&world_path))),
			// TODO: have an arg for this mayhps??
			term_col: term_size::dimensions().map(|(w, _)| w),
			config, rt, args,
		}
	}

	fn call(&mut self, bname: &str, name: &str, args: impl AsRef<[Janet]>) -> Janet {
		self.rt.env().unwrap().resolve(format!("{}/{name}", bname)).map_or_else(
			|| err!("{}: `{name}` not found", bname),
			|f| match f.unwrap() {
				TaggedJanet::Function(mut f) => f.call(&args).unwrap_or_else(|e| err!("{e}")),
				t => err!("{}: expected `function`, got `{}`", bname, t.kind()),
			},
		)
	}

	fn call_all_threaded(&mut self, name: &str, args: impl AsRef<[Janet]>) -> Vec<(String, Janet)> {
		self.rt.add_def(janetrs::env::DefOptions::new("pmm-chan", 
			self.rt.run(format!("(ev/thread-chan {})", self.backends.len()))
				.unwrap_or_else(|e| err!("{e}"))));

		self.backends.iter().for_each(|b| {
			let ns = &b.name;
			match self.rt.run(format!(
			"(fn [& x] (ev/spawn-thread (ev/give pmm-chan [\"{ns}\" 
				(try (apply {ns}/{name} x) ([err _] [:error err]))])))"))
				.unwrap_or_else(|e| err!("{ns}: {e}")).unwrap() {
				TaggedJanet::Function(mut f) => f.call(&args).unwrap_or_else(|e| err!("{ns}: {e}")),
				t => err!("{ns}/{name}: expected `function`, got `{}`", t.kind()),
			}
		});

		// FIXME: 🤮
		match self.rt.run(format!("(seq [_ :range [0 {}]] (ev/take pmm-chan))", self.backends.len()))
			.unwrap_or_else(|e| err!("{e}"))
			.unwrap() {
			TaggedJanet::Array(a) => a.into_iter().map(|e| {
				let TaggedJanet::Tuple(t) = e.unwrap() else { unreachable!() };
				let mut t = t.into_iter();
				(t.next().unwrap().to_string(), t.next().unwrap())
			}).filter(|(b, e)| match e.unwrap() {
				TaggedJanet::Tuple(t) 
					if t.len() == 2 && t[0] == Janet::keyword(JanetKeyword::new("error")) 
					=> { warn!("{b}: {}", t[1]); false },
				_ => true,
			}).collect::<Vec<_>>(),
			t => err!("Expected `array`, got `{}`", t.kind()),
		}
	}

	fn sort_by_priority<T>(&self, v: &mut Vec<(String, T)>) {
		v.sort_unstable_by_key(|(k, _)| self.config.priority.iter()
			.position(|p| p == k).unwrap_or(usize::MAX))
	}

	fn display_pkg(&self, b: &backend::Backend, pkg: &Package) -> (String, Vec<String>) {
		(format!("{b}{}{} {}{} {}", 
			"/".bold(),
			pkg.name.bold(), 
			pkg.alias.as_ref()
				.map(|a| format!("{} {} ", "as".bold().cyan(), a.bold()))
				.unwrap_or_default(),
			pkg.version.to_string().green().bold(),
			self.world.deref().has_package(&pkg, &b)
				.then(|| format!("{} ", "✓".bright_blue().bold()))
				.unwrap_or_default()),
		match self.term_col {
			Some(max) => {
				let (mut l, r) = pkg.desc.split_whitespace()
					.fold((Vec::new(), String::new()), |(mut acc, mut line), w| {
						if !line.is_empty() && line.len() + w.len() + 1 > max - 5 {
							acc.push(std::mem::take(&mut line));
						}

						if !line.is_empty() {
							line.push(' ');
						}

						line.push_str(w);
						(acc, line)
					});
				l.push(r); l
			},
			None => vec![String::from(pkg.desc.trim())],
		})
	}

	// fn add_pkg(&mut self, )

	fn cmd(&mut self, act: Action) {
		match act {
			Action::Search(args) => {
				let args = args.iter().map(|p| Janet::from(&**p)).collect::<Vec<_>>();
				let mut res = self.call_all_threaded("search", &args);
				self.sort_by_priority(&mut res);

				res.into_iter().for_each(|(b, v)| {
					let a = match v.unwrap() {
						TaggedJanet::Array(a) => a,
						t => err!("{b}: expected `array`, got `{}`", t.kind()),
					};

					let b = self.backends.get(&b);

					a.into_iter().for_each(|e| {
						let pkg: Package = e.janet_into();
						let (header, desc) = self.display_pkg(b, &pkg);
						println!("{header}");
						desc.into_iter().for_each(|l| println!("{l}"));
					})
				})
			},
			Action::Info(arg) => {
				self.call_all_threaded("info", &[Janet::wrap(&*arg)])
					.into_iter().for_each(|(b, o)| {
						let b = self.backends.get(&b);
						let p: PackageInfo = o.janet_into();

						println!("backend:      {b}");
						println!("installed:    {}", 
							if self.world.deref().has_package(&p.pkg, &b)
								{ "yes" } else { "no" });
						println!("{p}");
					});
			},
			Action::Add(arg) => {
				let mut res = self.call_all_threaded("search", &[Janet::wrap(arg)]);
				self.sort_by_priority(&mut res);

				let mut res = res.into_iter().filter_map(|(b, v)| {
					let a = match v.unwrap() {
						TaggedJanet::Array(a) => a,
						t => err!("{b}: expected `array`, got `{}`", t.kind()),
					};

					a.iter().find_map(|p| { 
						let p: Package = p.janet_into(); 
						(p.name == arg).then_some(p)
					}).map(|p| (b, p))
				}).rev().enumerate().collect::<Vec<_>>();
				
				if res.is_empty() {
					err!("Package `{arg}` not found");
				}

				let (backend, pkg) = match res.len() {
					1 => res.pop().map(|(_, x)| x).unwrap(),
					_ => {
						res.iter().for_each(|(i, (b, pkg))| {
							let b = self.backends.get(&b);

							let (header, desc) = self.display_pkg(b, &pkg);
							
							let num = (i + 1).to_string();
							println!("{}{} {header}", " ".repeat(3 - num.len()), num.purple());
							
							desc.into_iter().for_each(|l| println!("    {l}"));
						});

						let n = util::prompt_blocking(":: ".blue().bold());

						let n = n.trim().parse::<usize>()
							.unwrap_or_else(|e| err!("Invalid number `{n}`: {e}"));

						res.get_mut(n - 1).map_or_else(
							|| err!("Package index out `{n}` of bounds"),
							|(_, x)| std::mem::take(x))
					}
				};

				// TODO: document
				match self.args.get("dry") {
					true => self.world.add_package(pkg, &backend),
					false => self.call(&backend, "add", &[Janet::wrap(arg)]), // this should manage world changes
				}

				// does the add func manage world? if yes then that needs to be exposed in the prelude.
				// whiiich would require making PmmExec static, move .call() (or i guess separate funcs for each)
				// into Backend, and then have the funcs stored in Backend as JanetFunction :L
				// AALSO if the whole things is static we can use &'static str for stuff which avoids a WHOLE LOT of cloning
				// ALSOALSO do a small code cleanup when this happens, and maybe come up with a more
				// efficient way of converting between janet and rust values
			},

			Action::Test =>
				self.call_all_threaded("test", &[])
					.into_iter().for_each(|(b, o)| println!("{b}: {o:?}")),

			_ => todo!(),
		}
	}
}

enum Action<'d> {
	Search(&'d [String]),
	Info(&'d str),
	Add(&'d str),
	Del(&'d [String]),
	Test // TODO: remove
}

fn main() {
	let (args, mut verbs) = args::Args::parse(std::env::args().skip(1));
	args.handle_base_flags(); // TODO: maybe move into Args::parse

	let mut pmm = PmmExec::init(args);

	let action = match verbs.first().map(|s| s.as_str()) {
		Some("search") => Action::Search(&verbs[1..]),

		Some("info") if verbs.len() > 2 => err!("action `info` expected only one argument"),
		Some("info") if verbs.len() < 2 => err!("action `info` expected an argument"),
		Some("info") => Action::Info(&verbs[1]),

		Some("add") if verbs.len() < 2 => err!("action `add` expected at least one argument"),
		Some("add") => Action::Add(&verbs[1]),

		Some("test") => Action::Test,

		Some(a) => err!("Unknown action `{a}`"),
		None    => err!("No verbs provided"), // TODO: better error message
	};

	pmm.cmd(action);
}
