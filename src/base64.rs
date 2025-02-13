use base64::{engine::general_purpose::STANDARD, DecodeError, Engine};

pub fn decode<T: AsRef<[u8]>>(input: T) -> Result<Vec<u8>, DecodeError> {
    STANDARD.decode(input)
}

pub fn encode<T: AsRef<[u8]>>(input: T) -> String {
    STANDARD.encode(input)
}
