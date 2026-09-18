use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::db::Paper;
use crate::opener::builder::{build_open_command, resolve_paper_path};
use crate::opener::error::OpenerError;

/// Trait abstracting process execution for opening external viewers.
pub trait CommandRunner: Send + Sync {
    fn spawn(&self, program: &str, args: &[String]) -> Result<(), OpenerError>;
}

impl<T: CommandRunner + ?Sized> CommandRunner for &T {
    fn spawn(&self, program: &str, args: &[String]) -> Result<(), OpenerError> {
        (**self).spawn(program, args)
    }
}

impl<T: CommandRunner + ?Sized> CommandRunner for Arc<T> {
    fn spawn(&self, program: &str, args: &[String]) -> Result<(), OpenerError> {
        (**self).spawn(program, args)
    }
}

impl<T: CommandRunner + ?Sized> CommandRunner for Box<T> {
    fn spawn(&self, program: &str, args: &[String]) -> Result<(), OpenerError> {
        (**self).spawn(program, args)
    }
}

/// Real command runner that spawns a child process in the background.
#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessCommandRunner;

impl CommandRunner for ProcessCommandRunner {
    fn spawn(&self, program: &str, args: &[String]) -> Result<(), OpenerError> {
        std::process::Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|err| OpenerError::LaunchFailed(format!("{program}: {err}")))?;
        Ok(())
    }
}

/// Recorded command invocation consisting of program binary name and arguments list.
pub type ExecutedCommand = (String, Vec<String>);
type CommandHistory = Arc<Mutex<Vec<ExecutedCommand>>>;

/// Mock command runner for deterministic testing. Records all spawned commands
/// in a thread-safe list and allows simulating launch failures.
#[derive(Debug, Clone, Default)]
pub struct MockCommandRunner {
    executed: CommandHistory,
    fail_with: Arc<Mutex<Option<String>>>,
}

impl MockCommandRunner {
    pub fn new() -> Self {
        Self {
            executed: Arc::new(Mutex::new(Vec::new())),
            fail_with: Arc::new(Mutex::new(None)),
        }
    }

    pub fn executed_commands(&self) -> Vec<ExecutedCommand> {
        self.executed.lock().unwrap().clone()
    }

    pub fn last_command(&self) -> Option<ExecutedCommand> {
        self.executed.lock().unwrap().last().cloned()
    }

    pub fn command_count(&self) -> usize {
        self.executed.lock().unwrap().len()
    }

    pub fn clear(&self) {
        self.executed.lock().unwrap().clear();
    }

    pub fn set_fail_with(&self, msg: impl Into<String>) {
        *self.fail_with.lock().unwrap() = Some(msg.into());
    }

    pub fn clear_fail(&self) {
        *self.fail_with.lock().unwrap() = None;
    }
}

impl CommandRunner for MockCommandRunner {
    fn spawn(&self, program: &str, args: &[String]) -> Result<(), OpenerError> {
        if let Some(ref err) = *self.fail_with.lock().unwrap() {
            return Err(OpenerError::LaunchFailed(err.clone()));
        }
        self.executed
            .lock()
            .unwrap()
            .push((program.to_string(), args.to_vec()));
        Ok(())
    }
}

/// Opens the given paper using the configured viewer and command runner.
///
/// If `page` is Some, the page open template is used.
/// If `page` is None, the base viewer configuration (or page open template fallback) is used.
pub fn open_paper(
    paper: &Paper,
    page: Option<u32>,
    config: &Config,
    runner: &impl CommandRunner,
) -> Result<(), OpenerError> {
    if let Some(0) = page {
        return Err(OpenerError::InvalidPage(0));
    }

    let file_path = resolve_paper_path(paper, config)?;

    let template = match page {
        Some(_) => &config.pdf.page_open_template,
        None => {
            if !config.pdf.viewer.trim().is_empty() {
                &config.pdf.viewer
            } else {
                &config.pdf.page_open_template
            }
        }
    };

    let (program, args) = build_open_command(template, &file_path, page)?;
    runner.spawn(&program, &args)
}
