use std::ffi::OsString;
use std::io::Result;
use std::{env, fs};

fn main() -> Result<()> {
	println!("cargo:rerun-if-changed=src/protobuf/");

	let mut conf = prost_build::Config::new();
	conf.include_file("api_pf.rs");
	conf.enum_attribute(
		"MsgBody.body",
		"#[::spire_enum::prelude::delegated_enum(impl_variants_into_enum)]",
	);

	let mut v: Vec<OsString> = Vec::new();
	for entry in fs::read_dir("src/protobuf/")? {
		let entry = entry?;
		let path = entry.path();

		if let Some(s) = path.extension() {
			if s == "proto" {
				let mut tmp = OsString::from("src/protobuf/");
				tmp.push(path.file_name().unwrap());
				v.push(tmp);
			}
		}
	}

	println!("{:?}", v);

	conf.compile_protos(v.as_slice(), &["src", env::var("PATH").unwrap().as_str()])?;

	Ok(())
}
