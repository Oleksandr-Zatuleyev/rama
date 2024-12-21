//! Http Layer Utilities.

#[cfg(feature = "compression")]
pub(crate) mod compression;

pub(crate) mod content_encoding;

pub(crate) mod multicast_body;

pub(crate) mod try_clone_byte_frame;

pub(crate) mod intercept_body;

pub(crate) mod union_buf;

pub(crate) mod union_body;

pub(crate) mod copy_buf;

pub(crate) mod body_stream_wrapper;