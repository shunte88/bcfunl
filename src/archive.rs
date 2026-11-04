//! Streaming archive I/O shared by release processing and resource imports.
use crate::{shutdown, workers::file_pool};
use anyhow::{Context, Result, ensure};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub(crate) fn copy(mut input: impl Read, mut output: impl Write) -> Result<u64> {
    let mut buffer = [0; 128 * 1024];
    let mut total = 0;
    loop {
        shutdown::check()?;
        let n = input.read(&mut buffer)?;
        if n == 0 {
            return Ok(total);
        }
        output.write_all(&buffer[..n])?;
        total += n as u64;
    }
}

pub(crate) fn sha256(path: &Path) -> Result<String> {
    struct HashWriter(Sha256);
    impl Write for HashWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut digest = HashWriter(Sha256::new());
    copy(File::open(path)?, &mut digest)?;
    Ok(format!("{:x}", digest.0.finalize()))
}

pub(crate) fn collect(root: &Path) -> Result<Vec<PathBuf>> {
    let mut pending = vec![root.to_owned()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        shutdown::check()?;
        ensure!(
            fs::symlink_metadata(&directory)?.is_dir(),
            "Release directory is not a real directory"
        );
        for entry in fs::read_dir(&directory)
            .with_context(|| format!("Read release directory {}", directory.display()))?
        {
            shutdown::check()?;
            let entry = entry?;
            let kind = entry.file_type()?;
            ensure!(
                !kind.is_symlink(),
                "Symlink refused in release: {}",
                entry.path().display()
            );
            if kind.is_dir() {
                pending.push(entry.path());
            } else {
                ensure!(kind.is_file(), "Nonregular release member refused");
                files.push(entry.path());
            }
        }
    }
    files.sort();
    Ok(files)
}

fn member_path(name: &str) -> Result<PathBuf> {
    ensure!(
        !name.is_empty()
            && !name.contains(['\\', '\0', ':'])
            && !name.starts_with('/')
            && !name.split('/').any(|part| part == ".."),
        "Unsafe archive member: {name:?}"
    );
    let path: PathBuf = Path::new(name)
        .components()
        .filter_map(|part| match part {
            Component::Normal(part) => Some(part),
            _ => None,
        })
        .collect();
    ensure!(!path.as_os_str().is_empty(), "Empty archive member path");
    Ok(path)
}

pub(crate) fn extract(source: &Path, destination: &Path) -> Result<Vec<PathBuf>> {
    shutdown::check()?;
    ensure!(
        fs::symlink_metadata(destination)?.is_dir() && fs::read_dir(destination)?.next().is_none(),
        "Extraction requires an empty real directory"
    );
    let mut archive = zip::ZipArchive::new(File::open(source)?).context("Read ZIP directory")?;
    ensure!(
        archive.len() <= 100_000,
        "Archive contains too many members"
    );
    ensure!(!archive.has_duplicate_names(), "Duplicate archive member");
    let mut paths = HashSet::new();
    let mut members = Vec::new();
    let mut total = 0u64;
    for i in 0..archive.len() {
        shutdown::check()?;
        let member = archive.by_index(i).context("Inspect ZIP member")?;
        let path = member_path(&member.name()?)?;
        ensure!(
            paths.insert(path.clone()),
            "Duplicate archive member: {}",
            path.display()
        );
        let mode = member.unix_mode().unwrap_or(0) & 0o170000;
        ensure!(
            mode == 0 || mode == 0o100000 || (mode == 0o040000 && member.is_dir()),
            "Archive link or special file refused: {}",
            path.display()
        );
        // Bound declared output before creating anything; enforce it again while streaming.
        ensure!(
            member.size() <= 64 * 1024 * 1024 * 1024,
            "Archive member exceeds 64 GiB"
        );
        total = total
            .checked_add(member.size())
            .context("Archive size overflow")?;
        ensure!(
            total <= 1024 * 1024 * 1024 * 1024,
            "Archive output exceeds 1 TiB"
        );
        members.push((i, path, member.is_dir(), member.size()));
    }
    let files: HashSet<_> = members.iter().filter(|m| !m.2).map(|m| &m.1).collect();
    for (_, path, _, _) in &members {
        ensure!(
            !path
                .ancestors()
                .skip(1)
                .any(|parent| files.contains(&parent.to_path_buf())),
            "Archive file/directory collision: {}",
            path.display()
        );
    }
    for (_, path, directory, _) in &members {
        fs::create_dir_all(if *directory {
            destination.join(path)
        } else {
            destination.join(path).parent().unwrap().to_owned()
        })?;
    }
    drop(archive);
    members.retain(|m| !m.2);
    let pool = file_pool()?;
    let chunk_size = members.len().div_ceil(pool.current_num_threads()).max(1);
    let results: Vec<Result<()>> = pool.install(|| {
        members
            .par_chunks(chunk_size)
            .map(|chunk| {
                let mut archive =
                    zip::ZipArchive::new(File::open(source)?).context("Open ZIP worker")?;
                for (index, path, _, size) in chunk {
                    shutdown::check()?;
                    let mut member = archive.by_index(*index)?;
                    let output = destination.join(path);
                    let mut file = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&output)
                        .with_context(|| format!("Create extracted member {}", output.display()))?;
                    let copied = copy((&mut member).take(size + 1), &mut file)
                        .with_context(|| format!("Extract ZIP member {}", path.display()))?;
                    ensure!(
                        copied == *size,
                        "ZIP member size mismatch: {}",
                        path.display()
                    );
                }
                Ok(())
            })
            .collect()
    });
    // All workers have finished before the caller can remove its temporary directory.
    for result in results {
        result?;
    }
    collect(destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::{CompressionMethod, write::SimpleFileOptions};

    fn fixture(entries: &[(&str, &[u8])], method: CompressionMethod) -> Vec<u8> {
        let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, data) in entries {
            archive
                .start_file(
                    *name,
                    SimpleFileOptions::default().compression_method(method),
                )
                .unwrap();
            archive.write_all(data).unwrap();
        }
        archive.finish().unwrap().into_inner()
    }
    fn run(bytes: &[u8]) -> Result<Vec<Vec<u8>>> {
        let work = crate::processing::tempfile::tempdir()?;
        let source = work.path().join("source.zip");
        let output = work.path().join("out");
        fs::write(&source, bytes)?;
        fs::create_dir(&output)?;
        extract(&source, &output)?
            .iter()
            .map(|p| Ok(fs::read(p)?))
            .collect()
    }
    #[test]
    fn extracts_nested_unicode_empty_and_zip64_without_tools_concurrently() {
        for method in [CompressionMethod::Stored, CompressionMethod::Deflated] {
            let bytes = fixture(
                &[
                    ("Disc 1/Rødel.flac", b"audio"),
                    ("cover.jpg", b"cover"),
                    ("empty", b""),
                ],
                method,
            );
            std::thread::scope(|scope| {
                let jobs: Vec<_> = (0..4)
                    .map(|_| scope.spawn(|| run(&bytes).unwrap()))
                    .collect();
                for job in jobs {
                    assert_eq!(
                        job.join().unwrap(),
                        [b"audio".to_vec(), b"cover".to_vec(), Vec::new()]
                    );
                }
            });
        }
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("zip64", SimpleFileOptions::default().large_file(true))
            .unwrap();
        zip.write_all(b"zip64").unwrap();
        assert_eq!(
            run(&zip.finish().unwrap().into_inner()).unwrap(),
            [b"zip64"]
        );
    }
    #[test]
    fn rejects_escape_links_collisions_and_corrupt_data() {
        for name in [
            "../outside",
            "/absolute",
            "C:/windows",
            "a\\b",
            "a/../../outside",
            "./",
        ] {
            assert!(member_path(name).is_err(), "{name}");
        }
        assert!(
            run(&fixture(
                &[("file", b"x"), ("file/child", b"y")],
                CompressionMethod::Stored
            ))
            .is_err()
        );
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.add_symlink("link", "../outside", SimpleFileOptions::default())
            .unwrap();
        assert!(run(&zip.finish().unwrap().into_inner()).is_err());
        let mut bytes = fixture(
            &[("track", b"unique audio bytes")],
            CompressionMethod::Stored,
        );
        let offset = bytes
            .windows(18)
            .position(|b| b == b"unique audio bytes")
            .unwrap();
        bytes[offset] ^= 1;
        assert!(run(&bytes).is_err());
        assert!(run(&bytes[..bytes.len() - 10]).is_err());
        assert!(run(b"not a zip").is_err());
        let mut duplicate = fixture(&[("one", b"a"), ("two", b"b")], CompressionMethod::Stored);
        for i in 0..duplicate.len() - 2 {
            if &duplicate[i..i + 3] == b"two" {
                duplicate[i..i + 3].copy_from_slice(b"one");
            }
        }
        assert!(run(&duplicate).is_err());
    }
    #[test]
    fn preserves_existing_destinations_and_rejects_symlink_walks() {
        let root = crate::processing::tempfile::tempdir().unwrap();
        let source = root.path().join("source.zip");
        let output = root.path().join("output");
        fs::write(
            &source,
            fixture(&[("keep", b"new")], CompressionMethod::Stored),
        )
        .unwrap();
        fs::create_dir(&output).unwrap();
        fs::write(output.join("keep"), b"original").unwrap();
        assert!(extract(&source, &output).is_err());
        assert_eq!(fs::read(output.join("keep")).unwrap(), b"original");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&output, root.path().join("link")).unwrap();
            assert!(collect(&root.path().join("link")).is_err());
        }
    }
    #[test]
    fn hashes_streaming_known_vector() {
        let root = crate::processing::tempfile::tempdir().unwrap();
        let file = root.path().join("sample");
        fs::write(&file, b"abc").unwrap();
        assert_eq!(
            sha256(&file).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
