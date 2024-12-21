use bytes::Buf;

pub struct UnionBuf<FirstBuf: Buf, SecondBuf: Buf> {
    inner: UnionBufEnum<FirstBuf, SecondBuf>,
}

impl<FirstBuf: Buf, SecondBuf: Buf> UnionBuf<FirstBuf, SecondBuf> {
    pub fn first(first: FirstBuf) -> Self {
        return UnionBuf {
            inner: UnionBufEnum::First(first),
        };
    }

    pub fn second(second: SecondBuf) -> Self {
        return UnionBuf {
            inner: UnionBufEnum::Second(second),
        };
    }
}

impl<FirstBuf: Buf, SecondBuf: Buf> Buf for UnionBuf<FirstBuf, SecondBuf> {
    fn remaining(&self) -> usize {
        match &self.inner {
            UnionBufEnum::First(first_buf) => first_buf.remaining(),
            UnionBufEnum::Second(second_buf) => second_buf.remaining(),
        }
    }

    fn chunk(&self) -> &[u8] {
        match &self.inner {
            UnionBufEnum::First(first_buf) => first_buf.chunk(),
            UnionBufEnum::Second(second_buf) => second_buf.chunk(),
        }
    }

    fn advance(&mut self, cnt: usize) {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.advance(cnt),
            UnionBufEnum::Second(second_buf) => second_buf.advance(cnt),
        }
    }

    fn chunks_vectored<'a>(&'a self, dst: &mut [std::io::IoSlice<'a>]) -> usize {
        match &self.inner {
            UnionBufEnum::First(first_buf) => first_buf.chunks_vectored(dst),
            UnionBufEnum::Second(second_buf) => second_buf.chunks_vectored(dst),
        }
    }

    fn has_remaining(&self) -> bool {
        match &self.inner {
            UnionBufEnum::First(first_buf) => first_buf.has_remaining(),
            UnionBufEnum::Second(second_buf) => second_buf.has_remaining(),
        }
    }

    fn copy_to_slice(&mut self, dst: &mut [u8]) {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.copy_to_slice(dst),
            UnionBufEnum::Second(second_buf) => second_buf.copy_to_slice(dst),
        }
    }

    fn get_u8(&mut self) -> u8 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_u8(),
            UnionBufEnum::Second(second_buf) => second_buf.get_u8(),
        }
    }

    fn get_i8(&mut self) -> i8 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_i8(),
            UnionBufEnum::Second(second_buf) => second_buf.get_i8(),
        }
    }

    fn get_u16(&mut self) -> u16 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_u16(),
            UnionBufEnum::Second(second_buf) => second_buf.get_u16(),
        }
    }

    fn get_u16_le(&mut self) -> u16 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_u16_le(),
            UnionBufEnum::Second(second_buf) => second_buf.get_u16_le(),
        }
    }

    fn get_u16_ne(&mut self) -> u16 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_u16_ne(),
            UnionBufEnum::Second(second_buf) => second_buf.get_u16_ne(),
        }
    }

    fn get_i16(&mut self) -> i16 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_i16(),
            UnionBufEnum::Second(second_buf) => second_buf.get_i16(),
        }
    }

    fn get_i16_le(&mut self) -> i16 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_i16_le(),
            UnionBufEnum::Second(second_buf) => second_buf.get_i16_le(),
        }
    }

    fn get_i16_ne(&mut self) -> i16 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_i16_ne(),
            UnionBufEnum::Second(second_buf) => second_buf.get_i16_ne(),
        }
    }

    fn get_u32(&mut self) -> u32 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_u32(),
            UnionBufEnum::Second(second_buf) => second_buf.get_u32(),
        }
    }

    fn get_u32_le(&mut self) -> u32 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_u32_le(),
            UnionBufEnum::Second(second_buf) => second_buf.get_u32_le(),
        }
    }

    fn get_u32_ne(&mut self) -> u32 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_u32_ne(),
            UnionBufEnum::Second(second_buf) => second_buf.get_u32_ne(),
        }
    }

    fn get_i32(&mut self) -> i32 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_i32(),
            UnionBufEnum::Second(second_buf) => second_buf.get_i32(),
        }
    }

    fn get_i32_le(&mut self) -> i32 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_i32_le(),
            UnionBufEnum::Second(second_buf) => second_buf.get_i32_le(),
        }
    }

    fn get_i32_ne(&mut self) -> i32 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_i32_ne(),
            UnionBufEnum::Second(second_buf) => second_buf.get_i32_ne(),
        }
    }

    fn get_u64(&mut self) -> u64 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_u64(),
            UnionBufEnum::Second(second_buf) => second_buf.get_u64(),
        }
    }

    fn get_u64_le(&mut self) -> u64 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_u64_le(),
            UnionBufEnum::Second(second_buf) => second_buf.get_u64_le(),
        }
    }

    fn get_u64_ne(&mut self) -> u64 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_u64_ne(),
            UnionBufEnum::Second(second_buf) => second_buf.get_u64_ne(),
        }
    }

    fn get_i64(&mut self) -> i64 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_i64(),
            UnionBufEnum::Second(second_buf) => second_buf.get_i64(),
        }
    }

    fn get_i64_le(&mut self) -> i64 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_i64_le(),
            UnionBufEnum::Second(second_buf) => second_buf.get_i64_le(),
        }
    }

    fn get_i64_ne(&mut self) -> i64 {
        match &mut self.inner{
            UnionBufEnum::First(first_buf) => first_buf.get_i64_ne(),
            UnionBufEnum::Second(second_buf) => second_buf.get_i64_ne(),
        }
    }

    fn get_u128(&mut self) -> u128 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_u128(),
            UnionBufEnum::Second(second_buf) => second_buf.get_u128(),
        }
    }

    fn get_u128_le(&mut self) -> u128 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_u128_le(),
            UnionBufEnum::Second(second_buf) => second_buf.get_u128_le(),
        }
    }

    fn get_u128_ne(&mut self) -> u128 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_u128_ne(),
            UnionBufEnum::Second(second_buf) => second_buf.get_u128_ne(),
        }
    }

    fn get_i128(&mut self) -> i128 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_i128(),
            UnionBufEnum::Second(second_buf) => second_buf.get_i128(),
        }
    }

    fn get_i128_le(&mut self) -> i128 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_i128_le(),
            UnionBufEnum::Second(second_buf) => second_buf.get_i128_le(),
        }
    }

    fn get_i128_ne(&mut self) -> i128 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_i128_ne(),
            UnionBufEnum::Second(second_buf) => second_buf.get_i128_ne(),
        }
    }

    fn get_uint(&mut self, nbytes: usize) -> u64 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_uint(nbytes),
            UnionBufEnum::Second(second_buf) => second_buf.get_uint(nbytes),
        }
    }

    fn get_uint_le(&mut self, nbytes: usize) -> u64 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_uint_le(nbytes),
            UnionBufEnum::Second(second_buf) => second_buf.get_uint_le(nbytes),
        }
    }

    fn get_uint_ne(&mut self, nbytes: usize) -> u64 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_uint_ne(nbytes),
            UnionBufEnum::Second(second_buf) => second_buf.get_uint_ne(nbytes),
        }
    }

    fn get_int(&mut self, nbytes: usize) -> i64 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_int(nbytes),
            UnionBufEnum::Second(second_buf) => second_buf.get_int(nbytes),
        }
    }

    fn get_int_le(&mut self, nbytes: usize) -> i64 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_int_le(nbytes),
            UnionBufEnum::Second(second_buf) => second_buf.get_int_le(nbytes),
        }
    }

    fn get_int_ne(&mut self, nbytes: usize) -> i64 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_int_ne(nbytes),
            UnionBufEnum::Second(second_buf) => second_buf.get_int_ne(nbytes),
        }
    }

    fn get_f32(&mut self) -> f32 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_f32(),
            UnionBufEnum::Second(second_buf) => second_buf.get_f32(),
        }
    }

    fn get_f32_le(&mut self) -> f32 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_f32_le(),
            UnionBufEnum::Second(second_buf) => second_buf.get_f32_le(),
        }
    }

    fn get_f32_ne(&mut self) -> f32 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_f32_ne(),
            UnionBufEnum::Second(second_buf) => second_buf.get_f32_ne(),
        }
    }

    fn get_f64(&mut self) -> f64 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_f64(),
            UnionBufEnum::Second(second_buf) => second_buf.get_f64(),
        }
    }

    fn get_f64_le(&mut self) -> f64 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_f64_le(),
            UnionBufEnum::Second(second_buf) => second_buf.get_f64_le(),
        }
    }

    fn get_f64_ne(&mut self) -> f64 {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.get_f64_ne(),
            UnionBufEnum::Second(second_buf) => second_buf.get_f64_ne(),
        }
    }

    fn copy_to_bytes(&mut self, len: usize) -> bytes::Bytes {
        match &mut self.inner {
            UnionBufEnum::First(first_buf) => first_buf.copy_to_bytes(len),
            UnionBufEnum::Second(second_buf) => second_buf.copy_to_bytes(len),
        }
    }
}

enum UnionBufEnum<FirstBuf: Buf, SecondBuf: Buf> {
    First(FirstBuf),
    Second(SecondBuf),
}
