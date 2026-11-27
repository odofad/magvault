//! Ship every camera profile in `share/cameras`: generates the list `init` installs
//! (`src/init.rs` STARTER_CAMERAS), so a new `.cam` file there needs no code change.

use std::path::Path;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("share/cameras");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("share/cameras")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".cam"))
        .collect();
    names.sort();
    let mut out = String::from("&[\n");
    for n in &names {
        println!("cargo:rerun-if-changed={}", dir.join(n).display());
        out.push_str(&format!("    ({n:?}, include_str!({:?})),\n", dir.join(n).display().to_string()));
    }
    out.push(']');
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("starter_cameras.rs");
    std::fs::write(dest, out).unwrap();
}
