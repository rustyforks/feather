use anyhow::Context;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use walkdir::WalkDir;

mod backend;
mod frontend;
mod generated;
mod model;

pub fn load_directory(dir: impl AsRef<Path>, target_dir: &str) -> anyhow::Result<()> {
    let dir = dir.as_ref();

    generated::write(&format!("{}/generated", dir.display()))
        .context("failed to write generated data")?;

    let mut files = vec![];
    for entry in WalkDir::new(dir) {
        let entry = entry.context("failed to open DirEntry")?;

        let mut name = entry
            .path()
            .file_stem()
            .ok_or_else(|| {
                anyhow::anyhow!("failed to get fiel stem for `{}`", entry.path().display())
            })?
            .to_string_lossy();
        let mut contents = String::new();
        let mut file = File::open(entry.path())
            .with_context(|| format!("failed to open file `{}`", entry.path().to_string_lossy()))?;

        if file.metadata()?.is_dir() {
            continue;
        }

        file.read_to_string(&mut contents)
            .with_context(|| format!("failed to read file `{}`", entry.path().to_string_lossy()))?;

        files.push(frontend::DataFile {
            name: name.to_mut().clone(),
            contents,
        });
    }

    let data = frontend::from_slice(&files).context("failed to load data")?;
    backend::generate(target_dir, &data).context("failed to generate code for data")?;

    println!("{:#?}", data);

    Ok(())
}
