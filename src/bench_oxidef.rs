/// Converts a dataset type into one of its Oxidef-generated equivalents.
///
/// Each dataset type implements this once for the schema using `final` types and once for the
/// schema using `extensible` types.
pub trait Serialize<M> {
    fn serialize_oxidef(&self) -> M;
}

/// Defines `bench` against one copy of the Oxidef runtime crates, so that two versions of Oxidef
/// can be benchmarked side by side under different names.
macro_rules! impl_bench {
    ($compact1:ident, $validation:ident, $crate_name:literal) => {
        use super::Serialize;
        use criterion::{black_box, measurement::WallTime, BenchmarkGroup, Criterion};
        use $compact1::codec::Compact1Codec;
        use $validation::Validate;

        /// Benchmarks the `final` schema as the primary results and the `extensible` schema as
        /// the `(extensible)` variant, so the overhead of extensibility can be compared directly.
        pub fn bench<T, F, E>(name: &'static str, c: &mut Criterion, data: &T)
        where
            T: Serialize<F> + Serialize<E> + PartialEq,
            F: Compact1Codec + Validate + Into<T>,
            E: Compact1Codec + Validate + Into<T>,
        {
            let mut group = c.benchmark_group(format!("{}/{}", name, $crate_name));

            bench_schema::<T, F>(name, &mut group, data, None);
            bench_schema::<T, E>(name, &mut group, data, Some("extensible"));

            group.finish();
        }

        fn bench_schema<T, M>(
            name: &'static str,
            group: &mut BenchmarkGroup<'_, WallTime>,
            data: &T,
            variant: Option<&str>,
        ) where
            T: Serialize<M> + PartialEq,
            M: Compact1Codec + Validate + Into<T>,
        {
            let suffix = variant.map(|v| format!(" ({v})")).unwrap_or_default();

            let message: M = data.serialize_oxidef();
            group.bench_function(format!("serialize{suffix}"), |b| {
                b.iter(|| {
                    black_box($compact1::encode(black_box(&message)).unwrap());
                })
            });

            let deserialize_buffer = $compact1::encode(&message).unwrap();

            group.bench_function(format!("deserialize{suffix}"), |b| {
                b.iter(|| {
                    black_box($compact1::decode::<M>(black_box(&deserialize_buffer)).unwrap());
                })
            });

            match variant {
                Some(variant) => crate::bench_size_variant(
                    name,
                    $crate_name,
                    variant,
                    deserialize_buffer.as_slice(),
                ),
                None => crate::bench_size(name, $crate_name, deserialize_buffer.as_slice()),
            }

            assert!($compact1::decode::<M>(&deserialize_buffer).unwrap().into() == *data);
        }
    };
}

#[cfg(feature = "oxidef")]
mod current {
    impl_bench!(oxidef_compact1, oxidef_validation, "oxidef");
}

#[cfg(feature = "oxidef_old")]
mod old {
    impl_bench!(oxidef_compact1_old, oxidef_validation_old, "oxidef_old");
}

/// Benchmarks the current version of Oxidef.
#[cfg(feature = "oxidef")]
pub use current::bench;

/// Benchmarks the older version of Oxidef, under the name `oxidef_old`.
#[cfg(feature = "oxidef_old")]
pub use old::bench as bench_old;

// oxidef does not support borrowed decoding.
