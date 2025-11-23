use core::{iter, mem, ptr};

/// Allows interleaving samples, from a set of jack ports,
/// but allocates space for the pointers only once
#[repr(transparent)]
pub struct Interleaver<Spec> {
    ptrs: [(jack::Port<Spec>, ptr::NonNull<f32>)],
}

// TODO: What's the safety argument here?
unsafe impl<Spec> Send for Interleaver<Spec> {}

impl<Spec> Interleaver<Spec> {
    #[inline(always)]
    pub fn new(ports: impl IntoIterator<Item = jack::Port<Spec>>) -> Box<Self> {
        let boxed_slice = Box::from_iter(iter::zip(
            ports,
            iter::repeat_with(ptr::NonNull::<f32>::dangling),
        ));

        // SAFETY: We are a `#[repr(transparent)]` struct
        unsafe { mem::transmute(boxed_slice) }
    }

    #[inline(always)]
    pub fn from_mut(slice: &mut [(jack::Port<Spec>, ptr::NonNull<f32>)]) -> &mut Self {
        // SAFETY: We are a `#[repr(transparent)]` struct, lifetimes are well-defined
        unsafe { mem::transmute(slice) }
    }

    #[inline(always)]
    pub fn split_at_mut_checked(&mut self, mid: usize) -> Option<(&mut Self, &mut Self)> {
        self.ptrs.split_at_mut_checked(mid).map(|(a, b)| (Self::from_mut(a), Self::from_mut(b)))
    }
}

mod private {
    pub trait Sealed {}
    impl Sealed for jack::AudioIn {}
    impl Sealed for jack::AudioOut {}
}

pub trait ToJackPointer: private::Sealed {
    fn to_jack_buf_ptr(port: &mut jack::Port<Self>, scope: &jack::ProcessScope) -> ptr::NonNull<f32>
    where
        Self: Sized;
}

impl ToJackPointer for jack::AudioIn {
    #[inline(always)]
    fn to_jack_buf_ptr(port: &mut jack::Port<Self>, scope: &jack::ProcessScope) -> ptr::NonNull<f32>
    where
        Self: Sized,
    {
        ptr::NonNull::new(port.as_slice(scope).as_ptr().cast_mut()).unwrap()
    }
}

impl ToJackPointer for jack::AudioOut {
    #[inline(always)]
    fn to_jack_buf_ptr(port: &mut jack::Port<Self>, scope: &jack::ProcessScope) -> ptr::NonNull<f32>
    where
        Self: Sized,
    {
        ptr::NonNull::new(port.as_mut_slice(scope).as_ptr().cast_mut()).unwrap()
    }
}

pub trait FromJackPointer: private::Sealed {
    type Output<'a>;
    unsafe fn get_ref<'a>(ptr: ptr::NonNull<f32>) -> Self::Output<'a>;
}

impl FromJackPointer for jack::AudioIn {
    type Output<'a> = &'a f32;

    #[inline(always)]
    unsafe fn get_ref<'a>(ptr: ptr::NonNull<f32>) -> Self::Output<'a> {
        unsafe { ptr.as_ref() }
    }
}

impl FromJackPointer for jack::AudioOut {
    type Output<'a> = &'a mut f32;

    #[inline(always)]
    unsafe fn get_ref<'a>(mut ptr: ptr::NonNull<f32>) -> Self::Output<'a> {
        unsafe { ptr.as_mut() }
    }
}

impl<Spec: ToJackPointer> Interleaver<Spec> {
    #[inline(always)]
    pub fn interleave(&mut self, process_scope: &jack::ProcessScope) -> Interleaved<'_, Spec> {
        // write the pointers into our list

        for (port, ptr) in &mut self.ptrs.iter_mut() {
            *ptr = Spec::to_jack_buf_ptr(port, process_scope);
        }

        let remaining_frames = if self.ptrs.len() == 0 {
            0
        } else {
            process_scope.n_frames()
        };

        Interleaved {
            remaining_frames,
            current_index: 0,
            ptrs: &mut self.ptrs,
        }
    }
}

pub struct Interleaved<'a, Spec> {
    remaining_frames: u32,
    current_index: usize,
    ptrs: &'a mut [(jack::Port<Spec>, ptr::NonNull<f32>)],
}

impl<'a, Spec: FromJackPointer> Iterator for Interleaved<'a, Spec> {
    type Item = Spec::Output<'a>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_frames == 0 {
            return None;
        }

        self.remaining_frames -= 1;
        // SAFETY: current_idx starts at 0 and wraps around at ptrs.len
        // + remaining_frames != 0 --> ptrs.len() != 0
        let (_port, ptr_ref) = unsafe { self.ptrs.get_unchecked_mut(self.current_index) };
        let ptr = *ptr_ref;
        // SAFETY: happens at most remaining_frames times
        // ensuring we're within the buffer's bounds
        *ptr_ref = unsafe { ptr_ref.add(1) };
        // SAFETY: never overflows
        self.current_index = unsafe { self.current_index.unchecked_add(1) };
        Some(unsafe { Spec::get_ref(ptr) })
    }
}
