use darpi::header::{HeaderValue, ToStrError};
use derive_more::Display;
use std::convert::TryFrom;

#[derive(Display, Debug, Copy, Clone)]
pub enum EncodingType {
    Gzip,
    Deflate,
    Compress,
    Identity,
    Br,
}

#[derive(Display, Debug)]
pub enum Error {
    UnknownStr(String),
    ToStrError(ToStrError),
}

impl<'a> TryFrom<&'a str> for EncodingType {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let v = match value {
            "gzip" => Self::Gzip,
            "deflate" => Self::Deflate,
            "compress" => Self::Compress,
            "identity" => Self::Identity,
            "br" => Self::Br,
            _ => return Err(Error::UnknownStr(value.to_string())),
        };
        Ok(v)
    }
}

impl Into<&str> for EncodingType {
    fn into(self) -> &'static str {
        match self {
            Self::Gzip => "gzip",
            Self::Deflate => "deflate",
            Self::Compress => "compress",
            Self::Identity => "identity",
            Self::Br => "br",
        }
    }
}

pub struct ContentEncoding {
    encoding_types: Vec<EncodingType>,
}

impl ContentEncoding {
    pub fn append(&mut self, et: EncodingType) {
        self.encoding_types.push(et)
    }
    pub fn merge(&mut self, other: ContentEncoding) {
        for et in other.encoding_types {
            self.append(et)
        }
    }
}

impl Into<HeaderValue> for ContentEncoding {
    fn into(self) -> HeaderValue {
        let types: Vec<&str> = self.encoding_types.into_iter().map(|t| t.into()).collect();
        let types = types.join(", ");
        HeaderValue::from_str(&format!("Content-Encoding: {}", types)).expect("this cannot happen")
    }
}

impl TryFrom<Option<&mut HeaderValue>> for ContentEncoding {
    type Error = Error;

    fn try_from(hv: Option<&mut HeaderValue>) -> Result<Self, Self::Error> {
        let hv = match hv {
            Some(s) => s,
            None => {
                return Ok(Self {
                    encoding_types: vec![],
                })
            }
        };

        if hv.is_empty() {
            return Ok(Self {
                encoding_types: vec![],
            });
        }
        let parts: Vec<&str> = hv
            .to_str()
            .map_err(|e| Error::ToStrError(e))?
            .split(", ")
            .collect();

        let mut encoding_types = vec![];
        for part in parts {
            let et = EncodingType::try_from(part)?;
            encoding_types.push(et);
        }

        Ok(Self { encoding_types })
    }
}

impl TryFrom<&HeaderValue> for ContentEncoding {
    type Error = Error;

    fn try_from(hv: &HeaderValue) -> Result<Self, Self::Error> {
        if hv.is_empty() {
            return Ok(Self {
                encoding_types: vec![],
            });
        }
        let parts: Vec<&str> = hv
            .to_str()
            .map_err(|e| Error::ToStrError(e))?
            .split(", ")
            .collect();

        let mut encoding_types = vec![];
        for part in parts {
            let et = EncodingType::try_from(part)?;
            encoding_types.push(et);
        }

        Ok(Self { encoding_types })
    }
}

impl IntoIterator for ContentEncoding {
    type Item = EncodingType;
    type IntoIter = std::vec::IntoIter<EncodingType>;

    fn into_iter(self) -> Self::IntoIter {
        self.encoding_types.into_iter()
    }
}
