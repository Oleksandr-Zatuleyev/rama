// use bytes::{Buf, Bytes};

// pub fn copy_buf(mut buf: impl Buf) -> (Box<dyn Buf>, Box<dyn Buf>) {
//     // TODO: Chain<Bytes, Box<dyn Buf>> is not really efficient - maybe implement on Vec<Bytes>?

//     if buf.remaining() == buf.chunk().len() {
//         let bytes = buf.copy_to_bytes(buf.remaining());
//         return (Box::new(bytes.clone()), Box::new(bytes));
//     }

//     let mut byte_vec = Vec::new();

//     while buf.has_remaining() {
//         let chunk_len = buf.chunk().len();

//         let chunk_bytes = buf.copy_to_bytes(chunk_len);

//         byte_vec.push(chunk_bytes);
//     }

//     return (chain_byte_vec(&byte_vec), chain_byte_vec(&byte_vec));
// }

// fn chain_byte_vec(byte_vec: &Vec<Bytes>) -> Box<dyn Buf> {
//     if byte_vec.len() == 0 {
//         return Box::new(Bytes::new());
//     }

//     if byte_vec.len() == 1 {
//         return Box::new(byte_vec[0].clone());
//     }

//     let mut current: Box<dyn Buf> = Box::new(byte_vec.last().unwrap().clone());
//     for bytes in byte_vec.iter().rev().skip(1) {
//         current = Box::new(bytes.clone().chain(current));
//     }

//     return current;
// }

// struct ByteVecWrapper {
//     data: VecDeque<Bytes>,
//     empty: [u8; 0],
//     remaining: usize,
// }

// impl Buf for ByteVecWrapper {
//     fn remaining(&self) -> usize {
//         // return self.data.iter().map(|b| b.remaining()).sum();
//         return self.remaining;
//     }

//     fn chunk(&self) -> &[u8] {
//         if self.data.len() > 0 {
//             return self.data[0].chunk();
//         }

//         return &self.empty;
//     }

//     fn advance(&mut self, mut cnt: usize) {
//         assert!(
//             cnt <= self.remaining,
//             "cannot advance past `remaining`: {:?} <= {:?}",
//             cnt,
//             self.remaining,
//         );

//         self.remaining -= cnt;

//         while cnt > 0 {
//             let cur_remaining = self.data[0].remaining();

//             if cur_remaining >= cnt {
//                 self.data[0].advance(cnt);
//                 cnt -= cur_remaining;
//                 continue;
//             }

//             self.data[0].advance(cur_remaining);
//             self.data.pop_front();

//             cnt -= cur_remaining;
//         }
//     }
// }
