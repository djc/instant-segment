from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(
	use_scm_version=True,
	rust_extensions=[RustExtension("instant_segment", path='./instant-segment-py/Cargo.toml', binding=Binding.PyO3)],
) 
