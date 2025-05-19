use std::collections::HashMap;
use idb::{Database, DatabaseEvent, Error, Factory,  KeyPath, ObjectStoreParams, TransactionMode};
use std::sync::Arc;
use wasm_bindgen::JsValue;

use crate::keyframe::{Keyframe, KeyframeChunk};
pub struct KeyframeDatabase {
    db: Arc<Database>,
}

impl KeyframeDatabase {
    /// IndexedDB 연결 생성
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

        // open 요청 await 후 Rc로 래핑
        let raw_db: Database = open_req.await?;
        let db = Arc::new(raw_db);
        Ok(Arc::new(Self { db }))
    }

    pub async fn save_keyframes_sequentially(
        &self,
        object_id: &str,
        keyframes: Vec<Keyframe>,
        chunk_size: f64,
    ) -> Result<(), Error> {
        let chunks = self.split_into_chunks(object_id, keyframes, chunk_size);
        
        // 데이터가 없으면 바로 반환
        if chunks.is_empty() {
            return Ok(());
        }
        
        // 모든 청크에 대해 하나의 트랜잭션 생성
        let tx = self.db.transaction(&["keyframe_chunks"], TransactionMode::ReadWrite)?;
        let store = tx.object_store("keyframe_chunks")?;
        
        // 각 청크를 동일한 트랜잭션 내에서 저장
        for chunk in chunks {
            let js_val: JsValue = serde_wasm_bindgen::to_value(&chunk)
                .map_err(|e| Error::AddFailed(JsValue::from_str(&format!("Serialization error: {:?}", e))))?;
            
            store.add(&js_val, None)?;
        }
        
        // 모든 작업이 완료된 후 트랜잭션 커밋
        tx.commit()?;

        Ok(())
    }

    pub fn split_into_chunks(
        &self,
        object_id: &str,
        keyframes: Vec<Keyframe>,
        chunk_size: f64,
    ) -> Vec<KeyframeChunk> {
        if keyframes.is_empty() {
            return Vec::new();
        }

        let mut buckets: HashMap<u64, Vec<Keyframe>> = HashMap::new();
        for kf in keyframes {
            let idx = (kf.time() / chunk_size).floor() as u64;
            buckets.entry(idx).or_default().push(kf);
        }

        let mut chunks = Vec::with_capacity(buckets.len());
        for (idx, kfs) in buckets {
            let mut chunk = KeyframeChunk::new(
                &format!("{}_{}", object_id, idx),
                idx as f64 * chunk_size,
                (idx as f64 + 1.0) * chunk_size,
            );
            for kf in kfs {
                chunk.add_keyframe(kf.time(), kf.x(), kf.y());
            }
            chunks.push(chunk);
        }

        chunks
    }

    pub async fn load_chunk(
        &self,
        object_id: &str,
        chunk_id: u32,
    ) -> Result<KeyframeChunk, Error> {
        let key_str = format!("{}_{}", object_id, chunk_id);
        let js_key = JsValue::from_str(&key_str);

        // Start readonly transaction
        let tx = self.db.transaction(&["keyframe_chunks"], TransactionMode::ReadOnly)?;
        let store = tx.object_store("keyframe_chunks")?;
        let req = store.get(js_key)?;

        // Await the request
        let maybe = req.await?;
        
        // No need to explicitly commit a readonly transaction
        // It will automatically commit when all operations are complete

        // Deserialize if present, else return an error
        if let Some(js_val) = maybe {
            let chunk: KeyframeChunk = serde_wasm_bindgen::from_value(js_val).unwrap();
            //chunk.log_contents();
            Ok(chunk)
        } else {
            Err(Error::AddFailed(JsValue::from_str(
                &format!("No chunk found for key '{}'", key_str),
            )))
        }
    }
}