use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=fbs");

    // let it fail, not everyone has flatc
    let _ = flatc_rust::run(flatc_rust::Args {
        inputs: &[Path::new("fbs/data.fbs")],
        out_dir: Path::new("gen_flatbuffers"),
        ..Default::default()
    });
}
