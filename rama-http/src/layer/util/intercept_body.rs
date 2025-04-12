use core::fmt;
use std::{
    collections::VecDeque,
    marker::PhantomData,
    ops::DerefMut,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use bytes::Bytes;
use pin_project_lite::pin_project;
use rama_http_types::dep::http_body::{Body, Frame, SizeHint};

use rama_core::error::BoxError;

use super::try_clone_byte_frame::{self, convert_buf_to_byte_frame};

/// Allows to read response body two times without having to buffer the whole response in memory
/// Needed for caching
/// The client will receive frames from [`InterceptBodyResult::intercepted`]
/// The cache will read from [`InterceptBodyResult::intercepting`]
pub(crate) fn intercept_body<InnerBody: Body<Error: Into<BoxError>>>(
    source: InnerBody,
) -> InterceptBodyResult<InnerBody> {
    let intercepting_body_state = InterceptingBodyState {
        frames: VecDeque::new(),
        is_complete: false,
        is_failed: false,
        size_hint: SizeHint::new(),
        intercepting_body_waker: None,
    };

    let intercepting_body_handle = InterceptingBodyHandle {
        state: Arc::new(Mutex::new(intercepting_body_state)),
    };

    let intercepting_body = InterceptingBody {
        handle: intercepting_body_handle,
    };

    let intercepted_body = InterceptedBody::new(
        source,
        InterceptingBodyHandle {
            state: Arc::clone(&intercepting_body.handle.state),
        },
    );

    return InterceptBodyResult {
        intercepted: intercepted_body,
        intercepting: intercepting_body,
    };
}

pub(crate) struct InterceptBodyResult<Inner: Body<Error: Into<BoxError>>> {
    /// Original body wrapper
    pub intercepted: InterceptedBody<Inner>,
    /// This body will receive copies of frames as they are being read from [`InterceptBodyResult::intercepted`]
    pub intercepting: InterceptingBody,
}

pin_project! {
    #[derive(Debug)]
    pub(crate) struct InterceptedBody<Inner: Body<Error: Into<BoxError>>> {
        #[pin]
        inner: Inner,
        handle: InterceptingBodyHandle,
        is_interception_failed: bool
    }
}

impl<Inner: Body<Error: Into<BoxError>>> InterceptedBody<Inner> {
    fn new(inner: Inner, handle: InterceptingBodyHandle) -> Self {
        return InterceptedBody {
            inner,
            handle,
            is_interception_failed: false,
        };
    }
}

impl<Inner: Body<Error: Into<BoxError>>> Body for InterceptedBody<Inner> {
    type Data = Bytes;

    type Error = BoxError;

    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.as_mut().project();

        let size_hint = this.inner.size_hint();

        let poll_result = this.inner.poll_frame(cx);

        return match poll_result {
            Poll::Ready(Some(Ok(frame))) => {
                let byte_frame = convert_buf_to_byte_frame(frame);

                if !(*this.is_interception_failed) {
                    match try_clone_byte_frame::try_clone_byte_frame(&byte_frame) {
                        Some(cloned_frame) => this.handle.add_frame(cloned_frame, size_hint),
                        None => {
                            *this.is_interception_failed = true;
                            this.handle.fail();
                        }
                    }
                }

                Poll::Ready(Some(Ok(byte_frame)))
            }
            Poll::Ready(Some(Err(err))) => {
                this.handle.fail();

                Poll::Ready(Some(Err(err.into())))
            }
            Poll::Ready(None) => {
                this.handle.complete();

                Poll::Ready(None)
            }
            Poll::Pending => {
                this.handle.set_size_hint(size_hint);

                Poll::Pending
            }
        };
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

#[derive(Debug)]
pub(crate) struct InterceptingBody {
    handle: InterceptingBodyHandle,
}

impl Body for InterceptingBody {
    type Data = Bytes;

    type Error = BoxError;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        return self.handle.poll_frame(cx);
    }
}

#[derive(Debug)]
struct InterceptingBodyHandle {
    state: Arc<Mutex<InterceptingBodyState>>,
}

#[derive(Debug)]
struct InterceptingBodyState {
    frames: VecDeque<Frame<Bytes>>,
    is_complete: bool,
    is_failed: bool,
    size_hint: SizeHint,
    intercepting_body_waker: Option<Waker>,
}

impl InterceptingBodyHandle {
    fn add_frame(&self, frame: Frame<Bytes>, size_hint: SizeHint) {
        let waker = {
            let mut state_guard = self.state.lock().unwrap();

            let state = state_guard.deref_mut();

            if state.is_failed || state.is_complete {
                return;
            }

            state.size_hint = size_hint;
            state.frames.push_back(frame);
            state.intercepting_body_waker.take()
        };

        Self::try_wake(waker);
    }

    fn complete(&self) {
        let waker = {
            let mut state_guard = self.state.lock().unwrap();

            let state = state_guard.deref_mut();

            if state.is_failed || state.is_complete {
                return;
            }

            state.is_complete = true;
            state.intercepting_body_waker.take()
        };

        Self::try_wake(waker);
    }

    fn fail(&self) {
        let waker = {
            let mut state_guard = self.state.lock().unwrap();

            let state = state_guard.deref_mut();

            if state.is_failed || state.is_complete {
                return;
            }

            state.is_failed = true;
            state.intercepting_body_waker.take()
        };

        Self::try_wake(waker);
    }

    fn set_size_hint(&self, size_hint: SizeHint) {
        let mut state_guard = self.state.lock().unwrap();

        let state = state_guard.deref_mut();

        if state.is_failed || state.is_complete {
            return;
        }

        state.size_hint = size_hint;
    }

    fn try_wake(waker: Option<Waker>) {
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn poll_frame(
        &self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, BoxError>>> {
        let mut state_guard: std::sync::MutexGuard<'_, InterceptingBodyState> =
            self.state.lock().unwrap();
        let state = state_guard.deref_mut();

        if let Some(frame) = state.frames.pop_front() {
            return Poll::Ready(Some(Ok(frame)));
        }

        if state.is_failed {
            return Poll::Ready(Some(Err(Box::new(InterceptionFailedError::new()))));
        }

        if state.is_complete {
            return Poll::Ready(None);
        }

        state.intercepting_body_waker = Some(cx.waker().clone());

        return Poll::Pending;
    }
}

struct InterceptionFailedError {
    _private: PhantomData<u8>,
}

impl InterceptionFailedError {
    fn new() -> InterceptionFailedError {
        return InterceptionFailedError {
            _private: PhantomData,
        };
    }
}

impl fmt::Debug for InterceptionFailedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InterceptionFailedError").finish()
    }
}

impl fmt::Display for InterceptionFailedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return f.write_str("Interception failed");
    }
}

impl std::error::Error for InterceptionFailedError {}
