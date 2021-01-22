use async_compression::futures::bufread::{BrotliDecoder, DeflateDecoder, GzipDecoder};
use async_compression::futures::write::{BrotliEncoder, DeflateEncoder, GzipEncoder};
use async_trait::async_trait;
use darpi::body::Bytes;
use darpi::header::{ToStrError, CONTENT_ENCODING};
use darpi::hyper::http::HeaderValue;
use darpi::{middleware, response::ResponderError, Body, RequestParts, Response, StatusCode};
use darpi_headers::{ContentEncoding, EncodingType, Error as ContentEncodingError};
use derive_more::Display;
use futures_util::{AsyncReadExt, AsyncWriteExt};
use std::convert::TryFrom;

#[middleware(Response)]
pub async fn compress(
    #[handler] types: &[EncodingType],
    #[response] r: &mut Response<Body>,
) -> Result<(), Error> {
    let mut b = r.body_mut();
    let mut full_body = darpi::body::to_bytes(&mut b)
        .await
        .map_err(|e| Error::ReadBody(e))?;

    let mut encoding =
        ContentEncoding::try_from(None).map_err(|e| Error::InvalidContentEncoding(e))?;

    for t in types {
        match t {
            EncodingType::Gzip => {
                full_body = Gzip.encode(&full_body).await?.into();
                encoding.append(Gzip.encoding_type());
            }
            EncodingType::Deflate => {
                full_body = Deflate.encode(&full_body).await?.into();
                encoding.append(Deflate.encoding_type());
            }
            EncodingType::Br => {
                full_body = Brotli.encode(&full_body).await?.into();
                encoding.append(Brotli.encoding_type());
            }
            _ => return Err(Error::UnsupportedContentEncoding(*t)),
        };
    }

    *b = Body::from(full_body);

    if let Some(hv) = r.headers_mut().get_mut(CONTENT_ENCODING) {
        let mut original = ContentEncoding::try_from(Some(&mut *hv))
            .map_err(|e| Error::InvalidContentEncoding(e))?;
        original.merge(encoding);
        *hv = original.into();
    } else {
        let hv: HeaderValue = encoding.into();
        r.headers_mut().insert(CONTENT_ENCODING, hv);
    }

    Ok(())
}

#[middleware(Request)]
pub async fn decompress(
    #[request_parts] rp: &RequestParts,
    #[body] mut b: &mut Body,
) -> Result<(), Error> {
    let mut full_body = darpi::body::to_bytes(&mut b)
        .await
        .map_err(|e| Error::ReadBody(e))?;

    if let Some(ce) = rp.headers.get(CONTENT_ENCODING) {
        let encodings =
            ContentEncoding::try_from(&*ce).map_err(|e| Error::InvalidContentEncoding(e))?;
        for et in encodings.into_iter() {
            match et {
                EncodingType::Gzip => {
                    full_body = Gzip.decode(&full_body).await?.into();
                }
                EncodingType::Deflate => {
                    full_body = Deflate.decode(&full_body).await?.into();
                }
                EncodingType::Br => {
                    full_body = Brotli.decode(&full_body).await?.into();
                }
                _ => return Err(Error::UnsupportedContentEncoding(et)),
            }
        }
    }

    *b = Body::from(full_body);
    Ok(())
}

pub struct Brotli;

#[async_trait]
impl Encoder for Brotli {
    fn encoding_type(&self) -> EncodingType {
        EncodingType::Br
    }
    async fn encode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let x: Vec<u8> = vec![];
        let mut writer = BrotliEncoder::new(x);

        writer
            .write_all(bytes)
            .await
            .map_err(|e| Error::EncodingIOError(e))?;
        Ok(writer.into_inner().into())
    }
}

#[async_trait]
impl Decoder for Brotli {
    async fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let mut g = BrotliDecoder::new(bytes);
        let mut x: Vec<u8> = vec![];
        g.read_to_end(&mut x)
            .await
            .map_err(|e| Error::DecodingIOError(e))?;
        Ok(x)
    }
}

pub struct Deflate;

#[async_trait]
impl Encoder for Deflate {
    fn encoding_type(&self) -> EncodingType {
        EncodingType::Deflate
    }
    async fn encode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let x: Vec<u8> = vec![];
        let mut writer = DeflateEncoder::new(x);

        writer
            .write_all(bytes)
            .await
            .map_err(|e| Error::EncodingIOError(e))?;
        Ok(writer.into_inner().into())
    }
}

#[async_trait]
impl Decoder for Deflate {
    async fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let mut g = DeflateDecoder::new(bytes);
        let mut x: Vec<u8> = vec![];
        g.read_to_end(&mut x)
            .await
            .map_err(|e| Error::DecodingIOError(e))?;
        Ok(x)
    }
}

pub struct Gzip;

#[async_trait]
impl Encoder for Gzip {
    fn encoding_type(&self) -> EncodingType {
        EncodingType::Gzip
    }
    async fn encode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let x: Vec<u8> = vec![];
        let mut writer = GzipEncoder::new(x);

        writer
            .write_all(bytes)
            .await
            .map_err(|e| Error::EncodingIOError(e))?;
        Ok(writer.into_inner().into())
    }
}

#[async_trait]
impl Decoder for Gzip {
    async fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let mut g = GzipDecoder::new(bytes);
        let mut x: Vec<u8> = vec![];
        g.read_to_end(&mut x)
            .await
            .map_err(|e| Error::DecodingIOError(e))?;
        Ok(x)
    }
}

#[async_trait]
pub trait Encoder {
    fn encoding_type(&self) -> EncodingType;
    async fn encode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error>;
}

#[async_trait]
pub trait Decoder {
    async fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error>;
}

#[derive(Display, Debug)]
pub enum Error {
    #[display(fmt = "encoding error {}", _0)]
    EncodingIOError(std::io::Error),
    #[display(fmt = "decoding error {}", _0)]
    DecodingIOError(std::io::Error),
    #[display(fmt = "read body error {}", _0)]
    ReadBody(darpi::hyper::Error),
    #[display(fmt = "invalid content encoding error {}", _0)]
    InvalidContentEncoding(ContentEncodingError),
    ToStrError(ToStrError),
    UnsupportedContentEncoding(EncodingType),
}

impl ResponderError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    }
}
