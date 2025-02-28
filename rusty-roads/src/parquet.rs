use geo_traits::{to_geo::ToGeoLineString, GeometryTrait, GeometryType, MultiLineStringTrait};
use std::{convert::identity, ops::Deref, sync::Arc};

use crate::{Direction, OutOfBounds, Road};
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
    #[error("IncorectGeom Value")]
    IncorectGeomValue,
    #[error("Direction out of bounds")]
    DirectionOutOfBounds,
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
            vec: &'a mut Vec<Return>,
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

        let arrow_reader = ArrowReaderBuilder::try_new(bts)
            .map_err(RoadParseError::ParquetError)?
            .build()
            .map_err(RoadParseError::ParquetError)?;

        let mut id = vec![];
        let mut osm_id = vec![];
        let mut code = vec![];
        let mut maxspeed = vec![];
        let mut direction = vec![];
        let mut layer = vec![];
        let mut bridge = vec![];
        let mut tunnel = vec![];
        let mut geom = vec![];

        for record in arrow_reader {
            let record = record.map_err(RoadParseError::ArrowError)?;
            parse_record("id", &mut id, &record, ArrayRef::as_primitive::<UInt64Type>)?;
            parse_record(
                "osm_id",
                &mut osm_id,
                &record,
                ArrayRef::as_primitive::<UInt64Type>,
            )?;
            parse_record(
                "code",
                &mut code,
                &record,
                ArrayRef::as_primitive::<UInt16Type>,
            )?;
            parse_record(
                "direction",
                &mut direction,
                &record,
                ArrayRef::as_primitive::<UInt8Type>,
            )?;
            parse_record(
                "maxspeed",
                &mut maxspeed,
                &record,
                ArrayRef::as_primitive::<UInt16Type>,
            )?;
            parse_record(
                "layer",
                &mut layer,
                &record,
                ArrayRef::as_primitive::<Int16Type>,
            )?;
            parse_record("bridge", &mut bridge, &record, ArrayRef::as_boolean)?;
            parse_record("tunnel", &mut tunnel, &record, ArrayRef::as_boolean)?;
            parse_record(
                "geom",
                &mut geom,
                &record,
                ArrayRef::as_primitive::<UInt8Type>,
            )?;
        }

        let geom_parsed = wkb::reader::read_wkb(&geom).map_err(RoadParseError::GeomEncoding)?;
        let geom_type = geom_parsed.as_type();

        let GeometryType::MultiLineString(geom) = geom_type else {
            return Err(RoadParseError::IncorectGeomValue);
        };

        let geom = geom
            .line_strings()
            .map(|x| x.to_line_string())
            .collect_vec();

        let direction = direction
            .into_iter()
            .map(|d| Direction::try_from(d))
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
