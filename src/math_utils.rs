pub(crate) fn weighted_average<I>(i: I) -> f32
where
    I: Iterator<Item = (f32, f32)>,
{
    let mut sum = 0.0;
    let mut total_weight = 0.0;
    for (a, b) in i {
        sum += a;
        total_weight += b;
    }
    sum / total_weight
}
