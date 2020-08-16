use super::ArcStr;
#[cfg(feature = "substr")]
use super::Substr;

use core::marker::PhantomData;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for ArcStr {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self)
    }
}

impl<'de> Deserialize<'de> for ArcStr {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(StrVisitor::<ArcStr>(PhantomData))
    }
}

#[cfg(feature = "substr")]
impl Serialize for crate::Substr {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self)
    }
}

#[cfg(feature = "substr")]
impl<'de> Deserialize<'de> for Substr {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(StrVisitor::<Substr>(PhantomData))
    }
}

struct StrVisitor<StrTy>(PhantomData<fn() -> StrTy>);

impl<'de, StrTy> de::Visitor<'de> for StrVisitor<StrTy>
where
    for<'a> &'a str: Into<StrTy>,
{
    type Value = StrTy;
    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("a string")
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(v.into())
    }
    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        match core::str::from_utf8(v) {
            Ok(s) => Ok(s.into()),
            Err(_) => Err(de::Error::invalid_value(de::Unexpected::Bytes(v), &self)),
        }
    }
}
