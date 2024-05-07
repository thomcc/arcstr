//! Implements Decode and Encode traits for use of ArcStr and Substr types with bincode.

use super::ArcStr;
#[cfg(feature = "substr")]
use super::Substr;

use alloc::string::String;
use bincode::{Decode, Encode};
use bincode::error::{DecodeError, EncodeError};
use bincode::enc::Encoder;
use bincode::de::{BorrowDecode, BorrowDecoder, Decoder};

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

impl<'de> BorrowDecode<'de> for ArcStr {
    fn borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D
    ) -> Result<Self, DecodeError> {
        let s: String = bincode::BorrowDecode::borrow_decode(decoder)?;
        Ok(Self::from(s))
    }
}

#[cfg(feature = "substr")]
impl Decode for Substr {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let s: String = bincode::Decode::decode(decoder)?;
        Ok(Self::from(s))
    }
}

#[cfg(feature = "substr")]
impl Encode for Substr {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        bincode::Encode::encode(&self.as_str(), encoder)?;
        Ok(())
    }
}

#[cfg(feature = "substr")]
impl<'de> BorrowDecode<'de> for Substr {
    fn borrow_decode<D: BorrowDecoder<'de>>(
        decoder: &mut D
    ) -> Result<Self, DecodeError> {
        let s: String = bincode::BorrowDecode::borrow_decode(decoder)?;
        Ok(Self::from(s))
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn arcstr_decode_encode() {
        use crate::ArcStr;

        let mut slice = [0u8; 14];
        let input = ArcStr::from("Hello, world!");

        let length = bincode::encode_into_slice(
            &input,
            &mut slice,
            bincode::config::standard()
        ).unwrap();
        assert_eq!(length, 14);

        let decoded: ArcStr = bincode::decode_from_slice(&slice, bincode::config::standard()).unwrap().0;
        assert_eq!(decoded, input);
    }

    #[cfg(feature = "substr")]
    #[test]
    fn substr_decode_encode() {
        use crate::Substr;

        let mut slice = [0u8; 14];
        let input = Substr::from("Hello, world!");

        let length = bincode::encode_into_slice(
            &input,
            &mut slice,
            bincode::config::standard()
        ).unwrap();
        assert_eq!(length, 14);

        let decoded: Substr = bincode::decode_from_slice(&slice, bincode::config::standard()).unwrap().0;
        assert_eq!(decoded, input);
    }
}