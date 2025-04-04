use geo_traits::{GeometryTrait, GeometryType, to_geo::ToGeoLineString};
use std::sync::Arc;

use arrow_array::{
    ArrayRef, ArrowPrimitiveType, BinaryArray, BooleanArray, PrimitiveArray, RecordBatch,
    StringArray,
    cast::AsArray,
    types::{
        Float32Type, Float64Type, Int8Type, Int16Type, Int32Type, Int64Type, UInt8Type, UInt16Type,
        UInt32Type, UInt64Type,
    },
};
use arrow_schema::ArrowError;
use bytes::Bytes;
use geo_types::LineString;
use itertools::Itertools;
use parquet::errors::ParquetError;
use thiserror::Error;
use wkb::error::WKBError;

pub trait Parquet: Sized {
    /// Convert a [Self] into [Bytes] reperecenting a
    /// parquet arrow format.
    fn to_parquet(self) -> Result<Bytes, ParquetParseError>;
    /// Translate a [Bytes] sequence reprecenting parquet arrow into [Self].
    fn from_parquet(bts: Bytes) -> Result<Self, ParquetParseError>;
}

#[derive(Debug, Error)]
pub enum ParquetParseError {
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
}

impl From<WKBError> for ParquetParseError {
    fn from(value: WKBError) -> Self {
        Self::GeomEncoding(value)
    }
}
impl From<ArrowError> for ParquetParseError {
    fn from(value: ArrowError) -> Self {
        Self::ArrowError(value)
    }
}
impl From<ParquetError> for ParquetParseError {
    fn from(value: ParquetError) -> Self {
        Self::ParquetError(value)
    }
}

pub trait ParquetType {
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

impl ParquetType for f32 {
    type ParquetPrimitiveType = Float32Type;
}

impl ParquetType for f64 {
    type ParquetPrimitiveType = Float64Type;
}

pub trait ToParquetType {
    fn to_parquet_type(self) -> Result<ArrayRef, ParquetParseError>;
}

impl<U: ParquetType> ToParquetType for Vec<U>
where
    PrimitiveArray<U::ParquetPrimitiveType>: From<Self>,
{
    fn to_parquet_type(self) -> Result<ArrayRef, ParquetParseError> {
        Ok(Arc::new(
            Into::<PrimitiveArray<U::ParquetPrimitiveType>>::into(self),
        ))
    }
}

impl ToParquetType for Vec<bool> {
    fn to_parquet_type(self) -> Result<ArrayRef, ParquetParseError> {
        Ok(Arc::new(BooleanArray::from(self)))
    }
}

fn line_string_to_bytes(ls: LineString) -> Result<Vec<u8>, WKBError> {
    let mut buf = vec![];
    wkb::writer::write_line_string(&mut buf, &ls, wkb::Endianness::LittleEndian)?;
    Ok(buf)
}

impl ToParquetType for Vec<LineString> {
    fn to_parquet_type(self) -> Result<ArrayRef, ParquetParseError> {
        let bytes = self.into_iter().map(line_string_to_bytes);
        let bytes = bytes
            .collect::<Result<Vec<Vec<u8>>, WKBError>>()
            .map_err(ParquetParseError::GeomEncoding)?;
        let bts = bytes.iter().map(|v| &v[..]).collect_vec();
        let b_array = BinaryArray::from_vec(bts);
        Ok(Arc::new(b_array))
    }
}

impl ToParquetType for Vec<String> {
    fn to_parquet_type(self) -> Result<ArrayRef, ParquetParseError> {
        let b_array = StringArray::from(self);
        Ok(Arc::new(b_array))
    }
}

pub trait ToColumn: ToParquetType + Sized {
    fn to_column<S: AsRef<str>>(self, name: S) -> Result<(S, ArrayRef), ParquetParseError> {
        Ok((name, self.to_parquet_type()?))
    }
}

impl<T: ToParquetType + Sized> ToColumn for T {}

pub trait AppendFromColumn {
    fn append_from_column(
        &mut self,
        column_name: &str,
        record: &RecordBatch,
    ) -> Result<(), ParquetParseError>;
}
#[inline]
fn parse_record<'a, Intermediate, Return, FN>(
    col: &'a str,
    vec: &mut Vec<Return>,
    record: &'a RecordBatch,
    f: FN,
) -> Result<(), ParquetParseError>
where
    FN: Fn(&'a ArrayRef) -> Intermediate,
    Intermediate: IntoIterator,
    Option<Vec<Return>>: FromIterator<Intermediate::Item>,
{
    let colval = record
        .column_by_name(col)
        .ok_or(ParquetParseError::MissingColumn(col.into()))?;
    let mut val = f(colval)
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or(ParquetParseError::MissingValue(col.into()))?;
    vec.append(&mut val);
    Ok(())
}

impl<T: ParquetType> AppendFromColumn for Vec<T> {
    fn append_from_column(
        &mut self,
        column_name: &str,
        record: &RecordBatch,
    ) -> Result<(), ParquetParseError> {
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
    ) -> Result<(), ParquetParseError> {
        parse_record(column_name, self, record, ArrayRef::as_boolean)
    }
}

fn bytes_to_linestring(bts: &[u8]) -> Result<LineString, ParquetParseError> {
    let geom_parsed = wkb::reader::read_wkb(bts).map_err(Into::<ParquetParseError>::into)?;
    let geom_type = geom_parsed.as_type();

    let GeometryType::LineString(geom) = geom_type else {
        return Err(ParquetParseError::IncorectGeomValue);
    };

    Ok(geom.to_line_string())
}

impl AppendFromColumn for Vec<LineString> {
    fn append_from_column(
        &mut self,
        column_name: &str,
        record: &RecordBatch,
    ) -> Result<(), ParquetParseError> {
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

impl AppendFromColumn for Vec<String> {
    fn append_from_column(
        &mut self,
        column_name: &str,
        record: &RecordBatch,
    ) -> Result<(), ParquetParseError> {
        let mut v = vec![];
        parse_record(column_name, &mut v, record, ArrayRef::as_string::<i32>)?;
        self.append(&mut v.into_iter().map(String::from).collect_vec());
        Ok(())
    }
}
