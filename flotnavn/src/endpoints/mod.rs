pub mod anon;


pub fn get_bbox(query: &std::collections::HashMap<String, String>) -> ((f64, f64), (f64, f64)) {
    let get_coord = |key: &str| {
        query
            .get(key)
            .and_then(|val| val.parse::<f64>().ok())
            .unwrap_or_default()
    };

    let lon1 = get_coord("lon1");
    let lat1 = get_coord("lat1");
    let lon2 = get_coord("lon2");
    let lat2 = get_coord("lat2");

    ((lon1, lat1), (lon2, lat2))
}
