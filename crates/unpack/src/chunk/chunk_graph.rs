use index_vec::IndexVec;
use rustc_hash::FxHashMap;

use super::{chunk_group::ChunkGroup, chunk_group_id::ChunkGroupId, chunk_id::ChunkId, Chunk};

#[derive(Debug,Default)]
pub struct ChunkGraph {
    named_chunks: FxHashMap<String, ChunkId>,
    named_chunk_groups: FxHashMap<String, ChunkGroupId>,
    chunks: IndexVec<ChunkId, Chunk>,
    chunk_groups: IndexVec<ChunkGroupId, ChunkGroup>
}

impl ChunkGraph {
    pub fn create_chunk(&mut self,name:Option<String>) -> ChunkId{
        let chunk = Chunk::new(name.clone());
        let chunk_id = self.add_chunk(chunk);
        if let Some(name) = name {
            self.named_chunks.insert(name, chunk_id);
        }
        chunk_id
        
    }
    pub fn create_chunk_group(&mut self, entry_chunk_id: ChunkId,name: Option<String>)-> ChunkGroupId{
        let mut chunk_group = ChunkGroup::new();
        chunk_group.connect_chunk(entry_chunk_id);
        let chunk_group_id = self.add_chunk_group(chunk_group);
        if let Some(name) = name {
            self.named_chunk_groups.insert(name, chunk_group_id);
        }
        chunk_group_id
    }
    pub fn add_chunk(&mut self, chunk:Chunk)-> ChunkId {
        self.chunks.push(chunk)
    }
    pub fn add_chunk_group(&mut self, chunk_group: ChunkGroup) -> ChunkGroupId {
        self.chunk_groups.push(chunk_group)
    }
}