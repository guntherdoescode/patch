use std::{env, fs, path::PathBuf};

fn main() {
    let source = image::open("assets/patch.png")
        .expect("failed to load assets/patch.png")
        .into_rgba8();

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is not set"));

    // Match main's Cairo Filter::Nearest rendering, but bake the integer
    // nearest-neighbour enlargement into PNGs so GPUI displays them 1:1.
    for (scale, name) in [(5, "patch-5x.png"), (10, "patch-10x.png")] {
        let enlarged = image::imageops::resize(
            &source,
            source.width() * scale,
            source.height() * scale,
            image::imageops::FilterType::Nearest,
        );
        enlarged
            .save(out.join(name))
            .expect("failed to write nearest-neighbour logo");
    }

    println!("cargo:rerun-if-changed=assets/patch.png");
}
