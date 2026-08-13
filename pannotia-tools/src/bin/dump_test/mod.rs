use std::fs::{self, File};
use std::path::Path;

use super::*;

#[test]
fn run_dump_reftests() {
    let cargo_manifest_dir =
        std::env::var_os("CARGO_MANIFEST_DIR").expect("missig CARGO_MANIFEST_DIR");
    let examples_dir = Path::new(&cargo_manifest_dir).join("../examples");

    for example_file in fs::read_dir(&examples_dir).expect("read_dir") {
        let example_file = example_file.expect("get example file").path();
        if example_file.extension().map(|ext| ext == "txt") == Some(true) {
            let mut ref_bin = examples_dir.join("reftests");
            ref_bin.push(example_file.file_stem().unwrap());
            ref_bin.add_extension("bin");
            if !fs::exists(&ref_bin).unwrap_or(false) {
                continue;
            }

            let mut ref_txt = examples_dir.join("reftests");
            ref_txt.push(example_file.file_name().unwrap());
            if !fs::exists(&ref_txt).unwrap_or(false) {
                continue;
            }

            println!("Running dump reftest: {:?}", example_file);

            let input = File::open(ref_bin).expect("open input");
            let b = Bitstream::read(input).expect("bitstream read error");
            let mut explain = Vec::new();
            dump_explain(&b, &mut explain).unwrap();
            let explain = str::from_utf8(&explain).unwrap();

            let ref_txt = fs::read_to_string(ref_txt).expect("read reference text");

            let mut i = 0;
            for (explain_line, ref_line) in explain.lines().zip(ref_txt.lines()) {
                i += 1;
                assert_eq!(explain_line, ref_line, "explain mismatch on line {i}");
            }
        }
    }
}
