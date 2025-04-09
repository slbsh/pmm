NAME=$(shell basename $(shell pwd))

default:
	cargo build --release
	cp "target/x86_64-unknown-linux-musl/release/lib$(NAME).so" "../$(NAME).so"
