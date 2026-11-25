//! `dram-format`: File format readers and writers for `.dram`, Fountain 1.1, and (Phase 3) Fade In.

#![forbid(unsafe_code)]

pub mod dram_archive;
pub mod error;
pub mod fadein;
pub mod fdx;
pub mod fountain;

pub use dram_archive::{
    load_dram, read_dram_from_reader, read_dram_from_reader_with_limits, read_dram_manifest,
    recovery_path_for, remove_recovery, save_dram, write_atomic_bytes, write_dram_to_writer,
    write_dram_to_writer_with_generation, write_recovery, ArchiveLimits, DramManifest,
    RecoveryCoordinator, CURRENT_FORMAT_VERSION, MAX_ASSET_BYTES, MAX_ASSET_COUNT, MAX_JSON_BYTES,
    MAX_TOTAL_UNCOMPRESSED_BYTES,
};
pub use error::FormatError;
pub use fadein::{load_fadein, parse_osf_xml, save_fadein, serialize_osf_xml};
pub use fdx::{load_fdx, parse_fdx_xml, save_fdx, serialize_fdx_xml};
pub use fountain::{parse_fountain, serialize_fountain};
