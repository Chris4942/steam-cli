use std::{fmt::Display, sync::mpsc::SendError};

use async_trait::async_trait;

pub trait Logger: Send + Sync {
    fn stdout(&self, str: String) -> Result<(), Error>;
    fn stderr(&self, str: String) -> Result<(), Error>;
}

pub struct FilteringLogger<'a> {
    pub logger: &'a dyn Logger,
    pub verbose: bool,
}

impl<'a> FilteringLogger<'a> {
    pub async fn info(&self, str: String) {
        if let Err(err) = self.logger.stdout(str) {
            eprintln!("{}", err)
        }
    }

    pub async fn error(&self, str: String) {
        if let Err(err) = self.logger.stderr(str) {
            eprintln!("{}", err)
        }
    }

    // TODO: it would be more performant here to pass in a lambda instead having a branch here, but I'm not
    // gonna spend time right now caring about that
    pub async fn trace(&self, str: String) {
        if self.verbose {
            if let Err(err) = self.logger.stderr(str) {
                eprintln!("{}", err)
            }
        }
    }
}

pub enum Error {
    Send(SendError<String>),
}

impl From<SendError<String>> for Error {
    fn from(value: SendError<String>) -> Self {
        return Self::Send(value);
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Send(err) => write!(f, "LoggerError: {}", err),
        }
    }
}
