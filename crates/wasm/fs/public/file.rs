//! The bindgen API for PublicFile.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use js_sys::Error;
use wasm_bindgen::prelude::wasm_bindgen;
use wnfs::{public::PublicFile as WnfsPublicFile, Cid};

use crate::fs::JsResult;

/// A file in a WNFS public file system.
#[wasm_bindgen]
pub struct PublicFile(WnfsPublicFile);

#[wasm_bindgen]
impl PublicFile {
    /// Creates a new file in a WNFS public file system.
    #[wasm_bindgen(constructor)]
    pub fn new(time: &js_sys::Date, cid: &str) -> JsResult<PublicFile> {
        let time = DateTime::<Utc>::from(time);
        let cid = Cid::from_str(cid).map_err(|_| Error::new("Invalid CID"))?;
        Ok(PublicFile(WnfsPublicFile::new(time, cid)))
    }
}

#[cfg(test)]
mod public_file_tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn it_can_create_file() {
        let time = &js_sys::Date::new_0();

        let cid = Cid::default();

        let file = PublicFile::new(time, &cid.to_string());

        assert!(file.is_ok());
    }
}
