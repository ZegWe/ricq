use std::collections::HashMap;
use bytes::Buf;

pub trait BinaryReader {
    fn read_string(&mut self) -> String;
    fn read_bytes_short(&mut self) -> Vec<u8>;
    fn read_tlv_map(&mut self, tag_size: usize) -> HashMap<u16, Vec<u8>>;
}

impl<B> BinaryReader for B
    where B: Buf {
    fn read_string(&mut self) -> String {
        let len = self.get_i32() as usize - 4;
        String::from_utf8(self.copy_to_bytes(len).to_vec()).unwrap()
    }

    fn read_bytes_short(&mut self) -> Vec<u8> {
        let len = self.get_u16() as usize;
        return self.copy_to_bytes(len).to_vec();
    }

    fn read_tlv_map(&mut self, tag_size: usize) -> HashMap<u16, Vec<u8>> {
        let mut m = HashMap::new();
        loop {
            if self.remaining() < tag_size {
                return m;
            }
            let mut k = 0;
            if tag_size == 1 {
                k = self.get_u8() as u16;
            } else if tag_size == 2 {
                k = self.get_u16();
            } else if tag_size == 4 {
                k = self.get_i32() as u16;
            }
            if k == 255 {
                return m;
            }
            let len = self.get_u16() as usize;
            m.insert(k, self.copy_to_bytes(len).to_vec());
        }
    }
}