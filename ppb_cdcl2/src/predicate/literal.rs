#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Literal {
    bits: usize,
}

impl Literal {
    #[inline(always)]
    pub fn new(variable: usize, value: bool) -> Self {
        debug_assert!(((variable << 1) >> 1) == variable);
        Self {
            bits: (variable << 1) | (value as usize),
        }
    }

    #[inline(always)]
    pub fn from_bits(bits: usize) -> Self {
        Self { bits }
    }

    #[inline(always)]
    pub fn index(&self) -> usize {
        self.bits >> 1
    }

    #[inline(always)]
    pub fn value(&self) -> bool {
        (self.bits & 1) == 1
    }

    #[inline(always)]
    pub fn bits(&self) -> usize {
        self.bits
    }
}

impl std::ops::Not for Literal {
    type Output = Literal;
    #[inline(always)]
    fn not(self) -> Self::Output {
        Literal {
            bits: self.bits ^ 1,
        }
    }
}

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", ["!", ""][self.value() as usize], self.index())
    }
}

impl std::fmt::Debug for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
