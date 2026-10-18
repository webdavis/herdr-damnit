#[derive(Debug)]
pub struct Finished {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug)]
pub enum SpawnError {
    NotFound,
    Io(String),
}

pub struct RunningJob {
    pub cancel: Box<dyn Fn() + Send + Sync>,
    pub results: std::sync::mpsc::Receiver<Finished>,
}

pub trait DamRunner: Send + Sync {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError>;
}

pub trait Herdr: Send + Sync {
    fn call(&self, args: &[&str]) -> Result<String, String>;
}

pub trait Clock: Send + Sync {
    fn today(&self) -> herdr_damnit_domain::Date;
    fn now(&self) -> std::time::Instant;
}
