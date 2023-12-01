ifeq ($(shell uname), Darwin)
	PY_EXT := dylib
else
	PY_EXT := so
endif

test-python:
	cargo build --release
	cp target/release/libinstant_segment.$(PY_EXT) instant-segment-py/test/instant_segment.so
	PYTHONPATH=instant-segment-py/test/ python3 -m test
