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

pub fn encode_field_varint(field: u32, val: u64, buf: &mut Vec<u8>) {
    encode_varint(((field as u64) << 3) | 0, buf);
    encode_varint(val, buf);
}

pub fn encode_field_string(field: u32, val: &str, buf: &mut Vec<u8>) {
    encode_varint(((field as u64) << 3) | 2, buf);
    encode_varint(val.len() as u64, buf);
    buf.extend_from_slice(val.as_bytes());
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

pub struct FieldIter<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> FieldIter<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
}

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

    pub fn as_bool(&self) -> bool {
        match self {
            FieldValue::Varint(v) => *v != 0,
            _ => false,
        }
    }
}

impl<'a> Iterator for FieldIter<'a> {
    type Item = (u32, FieldValue<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() {
            return None;
        }
        let tag = decode_varint(self.data, &mut self.pos).ok()?;
        let field_number = (tag >> 3) as u32;
        let wire_type = (tag & 0x07) as u8;
        let value = match wire_type {
            0 => {
                let v = decode_varint(self.data, &mut self.pos).ok()?;
                FieldValue::Varint(v)
            }
            1 => {
                if self.pos + 8 > self.data.len() {
                    return None;
                }
                let v = u64::from_le_bytes(self.data[self.pos..self.pos + 8].try_into().ok()?);
                self.pos += 8;
                FieldValue::Fixed64(v)
            }
            2 => {
                let len = decode_varint(self.data, &mut self.pos).ok()? as usize;
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
                let v = u32::from_le_bytes(self.data[self.pos..self.pos + 4].try_into().ok()?);
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
        let len = match try_decode_varint(&self.buf, &mut pos) {
            Some(v) => v as usize,
            None => return None,
        };
        let msg_type = match try_decode_varint(&self.buf, &mut pos) {
            Some(v) => v as u32,
            None => return None,
        };

        let total = pos + len;
        if self.buf.len() < total {
            return None;
        }

        let payload = self.buf[pos..total].to_vec();
        self.buf.drain(..total);
        Some((msg_type, payload))
    }
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
