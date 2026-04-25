#[derive(Clone, Copy)]
pub struct ParameterLowerBound {
    value: f64,
}

impl ParameterLowerBound {
    #[inline(always)]
    pub fn new(value: f64) -> Self {
        Self { value }
    }

    #[inline(always)]
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl std::ops::Not for ParameterLowerBound {
    type Output = ParameterUpperBound;
    #[inline(always)]
    fn not(self) -> Self::Output {
        ParameterUpperBound {
            value: self.value.next_down(),
        }
    }
}

impl std::cmp::PartialEq for ParameterLowerBound {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl std::cmp::Eq for ParameterLowerBound {}

impl std::cmp::PartialOrd for ParameterLowerBound {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl std::cmp::Ord for ParameterLowerBound {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.partial_cmp(&other.value).unwrap()
    }
}

impl std::fmt::Display for ParameterLowerBound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "y >= {}", self.value)
    }
}

impl std::fmt::Debug for ParameterLowerBound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct ParameterUpperBound {
    value: f64,
}

impl ParameterUpperBound {
    #[inline(always)]
    pub fn new(value: f64) -> Self {
        Self { value }
    }

    #[inline(always)]
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl std::ops::Not for ParameterUpperBound {
    type Output = ParameterLowerBound;
    #[inline(always)]
    fn not(self) -> Self::Output {
        ParameterLowerBound {
            value: self.value.next_up(),
        }
    }
}

impl std::fmt::Display for ParameterUpperBound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "y <= {}", self.value)
    }
}

impl std::fmt::Debug for ParameterUpperBound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}
