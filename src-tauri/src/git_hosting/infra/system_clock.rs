use std::time::Instant;

use crate::git_hosting::domain::ports::Clock;

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
