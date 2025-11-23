use core::num;
pub mod interleaver;

pub const fn as_bytes(data: &[Sample]) -> &[u8] {
    // SAFETY: all bit patterns for u8 are valid, references have same lifetime
    unsafe {
        core::slice::from_raw_parts(
            data.as_ptr().cast(),
            data.len().strict_mul(SAMPLE_SIZE.get()),
        )
    }
}

pub const fn nz(x: usize) -> num::NonZeroUsize {
    num::NonZeroUsize::new(x).unwrap()
}

pub const DEFAULT_PORT: u16 = 6910;

pub type Sample = f32;
pub const SILENCE: Sample = 0.;

pub const SAMPLE_SIZE: num::NonZeroUsize = nz(size_of::<Sample>());

pub const SAMPLE_RATE: f64 = 48000.;

// This has to be big enough to accomodate at least a couple of process cycles
// Needless to say, those can reach very high buffer sizes.
// We choose four seconds here
pub const RB_SIZE_SECONDS: f64 = 4.;

pub const RB_SIZE_FRAMES: num::NonZeroUsize = nz((SAMPLE_RATE * RB_SIZE_SECONDS) as usize);

pub const MAX_DATAGRAM_SIZE: num::NonZeroUsize = nz(1452);

pub const MAX_SAMPLE_DATA_PER_DATAGRAM: num::NonZeroUsize =
    nz(MAX_DATAGRAM_SIZE.get().strict_sub(size_of::<u64>()));

pub const MAX_SPLS_PER_DATAGRAM: num::NonZeroUsize =
    nz(MAX_SAMPLE_DATA_PER_DATAGRAM.get() / SAMPLE_SIZE.get());

pub const DEFAULT_N_PORTS: num::NonZeroUsize = num::NonZeroUsize::MIN; // AKA 1