use std::{collections::VecDeque, ops::DerefMut, task::Poll};

use bytes::Bytes;
use rama_core::error::BoxError;
use rama_http_types::{
    HeaderMap,
    dep::http_body::{Body, Frame},
};

use crate::layer::util::try_clone_byte_frame::try_clone_byte_frame;

pub struct ByteBody {
    data: VecDeque<Frame<Bytes>>,
}

impl ByteBody {
    pub(crate) fn from_frame_vec(data: &Vec<Frame<Bytes>>) -> Option<ByteBody> {
        let mut response_data = VecDeque::new();

        for frame in data {
            let Some(cloned_frame) = try_clone_byte_frame(&frame) else {
                return None;
            };

            response_data.push_back(cloned_frame);
        }

        return Some(ByteBody {
            data: response_data,
        });
    }

    pub(crate) fn from_bytes_and_trailers(
        data: &Vec<Bytes>,
        trailers: Option<HeaderMap>,
    ) -> ByteBody {
        let mut data: VecDeque<Frame<Bytes>> = data
            .iter()
            .map(|bytes| Frame::data(bytes.clone()))
            .collect();

        if let Some(trailers) = trailers {
            data.push_back(Frame::trailers(trailers));
        }

        return ByteBody { data };
    }
}

impl Body for ByteBody {
    type Data = Bytes;

    type Error = BoxError;

    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let Some(frame) = self.deref_mut().data.pop_front() else {
            return Poll::Ready(None);
        };

        return Poll::Ready(Some(Ok(frame)));
    }
}
