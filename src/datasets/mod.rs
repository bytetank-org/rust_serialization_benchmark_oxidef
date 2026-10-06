pub mod log;
pub mod mesh;
pub mod minecraft_savedata;
pub mod mk48;

#[cfg(feature = "oxidef")]
#[allow(clippy::all)]
pub mod oxidef_generated {
    include!(concat!(env!("OUT_DIR"), "/oxidef/mod.rs"));
}

#[cfg(feature = "oxidef_old")]
#[allow(clippy::all)]
pub mod oxidef_old_generated {
    include!(concat!(env!("OUT_DIR"), "/oxidef_old/mod.rs"));
}

/// Trait for test data types that have a form with borrowed fields.
pub trait BorrowableData: Sized + PartialEq {
    type Borrowed<'a>: PartialEq + From<&'a Self> + Into<Self>
    where
        Self: 'a;
}
