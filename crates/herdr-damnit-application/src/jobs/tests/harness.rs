use crate::fakes::{FakeClock, FakeDam};
use crate::{Finished, Jobs, Request, Submitted};

pub(super) fn ok(stdout: &str) -> Finished {
    Finished {
        code: Some(0),
        stdout: stdout.to_string(),
        stderr: String::new(),
    }
}

pub(super) fn jobs_over(dam: &FakeDam) -> Jobs {
    Jobs::new(Box::new(dam.clone()), Box::new(FakeClock::on("2026-09-20")))
}

pub(super) struct Harness {
    pub(super) jobs: Jobs,
    dam: FakeDam,
}

pub(super) fn harness() -> Harness {
    let dam = FakeDam::default();
    Harness {
        jobs: jobs_over(&dam),
        dam,
    }
}

impl Harness {
    pub(super) fn submit(&mut self, request: Request) -> Submitted {
        self.jobs.submit(request)
    }

    pub(super) fn answer(&self, which: usize, finished: Finished) {
        self.dam.answer(which, finished);
    }

    pub(super) fn lines(&self) -> Vec<String> {
        self.dam.lines()
    }
}
