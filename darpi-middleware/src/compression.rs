use async_compression::futures::bufread::{BrotliDecoder, DeflateDecoder, GzipDecoder};
use async_compression::futures::write::{BrotliEncoder, DeflateEncoder, GzipEncoder};
use async_trait::async_trait;
use darpi::body::Bytes;
use darpi::header::CONTENT_ENCODING;
use darpi::{middleware, response::ResponderError, Body, RequestParts};
use derive_more::Display;
use futures_util::{AsyncReadExt, AsyncWriteExt};

#[middleware(Request)]
pub async fn decompress(
    #[request_parts] rp: &RequestParts,
    #[body] mut b: &mut Body,
) -> Result<(), Error> {
    let mut full_body = darpi::body::to_bytes(&mut b)
        .await
        .map_err(|e| Error::ReadBody(e))?;

    if let Some(ce) = rp.headers.get(CONTENT_ENCODING) {
        let ce = ce
            .to_str()
            .map_err(|e| Error::InvalidContentEncoding(e.to_string()))?;

        let formats: Vec<&str> = ce.split(", ").collect();
        for f in formats {
            match f {
                "gzip" => {
                    full_body = Gzip.decode(&full_body).await?.into();
                }
                "deflate" => {
                    full_body = Deflate.decode(&full_body).await?.into();
                }
                "br" => {
                    full_body = Brotli.decode(&full_body).await?.into();
                }
                _ => {}
            }
        }
    }

    let new_body: Bytes = full_body.into();
    *b = Body::from(new_body);
    Ok(())
}

pub struct Brotli;

#[async_trait]
impl Encoder for Brotli {
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
    InvalidContentEncoding(String),
}

impl ResponderError for Error {}
