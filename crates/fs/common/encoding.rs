/// Helper methods for decoding and encoding values into DagCbor.
pub mod dagcbor {
    use std::io::Cursor;

    use anyhow::Result;

    use libipld::{
        cbor::DagCborCodec,
        codec::{Decode, Encode},
        serde as ipld_serde, Ipld,
    };
    use serde::{de::DeserializeOwned, Serialize};

    use crate::{AsyncSerialize, BlockStore};

    /// Encodes a serializable value into DagCbor bytes.
    pub fn encode<S: Serialize>(value: &S) -> Result<Vec<u8>> {
        let ipld = ipld_serde::to_ipld(value)?;
        let mut bytes = Vec::new();
        ipld.encode(DagCborCodec, &mut bytes)?;
        Ok(bytes)
    }

    /// Encodes an async serializable value into DagCbor bytes.
    pub async fn async_encode<V: AsyncSerialize, B: BlockStore>(
        value: &V,
        store: &mut B,
    ) -> Result<Vec<u8>> {
        let ipld = value.async_serialize_ipld(store).await?;
        let mut bytes = Vec::new();
        ipld.encode(DagCborCodec, &mut bytes)?;
        Ok(bytes)
    }

    /// Decodes recieved DagCbor bytes into a deserializable value.
    pub fn decode<D: DeserializeOwned>(bytes: &[u8]) -> Result<D> {
        let ipld = Ipld::decode(DagCborCodec, &mut Cursor::new(bytes))?;
        Ok(ipld_serde::from_ipld::<_>(ipld)?)
    }
}
