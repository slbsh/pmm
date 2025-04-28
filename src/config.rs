use std::collections::HashMap;

#[derive(Debug)]
pub struct Config {
	pub frontend_dir: String,
	pub world_path:   String,
	pub env:          HashMap<String, String>,
}

#[derive(Debug)]
pub struct Frontend<'f> {
	pub ns:    String,
	pub query: janetrs::JanetFunction<'f>,
}
