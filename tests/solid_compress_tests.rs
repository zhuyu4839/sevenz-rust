use sevenz_rust::*;
use tempfile::*;

#[cfg(feature = "compress")]
#[test]
fn compress_multi_files_solid() -> anyhow::Result<()> {
    let temp_dir = tempdir()?;
    let folder = temp_dir.path().join("folder");
    std::fs::create_dir(&folder)?;
    let mut files = Vec::with_capacity(100);
    let mut contents = Vec::with_capacity(100);
    for i in 1..=10000 {
        let name = format!("file{}.txt", i);
        let content = format!("file{} with content", i);
        std::fs::write(folder.join(&name), &content)?;
        files.push(name);
        contents.push(content);
    }
    let dest = temp_dir.path().join("folder.7z");

    let mut sz = SevenZWriter::create(&dest)?;
    sz.push_source_path(&folder, |_| true)?;
    sz.finish().expect("compress ok");

    let decompress_dest = temp_dir.path().join("decompress");
    decompress_file(dest, &decompress_dest).expect("decompress ok");
    assert!(decompress_dest.exists());
    for i in 0..files.len() {
        let name = &files[i];
        let content = &contents[i];
        let decompress_file = decompress_dest.join(name);
        assert!(decompress_file.exists());
        assert_eq!(&std::fs::read_to_string(&decompress_file)?, content);
    }

    Ok(())
}

#[cfg(feature = "compress")]
#[test]
fn compress_multi_files_mix_solid_and_non_solid() -> anyhow::Result<()> {
    use std::fs::File;

    let temp_dir = tempdir()?;
    let folder = temp_dir.path().join("folder");
    std::fs::create_dir(&folder)?;
    let mut files = Vec::with_capacity(100);
    let mut contents = Vec::with_capacity(100);
    for i in 1..=100 {
        let name = format!("file{}.txt", i);
        let content = format!("file{} with content", i);
        std::fs::write(folder.join(&name), &content)?;
        files.push(name);
        contents.push(content);
    }
    let dest = temp_dir.path().join("folder.7z");

    let mut sz = SevenZWriter::create(&dest)?;

    // solid compression
    sz.push_source_path(&folder, |_| true)?;

    // non solid compression
    for i in 101..=200 {
        let name = format!("file{}.txt", i);
        let content = format!("file{} with content", i);
        std::fs::write(folder.join(&name), &content)?;
        files.push(name.clone());
        contents.push(content);

        let src = folder.join(&name);
        sz.push_archive_entry(
            SevenZArchiveEntry::from_path(&src, name),
            Some(File::open(src)?),
        )
        .expect("ok");
    }

    sz.finish().expect("compress ok");

    let decompress_dest = temp_dir.path().join("decompress");
    decompress_file(dest, &decompress_dest).expect("decompress ok");
    assert!(decompress_dest.exists());
    for i in 0..files.len() {
        let name = &files[i];
        let content = &contents[i];
        let decompress_file = decompress_dest.join(name);
        assert!(decompress_file.exists());
        assert_eq!(&std::fs::read_to_string(&decompress_file)?, content);
    }

    Ok(())
}
