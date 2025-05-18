use janetrs::{Janet, JanetStruct, TaggedJanet};

#[macro_export]
macro_rules! keyword {
	($name:ident) => {
		janetrs::Janet::keyword(janetrs::JanetKeyword::from(stringify!($name)))
	};
}

pub fn prompt_blocking(p: impl std::fmt::Display) -> String {
	print!("{p}");

	use std::io::Write;
	std::io::stdout().flush().unwrap();

	std::io::stdin().lines().next().unwrap().unwrap() // should never fail ??
}


pub trait JanetInto<T> {
	fn janet_into(self) -> T;
}

impl<T> JanetInto<Vec<T>> for Janet 
	where Janet: JanetInto<T> {
	fn janet_into(self) -> Vec<T> {
		match self.unwrap() {
			TaggedJanet::Array(a) => a.into_iter().map(|v| v.janet_into()).collect(),
			TaggedJanet::Tuple(t) => t.into_iter().map(|v| v.janet_into()).collect(),
			t => crate::err!("expected `array`, got `{}`", t.kind()),
		}
	}
}

impl<T> JanetInto<Option<T>> for Janet 
	where Janet: JanetInto<T> {
	fn janet_into(self) -> Option<T> {
		(!self.is_nil()).then(|| self.janet_into())
	}
}

impl JanetInto<String> for Janet {
	fn janet_into(self) -> String {
		self.to_string()
	}
}

impl JanetInto<u64> for Janet {
	fn janet_into(self) -> u64 {
		self.try_into().unwrap_or_else(|e| crate::err!("{e}"))
	}
}

impl<'a> JanetInto<JanetStruct<'a>> for Janet {
	fn janet_into(self) -> JanetStruct<'a> {
		self.try_into().unwrap_or_else(|e| crate::err!("{e}"))
	}
}


impl JanetInto<u8> for Janet {
	fn janet_into(self) -> u8 {
		self.try_into().map_or_else(|e| crate::err!("{e}"), |i: u64| i as u8)
	}
}

impl JanetInto<(u8, u8, u8)> for Janet {
	fn janet_into(self) -> (u8, u8, u8) {
		match self.unwrap() {
			TaggedJanet::Array(a) if a.len() != 3 =>
				crate::err!("expected 3 elements, got {}", a.len()),
			TaggedJanet::Array(a) => (a[0].janet_into(), a[1].janet_into(), a[2].janet_into()),
			TaggedJanet::Tuple(t) if t.len() != 3 =>
				crate::err!("expected 3 elements, got {}", t.len()),
			TaggedJanet::Tuple(t) => (t[0].janet_into(), t[1].janet_into(), t[2].janet_into()),
			TaggedJanet::Number(i) => (i as u8, i as u8, i as u8),
			TaggedJanet::Nil => (u8::MAX, u8::MAX, u8::MAX),
			_ => crate::err!("expected `array` or `tuple`, got `{}`", self.kind()),
		}
	}
}
//
// impl<T: TryFrom<Janet, Error = E>, E: std::fmt::Display> JanetInto<T> for Janet {
// 	fn janet_into(self) -> T {
// 		self.try_into().unwrap_or_else(|e| crate::err!("{e}"))
// 	}
// }
