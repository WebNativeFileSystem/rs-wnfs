use crate::fs::JsResult;
use crate::value;
use js_sys::{Object, Reflect};
use wasm_bindgen::JsValue;
use wnfs::{Metadata, UnixFsMetadata, UnixFsNodeKind};

pub(crate) struct JsMetadata<'a>(pub(crate) &'a Metadata);

impl TryInto<JsValue> for JsMetadata<'_> {
    type Error = js_sys::Error;

    fn try_into(self) -> JsResult<JsValue> {
        let metadata = Object::new();
        let unix_meta = unix_fs_to_js_value(&self.0.unix_fs)?;

        Reflect::set(&metadata, &value!("unixMeta"), &unix_meta)?;
        Reflect::set(&metadata, &value!("version"), &value!("3.0.0"))?;

        Ok(value!(metadata))
    }
}

fn unix_fs_to_js_value(unix_fs: &UnixFsMetadata) -> JsResult<JsValue> {
    let obj = Object::new();
    let kind = unix_fs_kind_to_js_value(&unix_fs.kind);

    Reflect::set(&obj, &value!("created"), &value!(unix_fs.created))?;
    Reflect::set(&obj, &value!("modified"), &value!(unix_fs.modified))?;
    Reflect::set(&obj, &value!("mode"), &value!(unix_fs.mode.clone() as u32))?;
    Reflect::set(&obj, &value!("kind"), &kind)?;

    Ok(value!(obj))
}

fn unix_fs_kind_to_js_value(kind: &UnixFsNodeKind) -> JsValue {
    match kind {
        UnixFsNodeKind::Raw => value!("raw"),
        UnixFsNodeKind::Dir => value!("dir"),
        UnixFsNodeKind::File => value!("file"),
        UnixFsNodeKind::Metadata => value!("metadata"),
        UnixFsNodeKind::SymLink => value!("symlink"),
        UnixFsNodeKind::HAMTShard => value!("hamtShard"),
    }
}
