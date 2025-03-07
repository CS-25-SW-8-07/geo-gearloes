use geo_types::{CoordNum, LineString};
use rstar::primitives::GeomWithData;
use rstar::{PointDistance, RTree, RTreeObject};

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Forward,
    Backward,
    Bidirectional,
}

pub type Id = usize;

#[derive(Debug, Clone, Copy)]
pub struct RoadKey(pub Id);

pub struct Road<T: CoordNum> {
    pub id: Id,
    pub geom: LineString<T>,
    pub osm_id: u64,
    pub code: u16,
    pub direction: Direction,
    pub maxspeed: u16,
    pub layer: i16,
    pub bridge: bool,
    pub tunnel: bool,
}

pub struct Roads<T: CoordNum + RTreeObject + PointDistance + Clone> {
    pub rtree: RoadIndex<T>,
    pub id: Vec<Id>, // Primary key
    pub geom: Vec<LineString<T>>,
    pub osm_id: Vec<u64>,
    pub code: Vec<u16>, // Foreign key to FeatureClass
    pub direction: Vec<Direction>,
    pub maxspeed: Vec<u16>,
    pub layer: Vec<i16>,
    pub bridge: Vec<bool>,
    pub tunnel: Vec<bool>,
}


pub struct RoadIndex<T: RTreeObject + PointDistance + Clone> {
    pub index: RTree<GeomWithData<T, u64>>,
}

impl<T: RTreeObject + PointDistance + Clone> RoadIndex<T> {
    pub fn new() -> RoadIndex<T> {
        Self {
            index: RTree::new(),
        }
    }

    pub fn from(ids: &[u64], roads: &[T]) -> RoadIndex<T> {
        let geomdata: Vec<GeomWithData<T, u64>> = roads
            .iter()
            .zip(ids.iter())
            .map(|(road, id)| GeomWithData::<T, u64>::new(road.clone(), *id))
            .collect();

        RoadIndex::<T> {
            index: RTree::<GeomWithData<T, u64>>::bulk_load(geomdata),
        }
    }

    pub fn insert(&mut self, id: u64, road: T) {
        let geomdata: GeomWithData<T, u64> = GeomWithData::new(road, id);
        self.index.insert(geomdata);
    }

    pub fn empty(&mut self) {
        self.index = RTree::<GeomWithData<T, u64>>::new();
    }

    pub fn remove(&mut self, _id: u64) {
        todo!()
    }
}

impl<T: RTreeObject + PointDistance + Clone> Default for RoadIndex<T> {
    fn default() -> Self {
        Self::new()
    }
}



impl<T: CoordNum + rstar::Point> Insertable<Road<T>> for Roads<T> {
    type Key = RoadKey;

    fn insert(&mut self, data: &Road<T>) -> Self::Key {
        // Does not insert duplicates
        if let Some((id, _)) = self.id.iter().zip(self.osm_id.iter()).find(|(&_, &o)| data.osm_id == o) {
            return RoadKey(*id);
        }

        // Finds the next id for the table
        let next_id = if let Some(id) = self.id.last() {
            id + 1
        } else {
            0
        };



        self.id.push(next_id);
        self.geom.push(data.geom.clone());
        self.osm_id.push(data.osm_id);
        self.code.push(data.code);
        self.direction.push(data.direction);
        self.maxspeed.push(data.maxspeed);
        self.layer.push(data.layer);
        self.bridge.push(data.bridge);
        self.tunnel.push(data.tunnel);

        RoadKey(next_id)
    }

    fn insert_many(&mut self, data: &[Road<T>]) -> Vec<Self::Key> {
        data.iter().map(|x| self.insert(x)).collect()
    }
}

impl<T: CoordNum + rstar::Point> Queryable<RoadKey> for Roads<T> {
    fn find_index(&self, key: RoadKey) -> Option<usize> {
        self.id.iter().position(|&x| x == key.0)
    }

    fn find_many_indexes(&self, key: &[RoadKey]) -> Vec<Option<usize>> {
        key.iter().map(|x| self.find_index(*x)).collect()
    }
}

impl<T: CoordNum + rstar::Point> Deleteable<RoadKey> for Roads<T> {
    type Output = Road<T>;
    fn delete(&mut self, key: RoadKey) -> Option<Self::Output> {
        if let Some(index) = self.id.iter().position(|&x| x == key.0) {
            Some(Self::Output {
                id: self.id.remove(index),
                geom: self.geom.remove(index),
                osm_id: self.osm_id.remove(index),
                code: self.code.remove(index),
                direction: self.direction.remove(index),
                maxspeed: self.maxspeed.remove(index),
                layer: self.layer.remove(index),
                bridge: self.bridge.remove(index),
                tunnel: self.tunnel.remove(index),
            })
        } else {
            None
        }
    }

    fn delete_many(&mut self, key: &[RoadKey]) -> Vec<Option<Self::Output>> {
        key.iter().map(|x| self.delete(*x)).collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NameKey(pub Id);

pub struct NameRow {
    pub id: Id,
    pub name: String,
}

pub struct Name {
    pub id: Vec<Id>, // Primary key
    pub name: Vec<String>,
}

impl Insertable<NameRow> for Name {
    type Key = NameKey;

    fn insert(&mut self, data: &NameRow) -> Self::Key {
        if let Some((id, _)) = self.id.iter().zip(self.name.iter()).find(|(&_, n)| data.name == **n) {
            return NameKey(*id);
        }

        let next_id = if let Some(id) = self.id.last() {
            id + 1
        } else {
            0
        };

        self.id.push(next_id);
        self.name.push(data.name.clone());

        NameKey(next_id)
    }

    fn insert_many(&mut self, data: &[NameRow]) -> Vec<Self::Key> {
        data.iter().map(|x| self.insert(x)).collect()
    }
}

impl Queryable<NameKey> for Name {
    fn find_index(&self, key: NameKey) -> Option<usize> {
        self.id.iter().position(|&x| x == key.0)
    }

    fn find_many_indexes(&self, key: &[NameKey]) -> Vec<Option<usize>> {
        key.iter().map(|x| self.find_index(*x)).collect()
    }
}

impl Deleteable<NameKey> for Name {
    type Output = NameRow;

    fn delete(&mut self, key: NameKey) -> Option<Self::Output> {
        if let Some(index) = self.id.iter().position(|&x| x == key.0) {
            Some(Self::Output {
                id: self.id.remove(index),
                name: self.name.remove(index),
            })
        } else {
            None
        }
    }

    fn delete_many(&mut self, key: &[NameKey]) -> Vec<Option<Self::Output>> {
        key.iter().map(|x| self.delete(*x)).collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RefManyKey(pub RoadKey, pub RefKey);
pub struct RefManyRow{
    pub road_id: Id,
    pub ref_id: Id,
}
pub struct RefMany {
    pub road_id: Vec<Id>, // Composite key 1
    pub ref_id: Vec<Id>,  // Composite key 2
}

impl Insertable<RefManyRow> for RefMany {
    type Key = RefManyKey;

    fn insert(&mut self, data: &RefManyRow) -> Self::Key {
        if let Some((road_id, ref_id)) = self.road_id.iter().zip(self.ref_id.iter()).find(|(&ro, &re)| data.road_id == ro && data.ref_id == re) {
            return RefManyKey(RoadKey(*road_id), RefKey(*ref_id));
        }

        self.road_id.push(data.road_id);
        self.ref_id.push(data.ref_id);

        RefManyKey(RoadKey(data.road_id), RefKey(data.ref_id))
    }

    fn insert_many(&mut self, data: &[RefManyRow]) -> Vec<Self::Key> {
        data.iter().map(|x| self.insert(x)).collect()
    }
}

impl Deleteable<RefManyKey> for RefMany {
    type Output = RefManyRow;

    fn delete(&mut self, key: RefManyKey) -> Option<Self::Output> {
        if let Some(index) = self.road_id.iter().zip(self.ref_id.iter()).position(|(&ro, &re)| key.0.0 == ro && key.1.0 == re) {
            Some(Self::Output {
                road_id: self.road_id.remove(index),
                ref_id: self.ref_id.remove(index),
            })
        } else {
            None
        }
    }

    fn delete_many(&mut self, key: &[RefManyKey]) -> Vec<Option<Self::Output>> {
        key.iter().map(|x| self.delete(*x)).collect()
    }
}

impl Queryable<RefManyKey> for RefMany {
    fn find_index(&self, key: RefManyKey) -> Option<usize> {
        self.road_id.iter().zip(self.ref_id.iter()).position(|(&ro, &re)| key.0.0 == ro && key.1.0 == re)
    }

    fn find_many_indexes(&self, key: &[RefManyKey]) -> Vec<Option<usize>> {
        key.iter().map(|x| self.find_index(*x)).collect()
    }
}


#[derive(Debug, Clone, Copy)]
pub struct RefKey(pub Id);
pub struct RefRow {
    pub id: Id,
    pub reff: String,
}

pub struct Ref {
    pub id: Vec<Id>, // Primary key
    pub reff: Vec<String>,
}

impl Insertable<RefRow> for Ref {
    type Key = RefKey;

    fn insert(&mut self, data: &RefRow) -> Self::Key {
        if let Some((id, _)) = self.id.iter().zip(self.reff.iter()).find(|(&_, r)| data.reff == **r) {
            return RefKey(*id);
        }

        let next_id = if let Some(id) = self.id.last() {
            id + 1
        } else {
            0
        };

        self.id.push(next_id);
        self.reff.push(data.reff.clone());

        RefKey(next_id)
    }

    fn insert_many(&mut self, data: &[RefRow]) -> Vec<Self::Key> {
        data.iter().map(|x| self.insert(x)).collect()
    }
}

impl Queryable<RefKey> for Ref {
    fn find_index(&self, key: RefKey) -> Option<usize> {
        self.id.iter().position(|&x| x == key.0)
    }

    fn find_many_indexes(&self, key: &[RefKey]) -> Vec<Option<usize>> {
        key.iter().map(|x| self.find_index(*x)).collect()
    }
}

impl Deleteable<RefKey> for Ref {
    type Output = RefRow;

    fn delete(&mut self, key: RefKey) -> Option<Self::Output> {
        if let Some(index) = self.id.iter().position(|&x| x == key.0) {
            Some(Self::Output {
                id: self.id.remove(index),
                reff: self.reff.remove(index),
            })
        } else {
            None
        }
    }

    fn delete_many(&mut self, key: &[RefKey]) -> Vec<Option<Self::Output>> {
        key.iter().map(|x| self.delete(*x)).collect()
    }
}


#[derive(Debug, Clone, Copy)]
pub struct FeatureClassKey(pub u16);

pub struct FeatureClassRow {
    pub code: u16, // Primary key
    pub fclass: String,
}

pub struct FeatureClass {
    pub code: Vec<u16>, // Primary key
    pub fclass: Vec<String>,
}

impl Insertable<FeatureClassRow> for FeatureClass {
    type Key = FeatureClassKey;

    fn insert(&mut self, data: &FeatureClassRow) -> Self::Key {
        if let Some((code, _)) = self.code.iter().zip(self.fclass.iter()).find(|(&_, o)| data.fclass == **o) {
            return FeatureClassKey(*code);
        }

        self.code.push(data.code);
        self.fclass.push(data.fclass.clone());


        FeatureClassKey(data.code)
    }

    fn insert_many(&mut self, data: &[FeatureClassRow]) -> Vec<Self::Key> {
        data.iter().map(|x| self.insert(x)).collect()
    }
}

impl Deleteable<FeatureClassKey> for FeatureClass {
    type Output = FeatureClassRow;

    fn delete(&mut self, key: FeatureClassKey) -> Option<Self::Output> {
        if let Some(index) = self.code.iter().position(|&x| x == key.0) {
            Some(Self::Output {
                code: self.code.remove(index),
                fclass: self.fclass.remove(index),
            })
        } else {
            None
        }
    }

    fn delete_many(&mut self, key: &[FeatureClassKey]) -> Vec<Option<Self::Output>> {
        key.iter().map(|x| self.delete(*x)).collect()
    }
}

impl Queryable<FeatureClassKey> for FeatureClass {
    fn find_index(&self, key: FeatureClassKey) -> Option<usize> {
        self.code.iter().position(|&x| x == key.0)
    }

    fn find_many_indexes(&self, key: &[FeatureClassKey]) -> Vec<Option<usize>> {
        key.iter().map(|x| self.find_index(*x)).collect()
    }
}




pub trait Insertable<T> {
    type Key;
    fn insert(&mut self, data: &T) -> Self::Key;
    fn insert_many(&mut self, data: &[T]) -> Vec<Self::Key>;
}

pub trait Deleteable<T> {
    type Output;
    fn delete(&mut self, key: T) -> Option<Self::Output>;
    fn delete_many(&mut self, key: &[T]) -> Vec<Option<Self::Output>>;
}

pub trait Queryable<T> {
    fn find_index(&self, key: T) -> Option<usize>;
    fn find_many_indexes(&self, key: &[T]) -> Vec<Option<usize>>;
    fn get(&self) -> &Self {
        self
    }

    fn get_mut(&mut self) -> &mut Self {
        mut self
    }

}

pub trait NearestNeighbor<T, U>
where
    T: RTreeObject + PointDistance,
    U: RTreeObject + PointDistance,
{
    fn nearest_neighbor(&self, point: T) -> Option<U>;
    fn nearest_neighbor_road(self, point: T, id: Id) -> Option<U>;
}

pub trait
