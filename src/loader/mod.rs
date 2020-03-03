use crate::error::Result;
use crate::hir;
use std::ffi::OsStr;
use std::path::Path;

mod falcon;

pub fn load_program(file_path: &Path, function_name_or_id: Option<&str>) -> Result<hir::Program> {
    match file_path.extension().map(OsStr::to_str).flatten() {
        //Some("muasm") => load_muasm_file(file_path),
        _ => falcon::load_program(file_path, function_name_or_id),
    }
}
