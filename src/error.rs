use thiserror::Error;
use winapi::shared::ntdef::NTSTATUS;
use winsafe::co::ERROR;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    WinSafe(#[from] ERROR),

    #[error("NT API errored with NTSTATUS: {0}")]
    NtApi(NTSTATUS),

    #[error("Attempted to query memory info beyond appropriate module")]
    AddressBeyondModule,

    #[error("Only {0} bytes were read out of {1}")]
    PartialMemoryRead(usize, usize),

    #[error(transparent)]
    FS(#[from] std::io::Error),
}
