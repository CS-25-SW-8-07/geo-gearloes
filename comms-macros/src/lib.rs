use proc_macro::TokenStream as TS;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Data, DeriveInput, Field, Fields, FieldsNamed, Meta, parse_macro_input};

/// A proc macro for implementign the [comms::Parquet] trait.
/// Make shure all type impl [comms::comms_types::AppendFromColumn]
/// and [comms::comms_types::ToColumn] and all types results in a
/// column of the same length.
#[proc_macro_derive(Parquet, attributes(parquet_type))]
pub fn parquet(input: TS) -> TS {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input as DeriveInput);

    let Data::Struct(data) = data else {
        panic!("Can only be implemented for structs");
    };
    let Fields::Named(fields) = data.fields else {
        panic!("Can only be for named structs");
    };

    let to = to_parquet(&fields);
    let from = from_parquet(&fields);

    let code = quote! {
        impl ::comms::Parquet for #ident {
            #to
            #from
        }
    };

    code.into()
}

fn parse_field(field: &Field) -> (TokenStream, Option<TokenStream>) {
    let ty = field
        .attrs
        .iter()
        .filter(|x| {
            x.path().get_ident().map_or(false, |x| {
                x.to_token_stream().to_string().as_str() == "parquet_type"
            })
        })
        .map(|x| {
            let Meta::List(meta) = &x.meta else {
                panic!("parquet_type must be on format #[parquet_type(`type`)]");
            };

            &meta.tokens
        })
        .next()
        .map(|t| quote! {#t});
    let name = field.ident.as_ref().expect("Field must have a name");
    let name = quote! {#name};
    (name, ty)
}

fn to_parquet(fields: &FieldsNamed) -> TokenStream {
    fn create_batch(fields: &FieldsNamed) -> TokenStream {
        let batch = fields.named.iter().map(parse_field).map(|(name, ty)| {
            let name_str = name.to_string();
            ty.map_or(quote! {self.#name.to_column(#name_str)?}, |ty|
                quote! {self.#name.into_iter().map(Into::<#ty>::into).collect::<Vec<_>>().to_column(#name_str)?}
            )
        });
        quote! {#(#batch),*}
    }

    let batch = create_batch(fields);
    quote! {
        fn to_parquet(self) -> Result<::comms::Bytes, ::comms::ParquetParseError> {
            use ::comms::comms_types::ToColumn as _;
            let batch = ::comms::exports::RecordBatch::try_from_iter([
                #batch
            ]).map_err(Into::<::comms::ParquetParseError>::into)?;
            let props = ::comms::exports::WriterProperties::new();
            let mut arrow_buf = Vec::<u8>::new();
            let mut arrow_writer = ::comms::exports::ArrowWriter::try_new(&mut arrow_buf, batch.schema(), Some(props) )
                .map_err(Into::<::comms::ParquetParseError>::into)?;
            arrow_writer.write(&batch).map_err(Into::<::comms::ParquetParseError>::into)?;
            arrow_writer.close().map_err(Into::<::comms::ParquetParseError>::into)?;
            Ok(::comms::Bytes::from(arrow_buf))
        }
    }
}

fn from_parquet(data: &FieldsNamed) -> TokenStream {
    let fields = data.named.iter().map(parse_field);
    let init = fields.clone().map(|(name, ty)| {
        ty.map_or(
            quote! {let mut #name = vec![]; },
            |ty| quote! { let mut #name: Vec<#ty> = vec![]; },
        )
    });

    let names = fields.clone().map(|(name, _)| {
        quote! {#name}
    });

    let append = names.clone().map(|name| {
        let name_str = name.to_string();
        quote! { #name.append_from_column(#name_str, &record)?; }
    });

    let clean_up = fields.filter_map(|(name, ty)| {
        ty.map(move |_| {
            quote! { let #name = #name.into_iter().map(Into::into).collect::<Vec<_>>(); }
        })
    });

    quote! {
        fn from_parquet(bts: ::comms::Bytes) -> Result<Self, ::comms::ParquetParseError> {
            use ::comms::comms_types::AppendFromColumn as _;
            #(#init)*
            let arrow_reader = ::comms::exports::ArrowReaderBuilder::try_new(bts)
                .map_err(Into::<::comms::ParquetParseError>::into)?
                .build()
                .map_err(Into::<::comms::ParquetParseError>::into)?;

            for record in arrow_reader {
                let record = record.map_err(Into::<::comms::ParquetParseError>::into)?;
                #(#append)*
            }

            #(#clean_up)*

            Ok(Self {
                #(#names),*
            })
        }
    }
}
