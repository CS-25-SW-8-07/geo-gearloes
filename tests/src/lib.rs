#[cfg(test)]
mod tests {
    use comms::Parquet;
    use geo_types::{Coord, LineString};
    use rand::*;
    use rusty_roads::*;

    fn random_road(id: Id) -> Road {
        Road {
            id,
            geom: LineString::from_iter((0..random_range(10..100)).map(|_| Coord {
                x: random(),
                y: random(),
            })),
            osm_id: random(),
            code: random(),
            direction: Direction::try_from(random_range(0..=2)).unwrap(),
            maxspeed: random(),
            layer: random_range(-3..=3),
            bridge: random(),
            tunnel: random(),
        }
    }

    fn eq<T: PartialEq>((t, q): (T, T)) -> bool {
        t == q
    }

    macro_rules! check {
        ($v: expr, $v2: expr, $e:ident) => {
            assert!($v.$e.iter().zip($v2.$e.iter()).all(eq))
        };
    }

    #[test]
    fn test_roads_parquet() {
        let roads: Roads = ((0..1000).map(random_road)).collect();
        let check = roads.clone();
        let parquet = roads.to_parquet().unwrap();
        let deque = Roads::from_parquet(parquet).unwrap();
        check!(check, deque, id);
        check!(check, deque, osm_id);
        check!(check, deque, geom);
        check!(check, deque, code);
        check!(check, deque, direction);
        check!(check, deque, maxspeed);
        check!(check, deque, layer);
        check!(check, deque, bridge);
        check!(check, deque, tunnel);
    }
}
