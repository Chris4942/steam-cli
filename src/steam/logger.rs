pub trait Logger: Send + Sync {
    fn stdout(&self, str: String);
    fn stderr(&self, str: String);
}

pub struct FilteringLogger<'a> {
    pub logger: &'a dyn Logger,
    pub verbose: bool,
}

impl<'a> FilteringLogger<'a> {
    pub async fn info(&self, str: String) {
        self.logger.stdout(str);
    }

    pub async fn error(&self, str: String) {
        self.logger.stderr(str);
    }

    // TODO: it would be more performant here to pass in a lambda instead having a branch here, but I'm not
    // gonna spend time right now caring about that
    pub async fn trace(&self, str: String) {
        if self.verbose {
            self.logger.stderr(str);
        }
    }
}
