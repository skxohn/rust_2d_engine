use std::collections::HashMap;
use futures::stream::{FuturesUnordered, StreamExt};
use idb::{Database, DatabaseEvent, Error, Factory,  KeyPath, ObjectStoreParams, TransactionMode};
use std::rc::Rc;
use wasm_bindgen::JsValue;

use crate::keyframe::{Keyframe, KeyframeChunk};
pub struct KeyframeDatabase {
    db: Rc<Database>,
}

impl KeyframeDatabase {
    /// IndexedDB 연결 생성
    pub async fn new() -> Result<Rc<Self>, Error> {
        let factory = Factory::new()?;
        let db_name = "keyframe_db";
        let db_version = 1;
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
        let db = Rc::new(raw_db);
        Ok(Rc::new(Self { db }))
    }

    /// 청크를 나눠 병렬 저장 (Rc<Self> 필요)
    pub async fn save_keyframes_parallel(
        self: Rc<Self>,
        object_id: &str,
        keyframes: Vec<Keyframe>,
        chunk_size: f64,
    ) -> Result<(), Error> {
        let chunks = self.split_into_chunks(object_id, keyframes, chunk_size);
        let mut tasks = FuturesUnordered::new();

        for chunk in chunks {
            let db_clone = self.db.clone();
            let js_val: JsValue = serde_wasm_bindgen::to_value(&chunk).unwrap();
            tasks.push(async move {
                let tx = db_clone.transaction(&["keyframe_chunks"], TransactionMode::ReadWrite)?;
                let store = tx.object_store("keyframe_chunks")?;
                store.add(&js_val, None)?.await?;
                tx.commit()?.await?;
                Ok::<(), Error>(())
            });
        }

        while let Some(res) = tasks.next().await {
            res?;
        }
        Ok(())
    }

    /// 시간 기반으로 키프레임을 청크로 분할
    pub fn split_into_chunks(
        &self,
        object_id: &str,
        mut keyframes: Vec<Keyframe>,
        chunk_size: f64,
    ) -> Vec<KeyframeChunk> {
        if keyframes.is_empty() {
            return Vec::new();
        }
        keyframes.sort_by(|a, b| a.time().partial_cmp(&b.time()).unwrap());
        let mut buckets: HashMap<u64, Vec<Keyframe>> = HashMap::new();
        for kf in keyframes {
            let idx = (kf.time() / chunk_size).floor() as u64;
            buckets.entry(idx).or_default().push(kf);
        }
        let mut chunks: Vec<KeyframeChunk> = buckets.into_iter().map(|(idx, mut kfs)| {
            kfs.sort_by(|a, b| a.time().partial_cmp(&b.time()).unwrap());
            let mut chunk = KeyframeChunk::new(
                &format!("{}_{}", object_id, idx),
                idx as f64 * chunk_size,
                (idx as f64 + 1.0) * chunk_size,
            );
            for kf in kfs {
                chunk.add_keyframe(kf.time(), kf.x(), kf.y());
            }
            chunk
        }).collect();
        chunks.sort_by(|a, b| a.start_time().partial_cmp(&b.start_time()).unwrap());
        chunks
    }

    /// 특정 청크 로드
    pub async fn load_chunk(
        &self,
        object_id: &str,
        chunk_id: u32,
    ) -> Result<Option<KeyframeChunk>, Error> {
        let key_str = format!("{}_{}", object_id, chunk_id);
        let js_key = JsValue::from_str(&key_str);
        let tx = self.db.transaction(&["keyframe_chunks"], TransactionMode::ReadOnly)?;
        let store = tx.object_store("keyframe_chunks")?;
        let req = store.get(js_key)?;
        let maybe = req.await?;
        tx.commit()?.await?;
        if let Some(js_val) = maybe {
            let chunk: KeyframeChunk = serde_wasm_bindgen::from_value(js_val).unwrap();
            Ok(Some(chunk))
        } else {
            Ok(None)
        }
    }
}