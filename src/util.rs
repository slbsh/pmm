use janetrs::{Janet, JanetStruct, TaggedJanet};

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
