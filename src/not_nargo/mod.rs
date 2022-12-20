mod resolver;
use noirc_frontend::graph::CrateType;
pub use resolver::Resolver;

mod toml;

mod errors;

mod git;

fn nargo_crates() -> std::path::PathBuf {
    dirs::home_dir().unwrap().join("nargo")
}

/// Searches for the Nargo.toml file
///
/// XXX: In the end, this should find the root of the project and check
/// for the Nargo.toml file there
/// However, it should only do this after checking the current path
/// This allows the use of workspace settings in the future.
fn find_package_config(current_path: &std::path::Path) -> Result<std::path::PathBuf, errors::CliError> {
    match fm::find_file(current_path, "Nargo", "toml") {
        Some(p) => Ok(p),
        None => {
            Err(errors::CliError::Generic(format!(
                    "cannot find a Nargo.toml in {}",
                    current_path.display()
                )))
        }
    }
}

fn lib_or_bin(current_path: &std::path::Path) -> Result<(std::path::PathBuf, CrateType), errors::CliError> {
    // A library has a lib.nr and a binary has a main.nr
    // You cannot have both.
    let src_path = match fm::find_dir(current_path, "src") {
        Some(path) => path,
        None => {
            return Err(errors::CliError::Generic(format!(
                "cannot find src file in path {}",
                current_path.display()
            )))
        }
    };
    let lib_nr_path = fm::find_file(&src_path, "lib", "nr");
    let bin_nr_path = fm::find_file(&src_path, "main", "nr");
    match (lib_nr_path, bin_nr_path) {
        (Some(_), Some(_)) => Err(errors::CliError::Generic(
            "package cannot contain both a `lib.nr` and a `main.nr`".to_owned(),
        )),
        (None, Some(path)) => Ok((path, CrateType::Binary)),
        (Some(path), None) => Ok((path, CrateType::Library)),
        (None, None) => Err(errors::CliError::Generic(
            "package must contain either a `lib.nr`(Library) or a `main.nr`(Binary).".to_owned(),
        )),
    }
}
