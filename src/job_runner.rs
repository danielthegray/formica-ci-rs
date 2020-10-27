mod script;

use script::ScriptErrorKind::{NoScriptFound, TooManyScriptsFound};
use std::path::Path;
use std::process::Output;
use std::thread;
use std::time::{Duration, Instant};

const CONFIG: &str = "formica_conf";
pub const CONFIG_INIT_PREFIX: &str = "config_init";
pub const UPDATE: &str = "update";

pub fn initialize() -> Result<(), JobRunnerError> {
    debug!("Initializing Formica CI");
    let config_dir = Path::new(CONFIG);
    let current_dir = Path::new(".").to_path_buf();
    if !config_dir.is_dir() {
        info!("No configuration directory was found... initializing the configuration!");
        let init_script_result = script::find_script(&current_dir, CONFIG_INIT_PREFIX);
        let execution_result = match init_script_result {
            Ok(init_script) => script::execute_script(&current_dir, &init_script),
            Err(no_script) => match no_script.kind {
                NoScriptFound => {
                    return Err(JobRunnerError {
                        kind: JobRunnerErrorKind::NoInitScriptFound,
                    });
                }
                TooManyScriptsFound(duplicate_scripts) => {
                    return Err(JobRunnerError {
                        kind: JobRunnerErrorKind::TooManyInitScriptsFound(duplicate_scripts),
                    });
                }
            },
        };
        let execution_result = execution_result.unwrap();
        if !execution_result.status.success() {
            return Err(JobRunnerError {
                kind: JobRunnerErrorKind::InitScriptExecutionError(execution_result),
            });
        }
    }
    let update_script_execution = update_config();
    let update_script_execution = match update_script_execution {
        Ok(update_result) => update_result.unwrap(),
        Err(update_error) => match update_error.kind {
            NoScriptFound => {
                return Err(JobRunnerError {
                    kind: JobRunnerErrorKind::NoUpdateScriptInsideConfig,
                })
            }
            TooManyScriptsFound(duplicate_scripts) => {
                return Err(JobRunnerError {
                    kind: JobRunnerErrorKind::TooManyUpdateScriptsFound(duplicate_scripts),
                })
            }
        },
    };
    if !update_script_execution.status.success() {
        return Err(JobRunnerError {
            kind: JobRunnerErrorKind::UpdateScriptExecutionError(update_script_execution),
        });
    }

    // TODO : configuration parse
    let job_update_delay = Duration::from_secs(15);

    thread::spawn(move || {
        let last_execution_time = Instant::now();
        loop {
            thread::sleep(Duration::from_secs(1));
            if Instant::now().duration_since(last_execution_time) > job_update_delay {
                match update_config() {
                    Ok(_) => (),
                    Err(update_err) => match update_err.kind {
                        NoScriptFound => warn!("Update script has disappeared!"),
                        TooManyInitScriptsFound => {
                            warn!("Unexpectedly, more than one update script found!")
                        }
                    },
                }
            }
        }
    });
    Ok(())
}

pub fn update_config() -> Result<std::io::Result<Output>, script::ScriptError> {
    let config_dir = Path::new(CONFIG);
    let update_script_result = script::find_script(&config_dir.to_path_buf(), UPDATE);
    match update_script_result {
        Ok(update_script) => Ok(script::execute_script(
            &config_dir.to_path_buf(),
            &update_script,
        )),
        Err(no_update_script) => Err(no_update_script),
    }
}

pub struct JobRunnerError {
    pub kind: JobRunnerErrorKind,
}

pub enum JobRunnerErrorKind {
    NoInitScriptFound,
    TooManyInitScriptsFound(Vec<String>),
    InitScriptExecutionError(Output),
    NoUpdateScriptInsideConfig,
    TooManyUpdateScriptsFound(Vec<String>),
    UpdateScriptExecutionError(Output),
}
