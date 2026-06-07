pub fn z_score(values: &[f64]) -> Vec<f64> {
    if values.is_empty() {
        return vec![];
    }

    let mean = mean(&values);
    let std = std_dev(&values, mean);

    if std == 0.0 {
        return vec![0.0; values.len()];
    }

    values.iter().map(|v| (v - mean) / std).collect()
}

pub fn rolling_min_max(values: &[f64], period: usize) -> Vec<f64> {
    let mut out = vec![0.0; values.len()];

    for i in period..values.len() {
        let window = &values[i - period..=i];

        let min = window.iter().cloned().fold(f64::INFINITY, f64::min);

        let max = window.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let range = max - min;

        out[i] = if range == 0.0 {
            0.0
        } else {
            (values[i] - min) / range
        };
    }

    out
}

pub fn norm_atr(values: &[f64], atr: &[f64]) -> Vec<f64> {
    assert_eq!(values.len(), atr.len());

    values
        .iter()
        .zip(atr.iter())
        .map(|(v, a)| if *a == 0.0 { 0.0 } else { v / a })
        .collect()
}

pub fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

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

pub fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 { 0.0 } else { a as f64 / b as f64 }
}
