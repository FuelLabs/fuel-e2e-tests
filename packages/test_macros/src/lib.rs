pub use const_format::formatcp;

pub const TEST_MACRO_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[macro_export]
macro_rules! test_project_build_path {
    ($project_name:literal) => {{
        $crate::formatcp!("{}/../../assets/{}", $crate::TEST_MACRO_DIR, $project_name)
    }};
}
pub fn test_project_build_path(project_name: &str) -> String {
    format!("{}/../../assets/{}", TEST_MACRO_DIR, project_name)
}

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

pub fn test_project_abi_path(project_name: &str) -> String {
    format!(
        "{}/{}-abi.json",
        test_project_build_path(project_name),
        project_name
    )
}

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
