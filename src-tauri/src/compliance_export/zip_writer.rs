// WP-60: builds an in-memory .zip from named byte blobs (documents + their
// detached .sig signature files + the public key certificate). Uses the
// pure-Rust `deflate` backend (no system zlib/bzip2 dependency).
use std::io::Write;
use zip::write::SimpleFileOptions;

pub fn build_zip(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut writer = zip::ZipWriter::new(cursor);
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, contents) in files {
            writer.start_file(name, options).map_err(|e| e.to_string())?;
            writer.write_all(contents).map_err(|e| e.to_string())?;
        }
        writer.finish().map_err(|e| e.to_string())?;
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_zip_produces_a_readable_archive_with_all_files() {
        let files = vec![
            ("cover.json".to_string(), b"{\"a\":1}".to_vec()),
            ("cover.json.sig".to_string(), b"signature-bytes".to_vec()),
        ];
        let zip_bytes = build_zip(&files).unwrap();
        let reader = std::io::Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(reader).unwrap();
        assert_eq!(archive.len(), 2);
        let mut names: Vec<String> = (0..archive.len()).map(|i| archive.by_index(i).unwrap().name().to_string()).collect();
        names.sort();
        assert_eq!(names, vec!["cover.json".to_string(), "cover.json.sig".to_string()]);
    }
}
