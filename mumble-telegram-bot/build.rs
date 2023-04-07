use std::env;
use std::io::Result;
use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;

pub fn copy_to_output(path: &str) -> Result<()> {
    let mut options = CopyOptions::new();
    let mut from_path = Vec::new();
    from_path.push(path);

    let out_path = format!("../target/{}", &env::var("PROFILE").unwrap());

    // Overwrite existing files with same name
    options.overwrite = true;

    copy_items(&from_path, &out_path, &options).expect("Failed to copy items");

    Ok(())
}

fn main() -> Result<()> {
    // Re-runs script if any files in res are changed
    // println!("cargo:rerun-if-changed=config.yaml");
    copy_to_output("./config.yaml", ).expect("Could not copy");
    Ok(())
}
