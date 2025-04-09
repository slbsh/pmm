use stabby::{
	boxed::Box,
	slice::Slice,
	string::String,
	option::Option,
	result::Result,
	vec::Vec,
	str::Str,
};

#[stabby::export(canaries)]
pub extern "C" fn init(conf: &'static pmm_abi::Config) -> stabby::dynptr!(Box<dyn pmm_abi::PmImpl>) {
	Box::new(Cargo).into()
}

struct Cargo;

impl pmm_abi::PmImpl for Cargo {
	extern "C" fn name(&self) -> String {
		String::from("pacman")
	}

	extern "C" fn query(&self, item: Str) -> Option<Result<pmm_abi::Package, String>> {
		None.into()
	}

	extern "C" fn list(&self, items: Slice<Str>) -> Option<Result<Vec<pmm_abi::Package>, String>> {
		None.into()
	}
}
