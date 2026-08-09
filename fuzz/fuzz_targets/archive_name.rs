#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| iroha_zip::fuzzing::archive_name(input));
