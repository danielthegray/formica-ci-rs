mod script;

use script::ScriptErrorKind::{NoScriptFound, TooManyScriptsFound};
use std::env;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::thread;
use std::time::{Duration, Instant};
use walkdir::{DirEntry, WalkDir};

const CONFIG: &str = "formica_conf";
pub const CONFIG_INIT_PREFIX: &str = "config_init";
pub const UPDATE: &str = "update";
pub const AGENT_INIT: &str = "agent_init";

pub fn initialize() -> Result<(), InitError> {
    debug!("Initializing Formica CI");
    let config_dir = Path::new(CONFIG);
    if !config_dir.is_dir() {
        info!("No configuration directory was found... initializing the configuration!");
        config_fetch()?;
    }
    initial_config_update()?;
    launch_background_updater();
    start_orchestrator()?;

    Ok(())
}

fn update_config() -> Result<std::io::Result<Output>, script::ScriptError> {
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

fn start_orchestrator() -> Result<(), InitError> {
    let jobs = find_jobs().unwrap();
    for job in jobs.iter() {
        println!("FOUND JOB AT {}", job.root_folder.to_str().unwrap());
    }
    launch_job_queue_poller();
    Ok(())
}

fn config_fetch() -> Result<(), InitError> {
    let current_dir = env::current_dir().expect("Failed to detect current directory!");
    let init_script_result = script::find_script(&current_dir, CONFIG_INIT_PREFIX);
    let execution_result = match init_script_result {
        Ok(init_script) => script::execute_script(&current_dir, &init_script),
        Err(no_script) => match no_script.kind {
            NoScriptFound => {
                return Err(InitError {
                    kind: InitErrorKind::NoInitScriptFound,
                });
            }
            TooManyScriptsFound(duplicate_scripts) => {
                return Err(InitError {
                    kind: InitErrorKind::TooManyInitScriptsFound(duplicate_scripts),
                });
            }
        },
    };
    let execution_result = execution_result.unwrap();
    if !execution_result.status.success() {
        return Err(InitError {
            kind: InitErrorKind::InitScriptExecutionError(execution_result),
        });
    }
    Ok(())
}

fn initial_config_update() -> Result<(), InitError> {
    let update_script_execution = update_config();
    let update_script_execution = match update_script_execution {
        Ok(update_result) => update_result.unwrap(),
        Err(update_error) => match update_error.kind {
            NoScriptFound => {
                return Err(InitError {
                    kind: InitErrorKind::NoUpdateScriptInsideConfig,
                })
            }
            TooManyScriptsFound(duplicate_scripts) => {
                return Err(InitError {
                    kind: InitErrorKind::TooManyUpdateScriptsFound(duplicate_scripts),
                })
            }
        },
    };
    if !update_script_execution.status.success() {
        return Err(InitError {
            kind: InitErrorKind::UpdateScriptExecutionError(update_script_execution),
        });
    }
    Ok(())
}

fn is_agent_init_script(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
        && entry
            .file_name()
            .to_str()
            .map(|name| name.starts_with(AGENT_INIT))
            .unwrap_or(false)
}

fn find_jobs() -> Result<Vec<Job>, JobRunnerError> {
    let jobs = Vec::from_iter(
        WalkDir::new(CONFIG)
            .follow_links(true)
            .into_iter()
            .filter_map(|f| f.ok())
            .filter(|file| is_agent_init_script(file))
            .map(|agent_init_script| {
                let job_folder = agent_init_script.path().parent().unwrap().to_path_buf();
                Job {
                    name: String::from("a job"),
                    root_folder: job_folder,
                }
            }),
    );
    if jobs.is_empty() {
        return Err(JobRunnerError {
            kind: JobRunnerErrorKind::NoJobsFound,
        });
    }

    Ok(jobs)
}

fn launch_job_queue_poller() {
    // TODO: create queue folder if missing
    // poll queue folder for files
}

fn launch_background_updater() {
    // TODO : configuration parse
    let job_update_delay = Duration::from_secs(5 * 60);

    thread::spawn(move || {
        let last_execution_time = Instant::now();
        loop {
            thread::sleep(Duration::from_secs(1));
            if Instant::now().duration_since(last_execution_time) > job_update_delay {
                match update_config() {
                    Ok(_) => (),
                    Err(update_err) => match update_err.kind {
                        NoScriptFound => warn!("Update script has disappeared!"),
                        TooManyScriptsFound(_) => {
                            warn!("Unexpectedly, more than one update script found!")
                        }
                    },
                }
            }
        }
    });
}

pub struct Job {
    name: String,
    root_folder: PathBuf,
    //steps: Vec<PathBuf>
}

#[derive(Debug)]
pub struct JobRunnerError {
    pub kind: JobRunnerErrorKind,
}

#[derive(Debug)]
pub enum JobRunnerErrorKind {
    NoJobsFound,
}

#[derive(Debug)]
pub struct InitError {
    pub kind: InitErrorKind,
}

#[derive(Debug)]
pub enum InitErrorKind {
    NoInitScriptFound,
    TooManyInitScriptsFound(Vec<String>),
    InitScriptExecutionError(Output),
    NoUpdateScriptInsideConfig,
    TooManyUpdateScriptsFound(Vec<String>),
    UpdateScriptExecutionError(Output),
}
