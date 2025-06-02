use std::task::Poll;

use pin_project_lite::pin_project;
use rama_core::error::BoxError;
use rama_http_types::{
    Response,
    dep::http_body::{Body, Frame, SizeHint},
};

use super::union_buf::UnionBuf;

// pub(crate) type UnionBody3<FirstBody, SecondBody, ThirdBody> =
//     UnionBody<FirstBody, UnionBody<SecondBody, ThirdBody>>;

// impl<
//         FirstBody: Body<Error: Into<BoxError>>,
//         SecondBody: Body<Error: Into<BoxError>>,
//         ThirdBody: Body<Error: Into<BoxError>>,
//     > UnionBody3<FirstBody, SecondBody, ThirdBody>
// {
//     pub fn first_of_3(first_body: FirstBody) -> UnionBody3<FirstBody, SecondBody, ThirdBody> {
//         return UnionBody::first(first_body);
//     }

//     pub fn second_of_3(second_body: SecondBody) -> UnionBody3<FirstBody, SecondBody, ThirdBody> {
//         return UnionBody::second(UnionBody::first(second_body));
//     }

//     pub fn third_of_3(third_body: ThirdBody) -> UnionBody3<FirstBody, SecondBody, ThirdBody> {
//         return UnionBody::second(UnionBody::second(third_body));
//     }
// }

pin_project! {
    pub struct UnionBody<
        FirstResponseBody: Body<Error: Into<BoxError>>,
        SecondResponseBody: Body<Error: Into<BoxError>>,
    > {
        #[pin] inner: UnionBodyEnum<FirstResponseBody, SecondResponseBody>,
    }
}

impl<
    FirstResponseBody: Body<Error: Into<BoxError>>,
    SecondResponseBody: Body<Error: Into<BoxError>>,
> UnionBody<FirstResponseBody, SecondResponseBody>
{
    pub fn first(
        first_body: FirstResponseBody,
    ) -> UnionBody<FirstResponseBody, SecondResponseBody> {
        return UnionBody {
            inner: UnionBodyEnum::First { body: first_body },
        };
    }

    pub fn second(
        second_body: SecondResponseBody,
    ) -> UnionBody<FirstResponseBody, SecondResponseBody> {
        return UnionBody {
            inner: UnionBodyEnum::Second { body: second_body },
        };
    }

    pub fn into_variant(self) -> UnionBodyVariant<FirstResponseBody, SecondResponseBody> {
        return match self.inner {
            UnionBodyEnum::First { body } => UnionBodyVariant::First(body),
            UnionBodyEnum::Second { body } => UnionBodyVariant::Second(body),
        };
    }

    pub fn get_first_union_response(response: Response<FirstResponseBody>) -> Response<UnionBody<FirstResponseBody, SecondResponseBody>> {
        let (head, body) = response.into_parts();

        return Response::from_parts(head, UnionBody::first(body));
    }

    pub fn get_second_union_response(response: Response<SecondResponseBody>) -> Response<UnionBody<FirstResponseBody, SecondResponseBody>> {
        let (head, body) = response.into_parts();

        return Response::from_parts(head, UnionBody::second(body));
    }
}

impl<
    FirstResponseBody: Body<Error: Into<BoxError>>,
    SecondResponseBody: Body<Error: Into<BoxError>>,
> Body for UnionBody<FirstResponseBody, SecondResponseBody>
{
    type Data = UnionBuf<FirstResponseBody::Data, SecondResponseBody::Data>;

    type Error = BoxError;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        return match self.project().inner.project() {
            UnionBodyEnumProj::First { body } => match body.poll_frame(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Ready(Some(Ok(frame))) => {
                    Poll::Ready(Some(Ok(frame.map_data(|data| UnionBuf::first(data)))))
                }
                Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error.into()))),
            },
            UnionBodyEnumProj::Second { body } => match body.poll_frame(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Ready(Some(Ok(frame))) => {
                    Poll::Ready(Some(Ok(frame.map_data(|data| UnionBuf::second(data)))))
                }
                Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error.into()))),
            },
        };
    }

    fn is_end_stream(&self) -> bool {
        return match &self.inner {
            UnionBodyEnum::First { body } => body.is_end_stream(),
            UnionBodyEnum::Second { body } => body.is_end_stream(),
        };
    }

    fn size_hint(&self) -> SizeHint {
        return match &self.inner {
            UnionBodyEnum::First { body } => body.size_hint(),
            UnionBodyEnum::Second { body } => body.size_hint(),
        };
    }
}

pin_project! {
    #[project = UnionBodyEnumProj]
    enum UnionBodyEnum<FirstResponseBody: Body<Error:Into<BoxError>>, SecondResponseBody: Body<Error:Into<BoxError>>> {
        First{#[pin] body: FirstResponseBody },
        Second{#[pin] body: SecondResponseBody },
    }
}

pub enum UnionBodyVariant<
    FirstResponseBody: Body<Error: Into<BoxError>>,
    SecondResponseBody: Body<Error: Into<BoxError>>,
> {
    First(FirstResponseBody),
    Second(SecondResponseBody),
}
