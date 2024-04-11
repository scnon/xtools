use std::fs::File;
use std::path::Path;
use zip::read::ZipArchive;

pub fn download_file(url: &str, path: &str) {
    let resp = reqwest::blocking::get(url).expect("request failed");
    let body = resp.bytes().expect("read body failed");
    let mut file = std::fs::File::create(path).expect("create file failed");
    std::io::Write::write_all(&mut file, &body).expect("write file failed");
}

pub(crate) fn unzip_file(file: &str, out: &str) {
    let file = File::open(file).expect("failed to open zip file");
    let mut archive = ZipArchive::new(file).expect("failed to open zip archive");

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .expect("failed to get file from archive");
        let outpath = Path::new(out).join(file.name());

        if file.is_dir() {
            std::fs::create_dir_all(&outpath).expect("failed to create directory");
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(&p).expect("failed to create directory");
                }
            }
            let mut outfile = File::create(&outpath).expect("failed to create file");
            std::io::copy(&mut file, &mut outfile).expect("failed to extract file");
        }
    }
}

pub(crate) fn delete_file(as_str: &str) {
    std::fs::remove_file(as_str).expect("failed to delete file");
}
