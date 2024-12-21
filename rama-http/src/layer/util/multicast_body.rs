// use std::{
//     error::Error,
//     mem,
//     ops::{Deref, DerefMut},
//     sync::{Arc, RwLock},
// };

// use bytes::Bytes;
// use http_body::{Body, Frame};
// use rama_core::error::{error, BoxError};
// use tokio::sync::broadcast::{self};

// use crate::layer::util::try_clone_frame::try_clone_frame;

// pub(crate) struct MulticastBodySource {
//     state: Option<MulticastBodySourceState>,
// }

// impl MulticastBodySource {
//     pub(crate) fn new() -> MulticastBodySource {
//         return MulticastBodySource {
//             state: Some(MulticastBodySourceState::InProgress {
//                 shared_state: Arc::new(RwLock::new(SharedState {
//                     frames: Vec::new(),
//                     is_completed: false,
//                     error: None,
//                 })),
//                 wake_sender: broadcast::Sender::new(1),
//             }),
//         };
//     }

//     pub(crate) fn get_body(&self) -> MulticastBody {
//         return match self.state.as_ref().unwrap() {
//             MulticastBodySourceState::InProgress {
//                 shared_state,
//                 wake_sender,
//             } => MulticastBody {
//                 shared_state: shared_state.clone(),
//                 wake_receiver: Some(wake_sender.subscribe()),
//                 processed_frame_num: 0,
//             },
//             MulticastBodySourceState::Failed { shared_state } => MulticastBody {
//                 shared_state: shared_state.clone(),
//                 wake_receiver: None,
//                 processed_frame_num: 0,
//             },
//             MulticastBodySourceState::Completed { shared_state } => MulticastBody {
//                 shared_state: shared_state.clone(),
//                 wake_receiver: None,
//                 processed_frame_num: 0,
//             },
//         };
//     }

//     pub(crate) fn add_frame(&mut self, frame: Frame<Bytes>) -> Result<(), Box<dyn Error>> {
//         match &self.state {
//             Some(MulticastBodySourceState::InProgress {
//                 shared_state: shared_state_ref,
//                 wake_sender: frame_sender,
//             }) => {
//                 {
//                     let mut shared_state = shared_state_ref.write().unwrap();
//                     let shared_state = shared_state.deref_mut();

//                     shared_state.frames.push(frame);
//                 }

//                 frame_sender.send(()).ok();
//             }
//             _ => {
//                 return Err(error!(
//                     "Incorrect state - the source is no longer waiting for additional data"
//                 )
//                 .into_boxed());
//             }
//         }

//         return Ok(());
//     }

//     pub(crate) fn fail(&mut self, error: impl Error + Send + Sync + 'static) {
//         let mut state = None;
//         mem::swap(&mut state, &mut self.state);

//         let mut state = state.unwrap();
//         state = match state {
//             MulticastBodySourceState::InProgress {
//                 shared_state: shared_state_ref,
//                 wake_sender,
//             } => {
//                 {
//                     let mut shared_state = shared_state_ref.write().unwrap();

//                     let shared_state = shared_state.deref_mut();
//                     shared_state.error = Some(Arc::new(error));
//                 }

//                 wake_sender.send(()).ok();

//                 MulticastBodySourceState::Failed {
//                     shared_state: shared_state_ref,
//                 }
//             }
//             _ => state,
//         };

//         self.state = Some(state);
//     }

//     pub(crate) fn complete(&mut self) {
//         let mut state = None;
//         mem::swap(&mut state, &mut self.state);

//         let mut state = state.unwrap();

//         state = match state {
//             MulticastBodySourceState::InProgress {
//                 shared_state: shared_state_ref,
//                 wake_sender,
//             } => {
//                 {
//                     let mut shared_state = shared_state_ref.write().unwrap();

//                     let shared_state = shared_state.deref_mut();
//                     shared_state.is_completed = true;
//                 }

//                 wake_sender.send(()).ok();

//                 MulticastBodySourceState::Completed {
//                     shared_state: shared_state_ref,
//                 }
//             }
//             _ => state,
//         };

//         self.state = Some(state);
//     }
// }

// pub(crate) struct MulticastBody {
//     shared_state: Arc<RwLock<SharedState>>,
//     wake_receiver: Option<broadcast::Receiver<()>>,
//     processed_frame_num: usize,
// }

// impl Body for MulticastBody {
//     type Data = Bytes;

//     type Error = BoxError;

//     fn poll_frame(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
//         {
//             let shared_state = self.shared_state.read().unwrap();
//             let shared_state = shared_state.deref();

//             if self.processed_frame_num < shared_state.frames.len() {
//                 let cloned_frame = try_clone_frame(&shared_state.frames[self.processed_frame_num])?;
//                 return std::task::Poll::Ready(Some(Ok(cloned_frame)));
//             }

//             if shared_state.is_completed {
//                 return std::task::Poll::Ready(None);
//             }

//             if let Some(ref error) = shared_state.error {
//                 let error: BoxError = Box::new(error.clone());
//                 return std::task::Poll::Ready(Some(Err(error)));
//             }
//         }

//         panic!();
//     }
// }

// enum MulticastBodySourceState {
//     InProgress {
//         shared_state: Arc<RwLock<SharedState>>,
//         wake_sender: broadcast::Sender<()>,
//     },
//     Completed {
//         shared_state: Arc<RwLock<SharedState>>,
//     },
//     Failed {
//         shared_state: Arc<RwLock<SharedState>>,
//     },
// }

// struct SharedState {
//     frames: Vec<Frame<Bytes>>,
//     is_completed: bool,
//     error: Option<Arc<dyn Error + Send + Sync>>,
// }
