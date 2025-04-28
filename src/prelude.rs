use janetrs::env::CFunOptions;
use janetrs::{Janet, JanetKeyword, JanetStruct};

#[janetrs::janet_fn(arity(range(1)))]
fn rsdbg(args: &mut [Janet]) -> Janet {
	args.iter()
		.map(|a| a.unwrap())
		.for_each(|a| println!("{a:?}"));
	Janet::nil()
}

#[janetrs::janet_fn]
fn exec(args: &mut [Janet]) -> Janet {
	if args.is_empty() {
		return Janet::nil();
	}

	let cmd = std::process::Command::new(args[0].to_string())
		.args(args.get(1..).unwrap_or_default().iter().map(ToString::to_string).collect::<Vec<_>>())
		.output().unwrap_or_else(|e| crate::err!("Failed to execute command: {e}"));

	let stdout = str::from_utf8(&cmd.stdout)
		.unwrap_or_else(|e| crate::err!("{e}"));
	let stderr = str::from_utf8(&cmd.stderr)
		.unwrap_or_else(|e| crate::err!("{e}"));

	Janet::structs(JanetStruct::builder(3)
		.put(JanetKeyword::new("status"), cmd.status.code().unwrap_or_default())
		.put(JanetKeyword::new("stdout"), stdout)
		.put(JanetKeyword::new("stderr"), stderr)
		.finalize())
}

pub fn append(rt: &mut janetrs::client::JanetClient) {
	rt.add_c_fn(CFunOptions::new(c"rsdbg", rsdbg_c));
	rt.add_c_fn(CFunOptions::new(c"exec", exec_c));
}
