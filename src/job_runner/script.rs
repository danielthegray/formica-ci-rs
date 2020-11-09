use std::fs;
use std::iter::FromIterator;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};

pub fn find_script(script_parent: &PathBuf, script_name: &str) -> Result<String, ScriptError> {
    let files_in_cd = fs::read_dir(script_parent).expect(&format!(
        "Error while listing files in {}!",
        script_parent.to_str().unwrap()
    ));

    let scripts = Vec::from_iter(files_in_cd.filter(|file| {
        let potential_script = file.as_ref().unwrap();
        let filetype = potential_script
            .file_type()
            .expect("Error while checking file type");
        filetype.is_file()
            && potential_script
                .file_name()
                .to_str()
                .unwrap_or("")
                .starts_with(script_name)
    }));
    if scripts.is_empty() {
        error!(
            "No script for {} found in {}",
            script_name,
            script_parent.to_str().unwrap()
        );
        return Err(ScriptError {
            kind: ScriptErrorKind::NoScriptFound,
        });
    }
    if scripts.len() > 1 {
        error!(
            "Too many scripts for {} found in {}",
            script_name,
            script_parent.to_str().unwrap()
        );
        return Err(ScriptError {
            kind: ScriptErrorKind::TooManyScriptsFound(Vec::from_iter(scripts.iter().map(
                |script| {
                    script
                        .as_ref()
                        .unwrap()
                        .file_name()
                        .to_str()
                        .unwrap()
                        .to_string()
                },
            ))),
        });
    }
    Ok(scripts
        .get(0)
        .unwrap()
        .as_ref()
        .unwrap()
        .file_name()
        .to_str()
        .unwrap()
        .to_string())
}

fn prepare_process(script_path: &PathBuf, script_file: &str) -> Command {
    let absolute_script_path = script_path
        .join(script_file)
        .canonicalize()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();
    let process: Command;
    if cfg!(target_os = "windows") {
        process = Command::new("cmd");
        process
            .current_dir(script_path)
            .args(&["/C", &absolute_script_path]);
    } else {
        process = Command::new("sh");
        process
            .current_dir(script_path)
            .args(&["-c", &absolute_script_path]);
    }
    process
}

pub fn execute_script(script_path: &PathBuf, script_file: &str) -> std::io::Result<Output> {
    prepare_process(script_path, script_file).output()
}

pub fn spawn_worker_script(script_path: &PathBuf, script_file: &str) -> std::io::Result<Child> {
    prepare_process(script_path, script_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

#[derive(Debug)]
pub struct ScriptError {
    pub kind: ScriptErrorKind,
}

#[derive(Debug)]
pub enum ScriptErrorKind {
    NoScriptFound,
    TooManyScriptsFound(Vec<String>),
}
