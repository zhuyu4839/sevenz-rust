use sevenz_rust::*;
use tempfile::*;

#[cfg(feature = "compress")]
#[test]
fn compress_empty_file() -> anyhow::Result<()> {
    let temp_dir = tempdir()?;
    let source = temp_dir.path().join("empty.txt");
    std::fs::File::create(&source)?;
    let dest = temp_dir.path().join("empty.7z");
    compress_to_path(source, &dest).expect("compress ok");

    let decompress_dest = temp_dir.path().join("decompress");
    decompress_file(dest, &decompress_dest).expect("decompress ok");
    assert!(decompress_dest.exists());
    let decompress_file = decompress_dest.join("empty.txt");
    assert!(decompress_file.exists());

    assert_eq!(std::fs::read_to_string(&decompress_file)?, "");

    Ok(())
}

#[cfg(feature = "compress")]
#[test]
fn compress_one_file_with_content() -> anyhow::Result<()> {
    let temp_dir = tempdir()?;
    let source = temp_dir.path().join("file1.txt");
    std::fs::write(&source, "file1 with content")?;
    let dest = temp_dir.path().join("file1.7z");
    compress_to_path(source, &dest).expect("compress ok");

    let decompress_dest = temp_dir.path().join("decompress");
    decompress_file(dest, &decompress_dest).expect("decompress ok");
    assert!(decompress_dest.exists());
    let decompress_file = decompress_dest.join("file1.txt");
    assert!(decompress_file.exists());

    assert_eq!(
        std::fs::read_to_string(&decompress_file)?,
        "file1 with content"
    );

    Ok(())
}

#[cfg(feature = "compress")]
#[test]
fn compress_empty_folder() -> anyhow::Result<()> {
    let temp_dir = tempdir()?;
    let folder = temp_dir.path().join("folder");
    std::fs::create_dir(&folder)?;
    let dest = temp_dir.path().join("folder.7z");
    compress_to_path(&folder, &dest).expect("compress ok");

    let decompress_dest = temp_dir.path().join("decompress");
    decompress_file(dest, &decompress_dest).expect("decompress ok");
    assert!(decompress_dest.exists());
    assert!(decompress_dest.read_dir()?.next().is_none());

    Ok(())
}

#[cfg(feature = "compress")]
#[test]
fn compress_folder_with_one_file() -> anyhow::Result<()> {
    let temp_dir = tempdir()?;
    let folder = temp_dir.path().join("folder");
    std::fs::create_dir(&folder)?;
    std::fs::write(folder.join("file1.txt"), "file1 with content")?;
    let dest = temp_dir.path().join("folder.7z");
    compress_to_path(&folder, &dest).expect("compress ok");

    let decompress_dest = temp_dir.path().join("decompress");
    decompress_file(dest, &decompress_dest).expect("decompress ok");
    assert!(decompress_dest.exists());
    let decompress_file = decompress_dest.join("file1.txt");
    assert!(decompress_file.exists());

    assert_eq!(
        std::fs::read_to_string(&decompress_file)?,
        "file1 with content"
    );

    Ok(())
}

#[cfg(feature = "compress")]
#[test]
fn compress_folder_with_multi_file() -> anyhow::Result<()> {
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
    compress_to_path(&folder, &dest).expect("compress ok");

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
fn compress_folder_with_nested_folder() -> anyhow::Result<()> {
    let temp_dir = tempdir()?;
    let folder = temp_dir.path().join("folder");
    let inner = folder.join("a/b/c");
    std::fs::create_dir_all(&inner)?;
    std::fs::write(inner.join("file1.txt"), "file1 with content")?;
    let dest = temp_dir.path().join("folder.7z");
    compress_to_path(&folder, &dest).expect("compress ok");

    let decompress_dest = temp_dir.path().join("decompress");
    decompress_file(dest, &decompress_dest).expect("decompress ok");
    assert!(decompress_dest.exists());
    let decompress_file = decompress_dest.join("a/b/c/file1.txt");
    assert!(decompress_file.exists());

    assert_eq!(
        std::fs::read_to_string(&decompress_file)?,
        "file1 with content"
    );

    Ok(())
}

#[cfg(all(feature = "compress", feature = "aes"))]
#[test]
fn compress_one_file_with_random_content_encrypted() -> anyhow::Result<()> {
    use rand::Rng;
    for _ in 0..10 {
        let temp_dir = tempdir()?;
        let source = temp_dir.path().join("file1.txt");
        let mut rng = rand::thread_rng();
        let mut content = String::with_capacity(rng.gen_range(1..10240));

        for _ in 0..content.capacity() {
            let c = rng.gen_range(' '..'~');
            content.push(c);
        }
        std::fs::write(&source, &content)?;
        let dest = temp_dir.path().join("file1.7z");

        compress_to_path_encrypted(source, &dest, "rust".into()).expect("compress ok");

        let decompress_dest = temp_dir.path().join("decompress");
        decompress_file_with_password(dest, &decompress_dest, "rust".into())
            .expect("decompress ok");
        assert!(decompress_dest.exists());
        let decompress_file = decompress_dest.join("file1.txt");
        assert!(decompress_file.exists());

        assert_eq!(std::fs::read_to_string(&decompress_file)?, content);
    }

    Ok(())
}
