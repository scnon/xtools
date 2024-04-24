xtools:
	cargo build --release

install:
	make && cp target/release/xtools /usr/local/bin

clean:
	cargo clean