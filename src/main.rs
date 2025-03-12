use std::ops::Not;
use std::process::{Command, Stdio};
use std::io::Write;
use std::sync::Arc;

mod config;
mod error;
mod args;

use config::{PmConfig, PmImpl, Config};

const DEFAULT_CONF_PATH: &str = "./config.toml";

fn main() {
	let args = args::Args::parse(std::env::args().skip(1));


	let config = {
		let file = std::env::var("PMM_CONFIG")
			.unwrap_or_else(|_| String::from(DEFAULT_CONF_PATH));

		config::deserialize_from::<Config>(&file)
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
			config::deserialize_from::<PmConfig>(&path)
				.unwrap_or_else(|e| err!("'{}': {e}", path.display()))))
		.collect::<Vec<_>>();
	
	#[cfg(debug_assertions)]
	println!("{:#?}", package_managers);

}

fn batch_cmd<'pm>(pms: &'pm [(String, PmConfig)], items: &[&str], conf: &Config, cmd: PmImpl) -> Vec<(&'pm str, Result<String, String>)> {
	let (tx, rx) = std::sync::mpsc::channel();
	let tx = Arc::new(tx);

	pms.iter().for_each(|(name, pmconf)| {
		let _ = std::thread::spawn(move || {
			let mut child = Command::new(&conf.shell)
				.envs(&pmconf.env)
				.envs(&conf.env)
				.env("items", items.join(" "))
				.stdin(Stdio::piped())
				.stdout(Stdio::piped())
				.stderr(Stdio::piped())
				.spawn()
				.map_err(|e| warn!("failed to run action '{cmd:?}' for '{name}': {e}"))?;

			let cmd = pmconf.impls.get(&cmd)
				.ok_or_else(|| warn!("action '{cmd:?}' not implemented for '{name}'"))?;

			child.stdin.as_mut().unwrap().write_all(cmd.as_bytes()).unwrap();
			let out = child.wait_with_output().unwrap();

			let stdout = String::from_utf8(out.stdout)
				.map_err(|_| warn!("invalid utf8 in stdout for '{name}'"))?;

			let stderr = String::from_utf8(out.stderr)
				.map_err(|_| warn!("invalid utf8 in stderr for '{name}'"))?;

			tx.send(
				(name.as_str(), out.status.success()
					.then_some(stdout)
					.ok_or(stderr)))
				.unwrap();
			Ok::<(), ()>(())
		});
	});

	let mut out = Vec::with_capacity(pms.len());
	while out.len() != pms.len() {
		out.push(rx.recv().unwrap());
	}
	out
}
