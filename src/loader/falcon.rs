use crate::error::Result;
use crate::hir;
use crate::translator::falcon_to_hir;
use falcon::il;
use falcon::loader::{Elf, Loader};
use std::path::Path;

pub fn load_program(file_path: &Path, function_name_or_id: Option<&str>) -> Result<hir::Program> {
    let program = load_elf(file_path)?;

    if let Some(name_or_id) = function_name_or_id {
        let function = match name_or_id.trim().parse::<usize>() {
            Ok(id) => program.function(id),
            Err(_) => program.function_by_name(name_or_id),
        };

        let function =
            function.ok_or_else(|| format!("Function '{}' could not be found", name_or_id))?;
        falcon_to_hir::translate_function(function)
    } else {
        Err("Falcon loader is currently limited to a single function".into())
    }
}

fn load_elf(file_path: &Path) -> Result<il::Program> {
    let elf = Elf::from_file(file_path)?;
    let result = elf.program_recursive_verbose();
    match result {
        Ok((program, lifting_errors)) => {
            lifting_errors.iter().for_each(|(func, err)| {
                println!(
                    "Lifting {} failed with: {}",
                    func.name().unwrap_or("unknown"),
                    err
                )
            });
            Ok(program)
        }
        Err(_) => Err("Failed to load ELF file!".into()),
    }
}
