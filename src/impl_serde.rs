use super::ArcStr;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for ArcStr {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self)
    }
}

impl<'de> Deserialize<'de> for ArcStr {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(ArcStrVisitor)
    }
}

struct ArcStrVisitor;
impl<'de> de::Visitor<'de> for ArcStrVisitor {
    type Value = ArcStr;
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
