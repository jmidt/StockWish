use std::time::Instant;

// Simple struct to gather data about how well the chess bot performs.
pub struct Statistics {
    start: Instant,
    iterations: i32,
}

impl Statistics {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            iterations: 0,
        }
    }

    pub fn increment(&mut self) {
        self.iterations += 1;
    }

    pub fn stop(self) {
        let dur = Instant::now() - self.start;
        println!(
            "Run finished. Considered {} positions in {} seconds",
            self.iterations,
            dur.as_secs_f32()
        )
    }
}
