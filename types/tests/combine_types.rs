use std::ffi::OsStr;
use std::fs::{read_dir, remove_file, File};
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;

const OUTPUT_DIR: &str = "./bindings";
const OUTPUT_FILE: &str = "index.d.ts";

#[test]
fn create_index() -> Result<(), Box<dyn std::error::Error>> {
    let mut tsfile = File::create(format!("{}/{}", OUTPUT_DIR, OUTPUT_FILE))?;
    let mut tscode = "".to_owned();

    tscode.push_str(
        "/* tslint:disable */\n\n\
        /* WARNING: This file is automatically generated */\n\
        /* DO NOT CHANGE IT MANUALLY */\n\n",
    );

    let mut paths: Vec<_> = read_dir(OUTPUT_DIR)?
        .into_iter()
        .filter(std::result::Result::is_ok)
        .map(|r| r.unwrap().path())
        .filter(|r| r.extension() == Some(OsStr::new("ts")))
        .filter(|r| r.file_name() != Some(OsStr::new(&OUTPUT_FILE)))
        .collect();

    paths.sort();

    for entry in paths {
        for line in BufReader::new(File::open(&entry)?).lines().flatten() {
            if !line.starts_with("import") {
                tscode.push_str(&line);
                tscode.push('\n');
            }
        }

        remove_file(Path::new(&entry))?;
    }

    Ok(tsfile.write_all(tscode.as_bytes())?)
}
