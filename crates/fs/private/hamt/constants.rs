use semver::Version;

pub const HAMT_BITMASK_BIT_SIZE: usize = 16;
pub const HAMT_BITMASK_BYTE_SIZE: usize = HAMT_BITMASK_BIT_SIZE / 8;
pub const HAMT_VALUES_BUCKET_SIZE: usize = 3;
pub const HAMT_VERSION: Version = Version::new(0, 1, 0);
