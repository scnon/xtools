xtools:
	cargo build --release

install:
	cp target/release/xtools /usr/local/bin

clean:
	cargo clean