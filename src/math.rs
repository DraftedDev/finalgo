/// Returns the Z-score of a slice of values.
#[inline]
pub fn z_score(values: &[f64]) -> Vec<f64> {
    if values.is_empty() {
        return vec![];
    }

    let mean = mean(values);
    let std = std_dev(values, mean);

    if std == 0.0 {
        return vec![0.0; values.len()];
    }

    values.iter().map(|v| (v - mean) / std).collect()
}

/// Computes the mean of a slice of values.
#[inline]
pub fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// Computes the standard deviation of a slice of values.
#[inline]
pub fn std_dev(values: &[f64], mean: f64) -> f64 {
    let variance = values
        .iter()
        .map(|v| {
            let diff = v - mean;
            diff * diff
        })
        .sum::<f64>()
        / values.len() as f64;

    variance.sqrt()
}

/// Returns the last finite value in a slice or [None] if no finite value is found.
#[inline]
pub fn last_finite(values: &[f64]) -> Option<f64> {
    values.iter().rev().copied().find(|v| v.is_finite())
}

/// Returns the mean of the last `n` finite values in a slice or [None] if no finite values are found.
#[inline]
pub fn last_finite_mean(values: &[f64], n: usize) -> Option<f64> {
    let mut sum = 0.0;
    let mut count = 0usize;

    for &v in values.iter().rev() {
        if v.is_finite() {
            sum += v;
            count += 1;
        }

        if count == n {
            break;
        }
    }

    if count == 0 {
        None
    } else {
        Some(sum / count as f64)
    }
}

/// Returns the last non-zero value in a slice or [None] if no non-zero value is found.
#[inline]
pub fn last_non_zero(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .rev()
        .copied()
        .find(|v| v.is_finite() && v.abs() > 1e-12)
}

/// Computes the sigmoid of a value.
#[inline]
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}
