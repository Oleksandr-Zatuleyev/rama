// use http_body::{Body, Frame};
use pin_project_lite::pin_project;
use rama_http_types::dep::http_body::{Body, Frame};

pin_project! {
    pub(crate) struct BodySreamWrapper<InnerBody: Body> {
        #[pin]
        inner_body: InnerBody
    }
}

impl<InnerBody: Body> BodySreamWrapper<InnerBody> {
    pub(crate) fn new(inner_body: InnerBody) -> BodySreamWrapper<InnerBody> {
        return BodySreamWrapper {
            inner_body
        };
    }
}

impl<InnerBody: Body> tokio_stream::Stream for BodySreamWrapper<InnerBody> {
    type Item = Result<Frame<InnerBody::Data>, InnerBody::Error>;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let inner_pin = self.as_mut().project().inner_body;

        return inner_pin.poll_frame(cx);
    }
}