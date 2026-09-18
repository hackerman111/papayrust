pub mod builder;
pub mod error;
pub mod runner;

#[cfg(test)]
mod tests;

pub use builder::{build_open_command, resolve_paper_path, substitute_path, tokenize_template};
pub use error::OpenerError;
pub use runner::{
    open_paper, CommandRunner, ExecutedCommand, MockCommandRunner, ProcessCommandRunner,
};
