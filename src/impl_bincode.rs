use super::ArcStr;
#[cfg(feature = "substr")]
use super::Substr;

use alloc::string::String;
use bincode::{Decode, Encode};
use bincode::error::EncodeError;
use bincode::error::DecodeError;
use bincode::enc::Encoder;
use bincode::de::Decoder;

impl Decode for ArcStr {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let s: String = bincode::Decode::decode(decoder)?;
        Ok(Self::from(s))
    }
}

impl Encode for ArcStr {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        bincode::Encode::encode(&self.as_str(), encoder)?;
        Ok(())
    }
}