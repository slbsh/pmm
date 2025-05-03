use janetrs::{Janet, JanetArray, JanetKeyword, JanetStruct};

#[janetrs::janet_fn(arity(range(1)))]
fn rsdbg(args: &mut [Janet]) -> Janet {
	args.iter()
		.map(|a| a.unwrap())
		.for_each(|a| println!("{a:?}"));
	Janet::nil()
}

#[janetrs::janet_fn(arity(range(1)))]
fn exec(args: &mut [Janet]) -> Janet {
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

#[janetrs::janet_fn(arity(range(1)))]
fn get_req(args: &mut [Janet]) -> Janet {
	let url = args[0].to_string();

	minreq::get(url)
		.with_header("User-Agent", "Bedrock Linux pmm")
		.send().unwrap_or_else(|e| crate::err!("{e}"))
		.as_str().unwrap_or_else(|e| crate::err!("{e}"))
		.into()
}

#[janetrs::janet_fn(arity(fix(1)))]
fn json_to_janet(args: &mut [Janet]) -> Janet {
	use miniserde::json::{Object, Value, Number};

	let obj = miniserde::json::from_str::<Object>(&args[0].to_string())
		.unwrap_or_else(|e| crate::err!("{e}"));

	fn parse_number(n: Number) -> Janet {
		match n {
			Number::U64(n) => Janet::uint64(n),
			Number::I64(n) => Janet::int64(n),
			Number::F64(n) => Janet::number(n),
		}
	}

	fn parse_val(val: Value) -> Janet {
		match val {
			Value::Null      => Janet::nil(),
			Value::Bool(b)   => Janet::boolean(b),
			Value::Number(n) => parse_number(n),
			Value::String(s) => Janet::string(s.into()),
			Value::Array(a)  => a.into_iter().map(parse_val).collect::<JanetArray>().into(),
			Value::Object(o) => parse_obj(o),
		}
	}

	fn parse_obj(obj: Object) -> Janet {
		obj.into_iter()
			.map(|(k, v)| (Janet::keyword(k.into()), parse_val(v)))
			.collect::<JanetStruct>().into()
	}

	parse_obj(obj)
}

pub fn append(rt: &mut janetrs::client::JanetClient) {
	use janetrs::env::CFunOptions;

	rt.add_c_fn(CFunOptions::new(c"rsdbg", rsdbg_c));
	rt.add_c_fn(CFunOptions::new(c"exec", exec_c));
	rt.add_c_fn(CFunOptions::new(c"get-req", get_req_c));
	rt.add_c_fn(CFunOptions::new(c"json->janet", json_to_janet_c));
}
