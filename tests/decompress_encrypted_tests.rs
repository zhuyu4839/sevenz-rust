use sevenz_rust::*;
use std::{fs::read_to_string, io::Read, path::PathBuf};
use tempfile::tempdir;

#[cfg(feature = "aes256")]
#[test]
fn test_decompress_file_with_password() -> anyhow::Result<()> {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/encrypted.7z");
    let temp_dir = tempdir()?;
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("encripted/7zFormat.txt");
    let r = decompress_file_with_password(source_file, target.as_path(), "sevenz-rust".into());
    assert!(r.is_ok());
    assert!(read_to_string(file1_path)?
        .starts_with("7z is the new archive format, providing high compression ratio."));

    Ok(())
}

#[cfg(feature = "aes256")]
#[test]
fn test_decompress_to_memory() -> anyhow::Result<()> {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/7zFormat.7z");
    let source_file = std::fs::File::open(source_file)?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    decompress_with_extract_fn_and_password(
        source_file,
        "",
        "sevenz-rust".into(),
        |entry, reader, _| {
            if !entry.is_directory() {
                if entry.size() > 0 {
                    std::io::copy(reader, &mut cursor)?;
                }
            }
            Ok(true)
        }
    )?;

    cursor.set_position(0);
    let mut buffer = Vec::new();
    cursor.read_to_end(&mut buffer)?;

    let result = String::from_utf8(buffer)?;
    assert!(result.starts_with("7z is the new archive format, providing high compression ratio."));

    Ok(())
}
