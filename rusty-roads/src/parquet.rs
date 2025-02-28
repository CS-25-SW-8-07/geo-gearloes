use std::{convert::identity, sync::Arc};

use crate::{Direction, Road};
use arrow_array::{
    cast::AsArray,
    types::{Int16Type, UInt16Type, UInt64Type, UInt8Type},
    ArrayRef, BooleanArray, Int16Array, Int32Array, Int64Array, PrimitiveArray, RecordBatch,
    UInt16Array, UInt64Array, UInt8Array,
};
use arrow_schema::ArrowError;
use bytes::{Buf, BufMut, Bytes};
use geo_types::{CoordNum, LineString, MultiLineString};
use itertools::Itertools;
use parquet::{
    arrow::{arrow_reader::ArrowReaderBuilder, *},
    data_type::ByteArray,
    errors::ParquetError,
    record,
};
use thiserror::Error;
use wkb::{error::WKBError, writer};

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
}

impl Road {
    pub fn to_parquet(self) -> Result<Bytes, RoadParseError> {
        let multi_line_string = MultiLineString::new(self.geom);
        let mut multi_line_string_buf: Vec<u8> = vec![];
        wkb::writer::write_multi_line_string(
            &mut multi_line_string_buf,
            &multi_line_string,
            wkb::Endianness::LittleEndian,
        )
        .map_err(RoadParseError::GeomEncoding)?;
        let batch = RecordBatch::try_from_iter([
            ("id", Arc::new(UInt64Array::from(self.id)) as ArrayRef),
            ("osm_id", Arc::new(UInt64Array::from(self.osm_id))),
            ("code", Arc::new(UInt16Array::from(self.code))),
            (
                "direction",
                Arc::new(UInt8Array::from(
                    self.direction
                        .into_iter()
                        .map(|x| x as u8)
                        .collect::<Vec<_>>(),
                )),
            ),
            ("maxspeed", Arc::new(UInt16Array::from(self.maxspeed))),
            ("layer", Arc::new(Int16Array::from(self.layer))),
            ("bridge", Arc::new(BooleanArray::from(self.bridge))),
            ("tunnel", Arc::new(BooleanArray::from(self.tunnel))),
            ("geom", Arc::new(UInt8Array::from(multi_line_string_buf))),
        ])
        .map_err(RoadParseError::ArrowError)?;

        let mut arrow_buf = Vec::<u8>::new();
        let mut arrow_writer = ArrowWriter::try_new(&mut arrow_buf, batch.schema(), None)
            .map_err(RoadParseError::ParquetError)?;

        arrow_writer
            .write(&batch)
            .map_err(RoadParseError::ParquetError)?;

        drop(arrow_writer);

        Ok(Bytes::from(arrow_buf))
    }
    pub fn from_parquet(bts: Bytes) -> Result<Self, RoadParseError> {
        fn parse_record<'a, Intermediate, Return, FN>(
            col: &'static str,
            record: &'a RecordBatch,
            f: FN,
        ) -> Result<Vec<Return>, RoadParseError>
        where
            FN: Fn(&'a ArrayRef) -> Intermediate,
            Intermediate: IntoIterator,
            Option<Vec<Return>>: FromIterator<Intermediate::Item>,
        {
            let colval = record
                .column_by_name(col)
                .ok_or(RoadParseError::MissingColumn(col.into()))?;
            f(colval)
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .ok_or(RoadParseError::MissingValue(col.into()))
        }

        let arrow_reader = ArrowReaderBuilder::try_new(bts)
            .map_err(RoadParseError::ParquetError)?
            .build()
            .map_err(RoadParseError::ParquetError)?;

        for record in arrow_reader {
            let record = record.map_err(RoadParseError::ArrowError)?;
            let id = parse_record("id", &record, ArrayRef::as_primitive::<UInt64Type>)?;
            let osm_id = parse_record("osm_id", &record, ArrayRef::as_primitive::<UInt64Type>)?;
            let code = parse_record("code", &record, ArrayRef::as_primitive::<UInt16Type>)?;
            let direction =
                parse_record("direction", &record, ArrayRef::as_primitive::<UInt8Type>)?;
            let maxspeed = parse_record("maxspeed", &record, ArrayRef::as_primitive::<UInt16Type>)?;
            let layer = parse_record("layer", &record, ArrayRef::as_primitive::<Int16Type>)?;
            let bridge = parse_record("bridge", &record, ArrayRef::as_boolean)?;
            let tunnel = parse_record("tunnel", &record, ArrayRef::as_boolean)?;
            let geom = parse_record("geom", &record, ArrayRef::as_primitive::<UInt8Type>)?;
        }

        Ok(Road::default())
    }
}
