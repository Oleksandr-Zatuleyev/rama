use bytes::{Buf, Bytes};
use rama_http_types::dep::http_body::Frame;

pub(crate) fn convert_buf_to_byte_frame(
    frame: Frame<impl Buf>,
) -> Frame<Bytes> {

    return frame.map_data(|mut buf| buf.copy_to_bytes(buf.remaining()));
}


pub(crate) fn try_clone_byte_frame(
    frame: &Frame<Bytes>,
) -> Option<Frame<Bytes>> {

    if let Some(bytes) = frame.data_ref() {
        return Some(Frame::data(bytes.clone()));
    }

    if let Some(trailers) = frame.trailers_ref() {
        return Some(Frame::trailers(trailers.clone()));
    }

    return None;
}
