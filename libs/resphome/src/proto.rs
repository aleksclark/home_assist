use anyhow::{bail, Result};

pub fn encode_varint(mut val: u64, buf: &mut Vec<u8>) {
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if val == 0 {
            break;
        }
    }
}

pub fn decode_varint(data: &[u8], pos: &mut usize) -> Result<u64> {
    let mut result: u64 = 0;
    let mut shift = 0;
    loop {
        if *pos >= data.len() {
            bail!("unexpected end of varint");
        }
        let byte = data[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            bail!("varint overflow");
        }
    }
    Ok(result)
}

fn try_decode_varint(data: &[u8], pos: &mut usize) -> Option<u64> {
    let mut result: u64 = 0;
    let mut shift = 0;
    loop {
        if *pos >= data.len() {
            return None;
        }
        let byte = data[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    Some(result)
}

pub fn encode_field_varint(field: u32, val: u64, buf: &mut Vec<u8>) {
    encode_varint(((field as u64) << 3) | 0, buf);
    encode_varint(val, buf);
}

pub fn encode_field_string(field: u32, val: &str, buf: &mut Vec<u8>) {
    encode_varint(((field as u64) << 3) | 2, buf);
    encode_varint(val.len() as u64, buf);
    buf.extend_from_slice(val.as_bytes());
}

pub fn encode_field_bytes(field: u32, val: &[u8], buf: &mut Vec<u8>) {
    encode_varint(((field as u64) << 3) | 2, buf);
    encode_varint(val.len() as u64, buf);
    buf.extend_from_slice(val);
}

pub fn encode_field_fixed32(field: u32, val: u32, buf: &mut Vec<u8>) {
    encode_varint(((field as u64) << 3) | 5, buf);
    buf.extend_from_slice(&val.to_le_bytes());
}

pub fn encode_field_float(field: u32, val: f32, buf: &mut Vec<u8>) {
    encode_varint(((field as u64) << 3) | 5, buf);
    buf.extend_from_slice(&val.to_le_bytes());
}

pub fn encode_field_bool(field: u32, val: bool, buf: &mut Vec<u8>) {
    if val {
        encode_field_varint(field, 1, buf);
    }
}

pub fn encode_field_fixed64(field: u32, val: u64, buf: &mut Vec<u8>) {
    encode_varint(((field as u64) << 3) | 1, buf);
    buf.extend_from_slice(&val.to_le_bytes());
}

pub fn encode_field_double(field: u32, val: f64, buf: &mut Vec<u8>) {
    encode_varint(((field as u64) << 3) | 1, buf);
    buf.extend_from_slice(&val.to_le_bytes());
}

#[derive(Debug, Clone)]
pub enum FieldValue<'a> {
    Varint(u64),
    Fixed64(u64),
    Bytes(&'a [u8]),
    Fixed32(u32),
}

impl<'a> FieldValue<'a> {
    pub fn as_str(&self) -> &'a str {
        if let FieldValue::Bytes(b) = self {
            core::str::from_utf8(b).unwrap_or("")
        } else {
            ""
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            FieldValue::Varint(v) => *v as u32,
            FieldValue::Fixed32(v) => *v,
            _ => 0,
        }
    }

    pub fn as_u64(&self) -> u64 {
        match self {
            FieldValue::Varint(v) => *v,
            FieldValue::Fixed64(v) => *v,
            _ => 0,
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            FieldValue::Varint(v) => *v != 0,
            _ => false,
        }
    }

    pub fn as_f32(&self) -> f32 {
        match self {
            FieldValue::Fixed32(v) => f32::from_bits(*v),
            FieldValue::Varint(v) => *v as f32,
            _ => 0.0,
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            FieldValue::Fixed64(v) => f64::from_bits(*v),
            FieldValue::Varint(v) => *v as f64,
            _ => 0.0,
        }
    }

    pub fn as_bytes(&self) -> &'a [u8] {
        if let FieldValue::Bytes(b) = self {
            b
        } else {
            &[]
        }
    }
}

pub struct FieldIter<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> FieldIter<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
}

impl<'a> Iterator for FieldIter<'a> {
    type Item = (u32, FieldValue<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() {
            return None;
        }
        let tag = try_decode_varint(self.data, &mut self.pos)?;
        let field_number = (tag >> 3) as u32;
        let wire_type = (tag & 0x07) as u8;
        let value = match wire_type {
            0 => {
                let v = try_decode_varint(self.data, &mut self.pos)?;
                FieldValue::Varint(v)
            }
            1 => {
                if self.pos + 8 > self.data.len() {
                    return None;
                }
                let v = u64::from_le_bytes(
                    self.data[self.pos..self.pos + 8].try_into().ok()?,
                );
                self.pos += 8;
                FieldValue::Fixed64(v)
            }
            2 => {
                let len = try_decode_varint(self.data, &mut self.pos)? as usize;
                if self.pos + len > self.data.len() {
                    return None;
                }
                let bytes = &self.data[self.pos..self.pos + len];
                self.pos += len;
                FieldValue::Bytes(bytes)
            }
            5 => {
                if self.pos + 4 > self.data.len() {
                    return None;
                }
                let v = u32::from_le_bytes(
                    self.data[self.pos..self.pos + 4].try_into().ok()?,
                );
                self.pos += 4;
                FieldValue::Fixed32(v)
            }
            _ => return None,
        };
        Some((field_number, value))
    }
}

pub fn frame_plaintext(msg_type: u32, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(1 + 10 + payload.len());
    frame.push(0x00);
    encode_varint(payload.len() as u64, &mut frame);
    encode_varint(msg_type as u64, &mut frame);
    frame.extend_from_slice(payload);
    frame
}

pub struct FrameReader {
    buf: Vec<u8>,
}

impl FrameReader {
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(512),
        }
    }

    pub fn push(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    pub fn next_frame(&mut self) -> Option<(u32, Vec<u8>)> {
        if self.buf.is_empty() {
            return None;
        }
        if self.buf[0] != 0x00 {
            self.buf.remove(0);
            return None;
        }

        let mut pos = 1;
        let len = try_decode_varint(&self.buf, &mut pos)? as usize;
        let msg_type = try_decode_varint(&self.buf, &mut pos)? as u32;

        let total = pos + len;
        if self.buf.len() < total {
            return None;
        }

        let payload = self.buf[pos..total].to_vec();
        self.buf.drain(..total);
        Some((msg_type, payload))
    }

    pub fn pending_bytes(&self) -> usize {
        self.buf.len()
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }
}

impl Default for FrameReader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_roundtrip_zero() {
        let mut buf = Vec::new();
        encode_varint(0, &mut buf);
        let mut pos = 0;
        assert_eq!(decode_varint(&buf, &mut pos).unwrap(), 0);
        assert_eq!(pos, 1);
    }

    #[test]
    fn test_varint_roundtrip_small() {
        let mut buf = Vec::new();
        encode_varint(127, &mut buf);
        assert_eq!(buf.len(), 1);
        let mut pos = 0;
        assert_eq!(decode_varint(&buf, &mut pos).unwrap(), 127);
    }

    #[test]
    fn test_varint_roundtrip_multibyte() {
        let mut buf = Vec::new();
        encode_varint(300, &mut buf);
        assert_eq!(buf.len(), 2);
        let mut pos = 0;
        assert_eq!(decode_varint(&buf, &mut pos).unwrap(), 300);
    }

    #[test]
    fn test_varint_roundtrip_large() {
        let mut buf = Vec::new();
        let val = u64::MAX >> 1;
        encode_varint(val, &mut buf);
        let mut pos = 0;
        assert_eq!(decode_varint(&buf, &mut pos).unwrap(), val);
    }

    #[test]
    fn test_varint_max() {
        let mut buf = Vec::new();
        encode_varint(u64::MAX, &mut buf);
        let mut pos = 0;
        assert_eq!(decode_varint(&buf, &mut pos).unwrap(), u64::MAX);
    }

    #[test]
    fn test_decode_varint_empty() {
        let buf = [];
        let mut pos = 0;
        assert!(decode_varint(&buf, &mut pos).is_err());
    }

    #[test]
    fn test_decode_varint_truncated() {
        let buf = [0x80]; // continuation bit set but no next byte
        let mut pos = 0;
        assert!(decode_varint(&buf, &mut pos).is_err());
    }

    #[test]
    fn test_encode_field_varint() {
        let mut buf = Vec::new();
        encode_field_varint(1, 150, &mut buf);
        let mut pos = 0;
        let tag = decode_varint(&buf, &mut pos).unwrap();
        assert_eq!(tag >> 3, 1); // field 1
        assert_eq!(tag & 0x07, 0); // wire type 0 (varint)
        let val = decode_varint(&buf, &mut pos).unwrap();
        assert_eq!(val, 150);
    }

    #[test]
    fn test_encode_field_string() {
        let mut buf = Vec::new();
        encode_field_string(3, "hello", &mut buf);
        let iter = FieldIter::new(&buf);
        let fields: Vec<_> = iter.collect();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].0, 3);
        assert_eq!(fields[0].1.as_str(), "hello");
    }

    #[test]
    fn test_encode_field_fixed32() {
        let mut buf = Vec::new();
        encode_field_fixed32(2, 0xDEADBEEF, &mut buf);
        let iter = FieldIter::new(&buf);
        let fields: Vec<_> = iter.collect();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].0, 2);
        assert_eq!(fields[0].1.as_u32(), 0xDEADBEEF);
    }

    #[test]
    fn test_encode_field_float() {
        let mut buf = Vec::new();
        encode_field_float(1, 3.14, &mut buf);
        let iter = FieldIter::new(&buf);
        let fields: Vec<_> = iter.collect();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].0, 1);
        let f = fields[0].1.as_f32();
        assert!((f - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_encode_field_bool_true() {
        let mut buf = Vec::new();
        encode_field_bool(5, true, &mut buf);
        let iter = FieldIter::new(&buf);
        let fields: Vec<_> = iter.collect();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].0, 5);
        assert!(fields[0].1.as_bool());
    }

    #[test]
    fn test_encode_field_bool_false() {
        let mut buf = Vec::new();
        encode_field_bool(5, false, &mut buf);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_encode_field_bytes() {
        let mut buf = Vec::new();
        let data = &[0xDE, 0xAD, 0xBE, 0xEF];
        encode_field_bytes(7, data, &mut buf);
        let iter = FieldIter::new(&buf);
        let fields: Vec<_> = iter.collect();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].0, 7);
        assert_eq!(fields[0].1.as_bytes(), data);
    }

    #[test]
    fn test_field_iter_multiple_fields() {
        let mut buf = Vec::new();
        encode_field_varint(1, 42, &mut buf);
        encode_field_string(2, "test", &mut buf);
        encode_field_fixed32(3, 100, &mut buf);
        encode_field_bool(4, true, &mut buf);

        let fields: Vec<_> = FieldIter::new(&buf).collect();
        assert_eq!(fields.len(), 4);
        assert_eq!(fields[0].0, 1);
        assert_eq!(fields[0].1.as_u32(), 42);
        assert_eq!(fields[1].0, 2);
        assert_eq!(fields[1].1.as_str(), "test");
        assert_eq!(fields[2].0, 3);
        assert_eq!(fields[2].1.as_u32(), 100);
        assert_eq!(fields[3].0, 4);
        assert!(fields[3].1.as_bool());
    }

    #[test]
    fn test_field_iter_empty() {
        let fields: Vec<_> = FieldIter::new(&[]).collect();
        assert!(fields.is_empty());
    }

    #[test]
    fn test_frame_plaintext_structure() {
        let payload = b"hello";
        let frame = frame_plaintext(1, payload);
        assert_eq!(frame[0], 0x00); // preamble
        let mut pos = 1;
        let len = decode_varint(&frame, &mut pos).unwrap();
        assert_eq!(len, 5);
        let msg_type = decode_varint(&frame, &mut pos).unwrap();
        assert_eq!(msg_type, 1);
        assert_eq!(&frame[pos..], b"hello");
    }

    #[test]
    fn test_frame_plaintext_empty_payload() {
        let frame = frame_plaintext(7, &[]);
        assert_eq!(frame[0], 0x00);
        let mut pos = 1;
        let len = decode_varint(&frame, &mut pos).unwrap();
        assert_eq!(len, 0);
        let msg_type = decode_varint(&frame, &mut pos).unwrap();
        assert_eq!(msg_type, 7);
        assert_eq!(pos, frame.len());
    }

    #[test]
    fn test_frame_reader_single_frame() {
        let frame = frame_plaintext(42, b"data");
        let mut reader = FrameReader::new();
        reader.push(&frame);
        let (msg_type, payload) = reader.next_frame().unwrap();
        assert_eq!(msg_type, 42);
        assert_eq!(payload, b"data");
        assert!(reader.next_frame().is_none());
    }

    #[test]
    fn test_frame_reader_multiple_frames() {
        let frame1 = frame_plaintext(1, b"first");
        let frame2 = frame_plaintext(2, b"second");
        let mut reader = FrameReader::new();
        reader.push(&frame1);
        reader.push(&frame2);

        let (t1, p1) = reader.next_frame().unwrap();
        assert_eq!(t1, 1);
        assert_eq!(p1, b"first");

        let (t2, p2) = reader.next_frame().unwrap();
        assert_eq!(t2, 2);
        assert_eq!(p2, b"second");

        assert!(reader.next_frame().is_none());
    }

    #[test]
    fn test_frame_reader_partial_data() {
        let frame = frame_plaintext(1, b"hello");
        let mut reader = FrameReader::new();
        reader.push(&frame[..2]);
        assert!(reader.next_frame().is_none());
        reader.push(&frame[2..]);
        let (msg_type, payload) = reader.next_frame().unwrap();
        assert_eq!(msg_type, 1);
        assert_eq!(payload, b"hello");
    }

    #[test]
    fn test_frame_reader_skips_invalid_preamble() {
        let mut data = vec![0xFF]; // invalid preamble
        data.extend_from_slice(&frame_plaintext(5, b"ok"));
        let mut reader = FrameReader::new();
        reader.push(&data);
        assert!(reader.next_frame().is_none()); // skips 0xFF
        let (msg_type, payload) = reader.next_frame().unwrap();
        assert_eq!(msg_type, 5);
        assert_eq!(payload, b"ok");
    }

    #[test]
    fn test_frame_reader_clear() {
        let mut reader = FrameReader::new();
        reader.push(b"some data");
        assert_eq!(reader.pending_bytes(), 9);
        reader.clear();
        assert_eq!(reader.pending_bytes(), 0);
    }

    #[test]
    fn test_field_value_defaults() {
        let v = FieldValue::Varint(0);
        assert_eq!(v.as_str(), "");
        assert_eq!(v.as_bytes(), &[]);

        let v = FieldValue::Bytes(b"test");
        assert_eq!(v.as_u32(), 0);
        assert_eq!(v.as_bool(), false);
        assert_eq!(v.as_f32(), 0.0);
    }

    #[test]
    fn test_encode_field_fixed64() {
        let mut buf = Vec::new();
        encode_field_fixed64(1, 0xDEAD_BEEF_CAFE_BABE, &mut buf);
        let iter = FieldIter::new(&buf);
        let fields: Vec<_> = iter.collect();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].0, 1);
        assert_eq!(fields[0].1.as_u64(), 0xDEAD_BEEF_CAFE_BABE);
    }

    #[test]
    fn test_encode_field_double() {
        let mut buf = Vec::new();
        encode_field_double(1, 3.141592653589793, &mut buf);
        let iter = FieldIter::new(&buf);
        let fields: Vec<_> = iter.collect();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].0, 1);
        let d = fields[0].1.as_f64();
        assert!((d - 3.141592653589793).abs() < 1e-10);
    }

    #[test]
    fn test_varint_consecutive_decode() {
        let mut buf = Vec::new();
        encode_varint(1, &mut buf);
        encode_varint(300, &mut buf);
        encode_varint(u64::MAX, &mut buf);

        let mut pos = 0;
        assert_eq!(decode_varint(&buf, &mut pos).unwrap(), 1);
        assert_eq!(decode_varint(&buf, &mut pos).unwrap(), 300);
        assert_eq!(decode_varint(&buf, &mut pos).unwrap(), u64::MAX);
        assert_eq!(pos, buf.len());
    }

    #[test]
    fn test_frame_reader_large_payload() {
        let payload: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        let frame = frame_plaintext(99, &payload);
        let mut reader = FrameReader::new();
        reader.push(&frame);
        let (msg_type, p) = reader.next_frame().unwrap();
        assert_eq!(msg_type, 99);
        assert_eq!(p, payload);
    }
}
