use rusty_roads::AnonymityConf;

#[derive(Debug)]
pub enum AnonymityError {
    ConversionError,
}

impl std::fmt::Display for AnonymityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            AnonymityError::ConversionError => write!(f, "Could not convert float to integer"),
        }
    }
}
impl std::error::Error for AnonymityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

/// Will evaluate whether the route is anonymous based on configurations and k_s provided by the current_ks.
/// The input [`current_k`] should only contain k's for rows that are being visited.
pub fn evaluate_route_anonymity(
    anon_conf: &AnonymityConf,
    current_k: impl IntoIterator<Item = impl Into<f64>> + std::marker::Copy,
) -> Result<bool, AnonymityError> {
    let min_per = anon_conf.min_k_percentile;
    let min_k = anon_conf.min_k;

    let count: u64 = current_k
        .into_iter()
        .count()
        .try_into()
        .map_err(|_| AnonymityError::ConversionError)?;
    let below_k: u64 = current_k
        .into_iter()
        .map(|x| x.into())
        .filter(|x| *x < min_k as f64)
        .count()
        .try_into()
        .map_err(|_| AnonymityError::ConversionError)?;

    let percentile = below_k as f64 / count as f64;

    Ok(percentile > min_per)
}


