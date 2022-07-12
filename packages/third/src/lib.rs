#[macro_export]
macro_rules! test_project_build_path {
    ($project_name:literal) => {{
        use const_format::formatcp;
        formatcp!(
            "{}/../fuel_e2e/tests/test_projects/{}/out/debug",
            env!("CARGO_MANIFEST_DIR"),
            $project_name
        )
    }};
}
pub fn test_project_build_path(project_name: &str) -> String {
    format!(
        "{}/../fuel_e2e/tests/test_projects/{}/out/debug",
        env!("CARGO_MANIFEST_DIR"),
        project_name
    )
}

#[macro_export]
macro_rules! test_project_abi_path {
    ($project_name:literal) => {{
        use const_format::formatcp;
        formatcp!(
            "{}/{}-abi.json",
            ::third::test_project_build_path!($project_name),
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
        use const_format::formatcp;
        formatcp!(
            "{}/{}.bin",
            ::third::test_project_build_path!($project_name),
            $project_name
        )
    }};
}
