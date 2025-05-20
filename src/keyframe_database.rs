use idb::{Database, DatabaseEvent, Error, Factory,  KeyPath, ObjectStoreParams, TransactionMode};
use std::sync::Arc;
use wasm_bindgen::JsValue;

use crate::keyframe::KeyframeChunk;
pub struct KeyframeDatabase {
    db: Arc<Database>,
}

impl KeyframeDatabase {
    pub async fn new() -> Result<Arc<Self>, Error> {
        let factory = Factory::new()?;
        let db_name = "keyframe_db";
        let db_version = 1;
        factory.delete(db_name)?.await?;

        let mut open_req = factory.open(db_name, Some(db_version))?;

        open_req.on_upgrade_needed(|event| {
            let db = event.database().unwrap();
            let mut params = ObjectStoreParams::new();
            params.key_path(Some(KeyPath::new_single("object_chunk_id")));
            params.auto_increment(true);
            let store = db.create_object_store("keyframe_chunks", params).unwrap();
            store.create_index(
                "by_object_chunk_id",
                KeyPath::new_single("object_chunk_id"),
                None,
            ).unwrap();
        });

        let raw_db: Database = open_req.await?;
        let db = Arc::new(raw_db);
        Ok(Arc::new(Self { db }))
    }

    pub async fn save_chunks(
        &self,
        chunks: Vec<KeyframeChunk>
    ) -> Result<(), Error> {
        if chunks.is_empty() {
            return Ok(());
        }

        const BATCH_SIZE: usize = 200;

        for chunk_batch in chunks.chunks(BATCH_SIZE) {
            let tx = self.db.transaction(&["keyframe_chunks"], TransactionMode::ReadWrite)?;
            let store = tx.object_store("keyframe_chunks")?;

            for chunk in chunk_batch {
                let js_val: JsValue = serde_wasm_bindgen::to_value(chunk)
                    .map_err(|e| Error::AddFailed(JsValue::from_str(&format!("Serialization error: {:?}", e))))?;

                let req = store.put(&js_val, None)?;
                req.await?;
            }

            tx.commit()?;

            gloo_timers::future::TimeoutFuture::new(0).await;
        }

        Ok(())
    }

    pub async fn load_chunk(
        &self,
        object_id: &str,
        chunk_id: u32,
    ) -> Result<KeyframeChunk, Error> {
        let key_str = format!("{}_{}", object_id, chunk_id);
        let js_key = JsValue::from_str(&key_str);

        let tx = self.db.transaction(&["keyframe_chunks"], TransactionMode::ReadOnly)?;
        let store = tx.object_store("keyframe_chunks")?;
        let req = store.get(js_key)?;

        // Await the request
        let maybe = req.await?;
        
        if let Some(js_val) = maybe {
            let chunk: KeyframeChunk = serde_wasm_bindgen::from_value(js_val).unwrap();
            Ok(chunk)
        } else {
            Err(Error::AddFailed(JsValue::from_str(
                &format!("No chunk found for key '{}'", key_str),
            )))
        }
    }
}