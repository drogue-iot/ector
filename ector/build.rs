fn main() {
	if std::env::var_os("CARGO_FEATURE_NIGHTLY").is_some() {
		println!("cargo:rustc-cfg=nightly");
	}
}
