// These reexports are important so that users of this macro don't need to
// explicitly depend on `const_format`.
pub use const_format::formatcp;

pub const TEST_MACRO_DIR: &str = env!("CARGO_MANIFEST_DIR");

// Expands the given project name into the path of the project inside the
// generated `assets` dir.
#[macro_export]
macro_rules! test_project_build_path {
    ($project_name:literal) => {{
        $crate::formatcp!("{}/../../assets/{}", $crate::TEST_MACRO_DIR, $project_name)
    }};
}

/// The same as `test_project_build_path!` just at runtime to sidestep unexpanded macro headaches.
pub fn test_project_build_path(project_name: &str) -> String {
    format!("{}/../../assets/{}", TEST_MACRO_DIR, project_name)
}

/// Expands the given project name into the path of the ABI JSON inside the
/// project folder inside the generated `assets` dir.
#[macro_export]
macro_rules! test_project_abi_path {
    ($project_name:literal) => {{
        $crate::formatcp!(
            "{}/{}-abi.json",
            $crate::test_project_build_path!($project_name),
            $project_name
        )
    }};
}

/// The same as `test_project_abi_path!` just at runtime to sidestep unexpanded macro headaches.
pub fn test_project_abi_path(project_name: &str) -> String {
    format!(
        "{}/{}-abi.json",
        test_project_build_path(project_name),
        project_name
    )
}

/// Expands the given project name into the path of the compiled .bin inside the
/// project folder inside the generated `assets` dir.
#[macro_export]
macro_rules! test_project_bin_path {
    ($project_name:literal) => {{
        $crate::formatcp!(
            "{}/{}.bin",
            $crate::test_project_build_path!($project_name),
            $project_name
        )
    }};
}

/// Expands the given project name into the path of the storage JSON file inside
/// the project folder inside the generated `assets` dir.
#[macro_export]
macro_rules! test_project_storage_path {
    ($project_name:literal) => {{
        $crate::formatcp!(
            "{}/{}-storage_slots.json",
            $crate::test_project_build_path!($project_name),
            $project_name
        )
        .to_string()
    }};
}
