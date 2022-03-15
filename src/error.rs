use thiserror::Error;
#[derive(Error, Debug)]

pub enum IbvContextError{
   #[error("NoDevice")]
   NoDevice,
   #[error("OpenDeviceError")]
   OpenDeviceError, 
}