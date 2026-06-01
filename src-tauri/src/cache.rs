//! LRU 快取。
//!
//! 兩級快取：
//! - **解碼快取**（`usize` → 已解碼 `DynamicImage`）：免去重複解碼，
//!   切換縮放模式時只需重新縮放。
//! - **算繪快取**（`RenderKey` → 最終 PNG 位元組）：相同參數的頁面直接命中，
//!   配合背景預載，順序閱讀翻頁當下零運算。

use std::hash::Hash;
use std::num::NonZeroUsize;

use lru::LruCache;

/// 通用 LRU 包裝。
pub struct Lru<K: Hash + Eq, V: Clone> {
    inner: LruCache<K, V>,
}

impl<K: Hash + Eq, V: Clone> Lru<K, V> {
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            inner: LruCache::new(cap),
        }
    }

    /// 取出並更新近用順序。
    pub fn get(&mut self, key: &K) -> Option<V> {
        self.inner.get(key).cloned()
    }

    pub fn put(&mut self, key: K, value: V) {
        self.inner.put(key, value);
    }

    pub fn contains(&self, key: &K) -> bool {
        self.inner.contains(key)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

/// 算繪快取的鍵：唯一決定一張最終 PNG 的所有參數。
#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub struct RenderKey {
    pub index: usize,
    /// 縮放模式代碼（見 commands::mode_code）。
    pub mode: u8,
    pub viewport_w: u32,
    pub viewport_h: u32,
    /// 固定倍率 ×1000 後取整，使浮點可入鍵。
    pub scale_milli: u32,
}
