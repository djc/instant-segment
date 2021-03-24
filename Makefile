test-python:
	cargo build --release
	cp target/release/libinstant_segment.dylib instant-segment-py/test/instant_segment.so
	PYTHONPATH=instant-segment-py/test/ python3 -m test
