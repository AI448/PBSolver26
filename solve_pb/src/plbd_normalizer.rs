use statrs::distribution::ContinuousCDF;

#[derive(Default, Clone)]
pub struct PlbdNormalizer {
    array: Vec<usize>,
}

impl PlbdNormalizer {
    pub fn resize(&mut self, new_n: usize) {
        debug_assert!(self.array.len() == 0 || self.array.len() % 2 == 1);
        let n = (self.array.len() + 1) / 2;
        debug_assert!(n == 0 || (n & (n - 1)) == 0);
        let new_n = 1 << (new_n - 1).bit_width();
        if new_n <= n {
            return;
        }
        let mut new_array = Vec::from_iter(std::iter::repeat_n(0, new_n * 2 - 1));
        for i in (0..n).rev() {
            new_array[new_n - 1 + i] = self.array[n - 1 + i];
        }
        self.array = new_array;
    }

    pub fn observe(&mut self, x: usize) -> f64 {
        debug_assert!(self.array.len() == 0 || self.array.len() % 2 == 1);
        let mut n = (self.array.len() + 1) / 2;
        debug_assert!(n == 0 || (n & (n - 1)) == 0);
        if x >= n {
            self.resize(x + 1);
            debug_assert!(self.array.len() == 0 || self.array.len() % 2 == 1);
            n = (self.array.len() + 1) / 2;
            debug_assert!((n & (n - 1)) == 0);
        }
        assert!(x < n);

        let i = n - 1 + x;
        let mut current = i;
        self.array[current] += 1;
        let mut sum_of_lowers = 0;
        while current != 0 {
            let parent = (current - 1) / 2;
            let sister = if current % 2 == 0 {
                sum_of_lowers += self.array[current - 1];
                current - 1
            } else {
                current + 1
            };
            self.array[parent] = self.array[current] + self.array[sister];
            current = parent;
        }
        let lower_tail_probability =
            (sum_of_lowers as f64 + self.array[i] as f64 / 2.0) / self.array[0] as f64;
        debug_assert!(lower_tail_probability > 0.0);
        debug_assert!(lower_tail_probability < 1.0);
        statrs::distribution::Normal::standard().inverse_cdf(lower_tail_probability)
    }
}
