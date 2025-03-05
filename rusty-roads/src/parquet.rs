use geo_traits::{to_geo::ToGeoLineString, GeometryTrait, GeometryType};
use std::sync::Arc;

use crate::{Direction, Road};
use arrow_array::{
    cast::AsArray,
    types::{
        Int16Type, Int32Type, Int64Type, Int8Type, UInt16Type, UInt32Type, UInt64Type, UInt8Type,
    },
    ArrayRef, ArrowPrimitiveType, BinaryArray, BooleanArray, PrimitiveArray, RecordBatch,
};
use arrow_schema::ArrowError;
use bytes::Bytes;
use geo_types::LineString;
use itertools::Itertools;
use parquet::{
    arrow::{arrow_reader::ArrowReaderBuilder, *},
    errors::ParquetError,
    file::properties::WriterProperties,
};
use thiserror::Error;
use wkb::error::WKBError;

pub trait ToParquet: Sized {
    type Error;
    /// Convert a [Self] into [Bytes] reperecenting a
    /// parquet arrow format.
    fn to_parquet(self) -> Result<Bytes, Self::Error>;
}

pub trait FromParquet: Sized {
    type Error;
    /// Translate a [Bytes] sequence reprecenting parquet arrow into [Self].
    fn from_parquet(bts: Bytes) -> Result<Self, Self::Error>;
}

#[derive(Debug, Error)]
pub enum RoadParseError {
    #[error("WBK Eroor {0}")]
    GeomEncoding(WKBError),
    #[error("Arrow Eroor {0}")]
    ArrowError(ArrowError),
    #[error("Parquet Error {0}")]
    ParquetError(ParquetError),
    #[error("Missing Column {0}")]
    MissingColumn(String),
    #[error("Missing value in column {0}")]
    MissingValue(String),
    #[error("IncorectGeom Value")]
    IncorectGeomValue,
    #[error("Direction out of bounds")]
    DirectionOutOfBounds,
}

trait ParquetType {
    type ParquetPrimitiveType: ArrowPrimitiveType<Native = Self>;
}

impl ParquetType for i8 {
    type ParquetPrimitiveType = Int8Type;
}

impl ParquetType for i16 {
    type ParquetPrimitiveType = Int16Type;
}

impl ParquetType for i32 {
    type ParquetPrimitiveType = Int32Type;
}

impl ParquetType for i64 {
    type ParquetPrimitiveType = Int64Type;
}

impl ParquetType for u8 {
    type ParquetPrimitiveType = UInt8Type;
}

impl ParquetType for u16 {
    type ParquetPrimitiveType = UInt16Type;
}

impl ParquetType for u32 {
    type ParquetPrimitiveType = UInt32Type;
}

impl ParquetType for u64 {
    type ParquetPrimitiveType = UInt64Type;
}

trait ToParquetType {
    fn to_parquet_type(self) -> Result<ArrayRef, RoadParseError>;
}

impl<U: ParquetType> ToParquetType for Vec<U>
where
    PrimitiveArray<U::ParquetPrimitiveType>: From<Self>,
{
    fn to_parquet_type(self) -> Result<ArrayRef, RoadParseError> {
        Ok(Arc::new(
            Into::<PrimitiveArray<U::ParquetPrimitiveType>>::into(self),
        ))
    }
}

impl ToParquetType for Vec<bool> {
    fn to_parquet_type(self) -> Result<ArrayRef, RoadParseError> {
        Ok(Arc::new(BooleanArray::from(self)))
    }
}

fn line_string_to_bytes(ls: LineString) -> Result<Vec<u8>, WKBError> {
    let mut buf = vec![];
    wkb::writer::write_line_string(&mut buf, &ls, wkb::Endianness::LittleEndian)?;
    Ok(buf)
}

impl ToParquetType for Vec<LineString> {
    fn to_parquet_type(self) -> Result<ArrayRef, RoadParseError> {
        let bytes = self.into_iter().map(line_string_to_bytes);
        let bytes = bytes
            .collect::<Result<Vec<Vec<u8>>, WKBError>>()
            .map_err(RoadParseError::GeomEncoding)?;
        let bts = bytes.iter().map(|v| &v[..]).collect_vec();
        let b_array = BinaryArray::from_vec(bts);
        Ok(Arc::new(b_array))
    }
}

trait ToColumn: ToParquetType + Sized {
    fn to_column<S: AsRef<str>>(self, name: S) -> Result<(S, ArrayRef), RoadParseError> {
        Ok((name, self.to_parquet_type()?))
    }
}

impl<T: ToParquetType + Sized> ToColumn for T {}

trait AppendFromColumn {
    fn append_from_column(
        &mut self,
        column_name: &str,
        record: &RecordBatch,
    ) -> Result<(), RoadParseError>;
}
#[inline]
fn parse_record<'a, Intermediate, Return, FN>(
    col: &'a str,
    vec: &mut Vec<Return>,
    record: &'a RecordBatch,
    f: FN,
) -> Result<(), RoadParseError>
where
    FN: Fn(&'a ArrayRef) -> Intermediate,
    Intermediate: IntoIterator,
    Option<Vec<Return>>: FromIterator<Intermediate::Item>,
{
    let colval = record
        .column_by_name(col)
        .ok_or(RoadParseError::MissingColumn(col.into()))?;
    let mut val = f(colval)
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or(RoadParseError::MissingValue(col.into()))?;
    vec.append(&mut val);
    Ok(())
}

impl<T: ParquetType> AppendFromColumn for Vec<T> {
    fn append_from_column(
        &mut self,
        column_name: &str,
        record: &RecordBatch,
    ) -> Result<(), RoadParseError> {
        parse_record(
            column_name,
            self,
            record,
            ArrayRef::as_primitive::<T::ParquetPrimitiveType>,
        )
    }
}

impl AppendFromColumn for Vec<bool> {
    fn append_from_column(
        &mut self,
        column_name: &str,
        record: &RecordBatch,
    ) -> Result<(), RoadParseError> {
        parse_record(column_name, self, record, ArrayRef::as_boolean)
    }
}

fn bytes_to_linestring(bts: &[u8]) -> Result<LineString, RoadParseError> {
    let geom_parsed = wkb::reader::read_wkb(bts).map_err(RoadParseError::GeomEncoding)?;
    let geom_type = geom_parsed.as_type();

    let GeometryType::LineString(geom) = geom_type else {
        return Err(RoadParseError::IncorectGeomValue);
    };

    Ok(geom.to_line_string())
}

impl AppendFromColumn for Vec<LineString> {
    fn append_from_column(
        &mut self,
        column_name: &str,
        record: &RecordBatch,
    ) -> Result<(), RoadParseError> {
        let mut v = vec![];
        parse_record(column_name, &mut v, record, ArrayRef::as_binary::<i32>)?;
        let mut ls = v
            .into_iter()
            .map(bytes_to_linestring)
            .collect::<Result<_, _>>()?;

        self.append(&mut ls);

        Ok(())
    }
}

impl ToParquet for Road {
    type Error = RoadParseError;
    fn to_parquet(self) -> Result<Bytes, Self::Error> {
        let batch = RecordBatch::try_from_iter([
            self.id.to_column("id")?,
            self.osm_id.to_column("osm_id")?,
            self.code.to_column("code")?,
            self.direction
                .into_iter()
                .map(|x| x as u8)
                .collect_vec()
                .to_column("direction")?,
            self.maxspeed.to_column("maxspeed")?,
            self.layer.to_column("layer")?,
            self.bridge.to_column("bridge")?,
            self.tunnel.to_column("tunnel")?,
            self.geom.to_column("geom")?,
        ])
        .map_err(RoadParseError::ArrowError)?;

        let props = WriterProperties::new();

        let mut arrow_buf = Vec::<u8>::new();
        let mut arrow_writer = ArrowWriter::try_new(&mut arrow_buf, batch.schema(), Some(props))
            .map_err(RoadParseError::ParquetError)?;

        arrow_writer
            .write(&batch)
            .map_err(RoadParseError::ParquetError)?;

        arrow_writer.close().map_err(RoadParseError::ParquetError)?;

        Ok(Bytes::from(arrow_buf))
    }
}

impl FromParquet for Road {
    type Error = RoadParseError;
    fn from_parquet(bts: Bytes) -> Result<Self, Self::Error> {
        println!("{}", bts.len());
        let arrow_reader = ArrowReaderBuilder::try_new(bts)
            .map_err(RoadParseError::ParquetError)?
            .build()
            .map_err(RoadParseError::ParquetError)?;

        let mut id = vec![];
        let mut osm_id = vec![];
        let mut code = vec![];
        let mut maxspeed = vec![];
        let mut direction: Vec<u8> = vec![];
        let mut layer = vec![];
        let mut bridge = vec![];
        let mut tunnel = vec![];
        let mut geom = vec![];

        for record in arrow_reader {
            let record = record.map_err(RoadParseError::ArrowError)?;
            id.append_from_column("id", &record)?;
            osm_id.append_from_column("osm_id", &record)?;
            code.append_from_column("code", &record)?;
            direction.append_from_column("direction", &record)?;
            maxspeed.append_from_column("maxspeed", &record)?;
            layer.append_from_column("layer", &record)?;
            bridge.append_from_column("bridge", &record)?;
            tunnel.append_from_column("tunnel", &record)?;
            geom.append_from_column("geom", &record)?;
        }

        let direction = direction
            .into_iter()
            .map(Direction::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| RoadParseError::DirectionOutOfBounds)?;

        Ok(Road {
            id,
            osm_id,
            geom,
            code,
            direction,
            maxspeed,
            layer,
            bridge,
            tunnel,
        })
    }
}

#[cfg(test)]
mod test {

    use geo_types::Coord;
    use rand::{random, random_range};

    use crate::{Id, RoadRow};

    use super::*;

    fn random_road(id: Id) -> RoadRow {
        RoadRow {
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
    fn test() {
        let roads: Road = ((0..100).map(random_road)).collect();
        let check = roads.clone();
        let parquet = roads.to_parquet().unwrap();
        let deque = Road::from_parquet(parquet).unwrap();
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
