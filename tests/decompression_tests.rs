use std::{
    fs::{read, read_to_string, File},
    path::PathBuf,
};

use tempfile::tempdir;

use sevenz_rust::{decompress_file, Archive, BlockDecoder};

#[test]
fn decompress_single_empty_file_unencoded_header() -> anyhow::Result<()> {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/single_empty_file.7z");
    let temp_dir = tempdir()?;
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("empty.txt");

    decompress_file(source_file, target)?;

    assert_eq!(read_to_string(file1_path)?, "");

    Ok(())
}

#[test]
fn decompress_two_empty_files_unencoded_header() -> anyhow::Result<()> {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/two_empty_file.7z");
    let temp_dir = tempdir()?;
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("file1.txt");
    let mut file2_path = target.clone();
    file2_path.push("file2.txt");

    decompress_file(source_file, target)?;

    assert_eq!(read_to_string(file1_path)?, "");
    assert_eq!(read_to_string(file2_path)?, "");

    Ok(())
}

#[test]
fn decompress_lzma_single_file_unencoded_header() -> anyhow::Result<()> {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/single_file_with_content_lzma.7z");
    let temp_dir = tempdir()?;
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("file.txt");

    decompress_file(source_file, target)?;

    assert_eq!(read_to_string(file1_path)?, "this is a file\n");

    Ok(())
}

#[test]
fn decompress_lzma2_bcj_x86_file() -> anyhow::Result<()> {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/decompress_example_lzma2_bcj_x86.7z");
    let temp_dir = tempdir()?;
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("decompress.exe");

    decompress_file(source_file, target)?;

    let mut expected_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    expected_file.push("tests/resources/decompress_x86.exe");

    assert!(
        read(file1_path)? == read(expected_file)?,
        "decompressed files do not match!"
    );

    Ok(())
}

#[test]
fn decompress_lzma_multiple_files_encoded_header() -> anyhow::Result<()> {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/two_files_with_content_lzma.7z");
    let temp_dir = tempdir()?;
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("file1.txt");
    let mut file2_path = target.clone();
    file2_path.push("file2.txt");

    decompress_file(source_file, target)?;

    assert_eq!(read_to_string(file1_path)?, "file one content\n");
    assert_eq!(read_to_string(file2_path)?, "file two content\n");

    Ok(())
}

#[test]
fn decompress_delta_lzma_single_file_unencoded_header() -> anyhow::Result<()> {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/delta.7z");
    let temp_dir = tempdir()?;
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("delta.txt");

    decompress_file(source_file, target)?;

    assert_eq!(read_to_string(file1_path)?, "aaaabbbbcccc");

    Ok(())
}

#[test]
fn decompress_copy_lzma2_single_file() -> anyhow::Result<()> {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/copy.7z");
    let temp_dir = tempdir()?;
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("copy.txt");

    decompress_file(source_file, target)?;

    assert_eq!(read_to_string(file1_path)?, "simple copy encoding");

    Ok(())
}

#[cfg(feature = "bzip2")]
#[test]
fn decompress_bzip2_file() -> anyhow::Result<()> {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/bzip2_file.7z");
    let temp_dir = tempdir()?;
    let target = temp_dir.path().to_path_buf();

    let mut hello_path = target.clone();
    hello_path.push("hello.txt");

    let mut foo_path = target.clone();
    foo_path.push("foo.txt");

    decompress_file(source_file, target)?;

    assert_eq!(read_to_string(hello_path)?, "world\n");
    assert_eq!(read_to_string(foo_path)?, "bar\n");

    Ok(())
}

#[test]
fn test_bcj2() -> anyhow::Result<()> {
    let mut file = File::open("tests/resources/7za433_7zip_lzma2_bcj2.7z")?;
    let file_len = file.metadata()?.len();
    let archive = Archive::read(&mut file, file_len, &[])?;
    for i in 0..archive.folders.len() {
        let fd = BlockDecoder::new(i, &archive, &[], &mut file);
        println!("entry_count:{}", fd.entry_count());
        fd.for_each_entries(&mut |entry, reader| {
            println!("{}=>{:?}", entry.has_stream, entry.name());
            std::io::copy(reader, &mut std::io::sink())?;
            Ok(true)
        })?;
    }

    Ok(())
}

#[test]
fn test_entry_compressed_size() -> anyhow::Result<()> {
    let dir = std::fs::read_dir("tests/resources")?;
    for entry in dir {
        let path = entry?.path();
        if path.to_string_lossy().ends_with("7z") {
            println!("{:?}", path);
            let mut file = File::open(path)?;
            let file_len = file.metadata()?.len();
            let archive = Archive::read(&mut file, file_len, &[])?;
            for i in 0..archive.folders.len() {
                let fi = archive.stream_map.folder_first_file_index[i];
                let file = &archive.files[fi];
                println!(
                    "\t:{}\tsize={}, \tcompressed={}",
                    file.name(),
                    file.size,
                    file.compressed_size
                );
                if file.has_stream && file.size > 0 {
                    assert!(file.compressed_size > 0);
                }
            }
        }
    }

    Ok(())
}
