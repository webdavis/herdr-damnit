#[derive(Debug)]
pub struct Finished {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub elapsed: std::time::Duration,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::mpsc::channel;

    struct FakeRunner {
        log: Mutex<Vec<Vec<String>>>,
    }

    impl DamRunner for FakeRunner {
        fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
            self.log.lock().expect("the log").push(argv.to_vec());
            let (sender, results) = channel();
            sender
                .send(Finished {
                    code: Some(0),
                    stdout: "{}".to_string(),
                    stderr: String::new(),
                    elapsed: std::time::Duration::from_millis(9),
                })
                .expect("the receiver is alive");
            Ok(RunningJob {
                cancel: Box::new(|| {}),
                results,
            })
        }
    }

    #[test]
    fn a_runner_records_the_argv_it_was_handed_and_answers_once() {
        let runner = FakeRunner {
            log: Mutex::new(Vec::new()),
        };
        let job = runner
            .spawn(&["status".to_string(), "--json".to_string()])
            .expect("it spawned");

        assert_eq!(
            runner.log.lock().expect("the log").as_slice(),
            [vec!["status".to_string(), "--json".to_string()]]
        );
        assert_eq!(job.results.recv().expect("one result").code, Some(0));
    }
}
