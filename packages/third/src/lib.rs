#[macro_export]
macro_rules! test_project_build_path {
    ($project_name:literal) => {{
        use const_format::formatcp;
        formatcp!(
            "{}/compiled_sway_projects/{}",
            env!("OUT_DIR"),
            $project_name
        )
    }};
}
pub fn test_project_build_path(project_name: &str) -> String {
    format!(
        "{}/compiled_sway_projects/{}",
        std::env::var_os("OUT_DIR").unwrap().to_str().unwrap(),
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
