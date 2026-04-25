#[derive(Clone)]
pub struct PlbdScoreAverage {
    ring_buffer: RingBuffer,
    n: usize,
    sum: f64,
}

impl PlbdScoreAverage {
    pub fn new(short_term_interval: usize) -> Self {
        Self {
            ring_buffer: RingBuffer::new(short_term_interval),
            n: 0,
            sum: 0.0,
        }
    }

    pub fn add_score(&mut self, score: f64) {
        self.ring_buffer.push(score);
        self.n += 1;
        self.sum += score;
    }

    pub fn short_term_average(&self) -> f64 {
        self.ring_buffer.average()
    }

    pub fn short_term_lower_bound(&self) -> f64 {
        self.ring_buffer.lower_bound()
    }

    pub fn long_term_average(&self) -> f64 {
        self.sum / (self.n as f64)
    }

    pub fn reset(&mut self) {
        self.ring_buffer.clear();
        self.n = 0;
        self.sum = 0.0;
    }
}

#[derive(Clone)]
pub struct RingBuffer {
    values: Vec<f64>,
    current: usize,
    sum: f64,
}

impl RingBuffer {
    pub fn new(len: usize) -> Self {
        Self {
            values: std::iter::repeat_n(0.0, len).collect(),
            current: 0,
            sum: 0.0,
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn push(&mut self, value: f64) {
        self.sum = self.sum - self.values[self.current] + value;
        self.values[self.current] = value;
        self.current = (self.current + 1) % self.values.len();
    }

    pub fn sum(&self) -> f64 {
        self.sum
    }

    pub fn average(&self) -> f64 {
        self.sum / self.values.len() as f64
    }

    pub fn lower_bound(&self) -> f64 {
        self.sum / self.values.len() as f64 - 1.0 / (self.values.len() as f64).sqrt()
    }

    pub fn clear(&mut self) {
        self.values.fill(0.0);
        self.sum = 0.0;
    }
}
