use std::fs::{self, File};
use std::path::Path;

use super::*;

#[test]
fn run_pack_reftests() {
    let p = Packer::new(Family::AGRV2K);

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

            println!("Running pack reftest: {:?}", example_file);

            let input = File::open(example_file).expect("open input");
            let b = p.pack(input).expect("packing error");
            let mut packed = Vec::new();
            b.save(&mut packed).unwrap();

            let ref_bin = fs::read(ref_bin).expect("read reference bin");

            assert_eq!(packed.len(), ref_bin.len());
            for i in 0..packed.len() {
                assert_eq!(packed[i], ref_bin[i], "File mismatch at byte 0x{i:x}");
            }
        }
    }
}
