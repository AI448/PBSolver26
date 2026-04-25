use inari::Interval;

#[inline(always)]
pub fn interval(value: f64) -> Interval {
    if value == f64::INFINITY {
        Interval::try_from((f64::MAX, f64::INFINITY)).unwrap()
    } else if value == f64::NEG_INFINITY {
        Interval::try_from((f64::NEG_INFINITY, f64::MIN)).unwrap()
    } else {
        Interval::try_from((value, value)).unwrap()
    }
}
