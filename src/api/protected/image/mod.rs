pub(crate) mod image;
mod normalizer;
mod validator;

pub const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;
pub const MAX_REQUEST_SIZE: usize = MAX_IMAGE_SIZE + 1024 * 1024;
