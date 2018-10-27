use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

pub fn get_resource_dir() -> Result<PathBuf, std::io::Error> {
    let current_exe = ::std::env::current_exe()?;
    let current_dir = current_exe.parent().unwrap();
    let test_path = current_dir.join("../../resource/");
    if test_path.exists() {
        return Ok(test_path);
    }
    let test_path = current_dir.join("../../../resource/");
    if test_path.exists() {
        return Ok(test_path);
    }
    let test_path = current_dir.join("resource");
    if test_path.exists() {
        return Ok(test_path);
    }
    Err(Error::new(ErrorKind::NotFound, "Resource dir not found"))
}

pub fn get_test_data_dir() -> Result<PathBuf, std::io::Error> {
    let current_exe = ::std::env::current_exe()?;
    let current_dir = current_exe.parent().unwrap();
    let test_path = current_dir.join("../../testdata/");
    if test_path.exists() {
        return Ok(test_path);
    }
    let test_path = current_dir.join("../../../testdata/");
    if test_path.exists() {
        return Ok(test_path);
    }
    let test_path = current_dir.join("testdata");
    if test_path.exists() {
        return Ok(test_path);
    }
    Err(Error::new(ErrorKind::NotFound, "Testdata dir not found"))
}
