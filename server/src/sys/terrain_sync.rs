#[cfg(not(feature = "worldgen"))]
use crate::test_world::World;
use crate::{Settings, chunk_serialize::ChunkSendEntry, client::Client};
use common::{
    comp::{Pos, Presence},
    event::EventBus,
    terrain::CoordinateConversions,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{CompressedData, ServerGeneral};
use common_state::TerrainChanges;
use rayon::prelude::*;
use specs::{Entities, Read, ReadExpect, ReadStorage};
use std::sync::Arc;
use vek::Vec2;
#[cfg(feature = "worldgen")] use world::World;

/// Returns `true` if any block in `modified_block_chunks` falls within the
/// view distance of the player at `player_chunk_pos` with squared VD
/// `player_vd_sqr`.
fn blocks_in_player_vd(
    player_chunk_pos: Vec2<i16>,
    player_vd_sqr: i32,
    modified_block_chunks: &[Vec2<i32>],
) -> bool {
    modified_block_chunks
        .iter()
        .any(|&chunk| super::terrain::chunk_in_vd(player_chunk_pos, player_vd_sqr, chunk))
}

fn has_syncable_terrain_changes(terrain_changes: &TerrainChanges) -> bool {
    !terrain_changes.modified_chunks.is_empty() || !terrain_changes.modified_blocks.is_empty()
}

/// This systems sends modified chunks (existing chunks that had a new chunk
/// generated) to clients as well as block modifications in existing chunks.
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, Arc<World>>,
        Read<'a, Settings>,
        Read<'a, TerrainChanges>,
        ReadExpect<'a, EventBus<ChunkSendEntry>>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Presence>,
        ReadStorage<'a, Client>,
    );

    const NAME: &'static str = "terrain_sync";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            world,
            server_settings,
            terrain_changes,
            chunk_send_bus,
            positions,
            presences,
            clients,
        ): Self::SystemData,
    ) {
        if !has_syncable_terrain_changes(&terrain_changes) {
            return;
        }

        let max_view_distance = server_settings.max_view_distance.unwrap_or(u32::MAX);
        #[cfg(feature = "worldgen")]
        let world_size = world.sim().get_size();
        #[cfg(not(feature = "worldgen"))]
        let world_size = world.map_size_lg().chunks().as_();
        let (presences_position_entities, _) = super::terrain::prepare_player_presences(
            world_size,
            max_view_distance,
            &entities,
            &positions,
            &presences,
            &clients,
        );
        let real_max_view_distance =
            super::terrain::convert_to_loaded_vd(u32::MAX, max_view_distance);

        // Sync changed chunks
        terrain_changes.modified_chunks.par_iter().for_each_init(
            || chunk_send_bus.emitter(),
            |chunk_send_emitter, &chunk_key| {
                // We only have to check players inside the maximum view distance of the server
                // of our own position.
                //
                // We start by partitioning by X, finding only entities in chunks within the X
                // range of us.  These are guaranteed in bounds due to restrictions on max view
                // distance (namely: the square of any chunk coordinate plus the max view
                // distance along both axes must fit in an i32).
                let min_chunk_x = chunk_key.x - real_max_view_distance;
                let max_chunk_x = chunk_key.x + real_max_view_distance;
                let start = presences_position_entities
                    .partition_point(|((pos, _), _)| i32::from(pos.x) < min_chunk_x);
                // NOTE: We *could* just scan forward until we hit the end, but this way we save
                // a comparison in the inner loop, since also needs to check the
                // list length.  We could also save some time by starting from
                // start rather than end, but the hope is that this way the
                // compiler (and machine) can reorder things so both ends are
                // fetched in parallel; since the vast majority of the time both fetched
                // elements should already be in cache, this should not use any
                // extra memory bandwidth.
                //
                // TODO: Benchmark and figure out whether this is better in practice than just
                // scanning forward.
                let end = presences_position_entities
                    .partition_point(|((pos, _), _)| i32::from(pos.x) < max_chunk_x);
                let interior = &presences_position_entities[start..end];
                interior
                    .iter()
                    .filter(|((player_chunk_pos, player_vd_sqr), _)| {
                        super::terrain::chunk_in_vd(*player_chunk_pos, *player_vd_sqr, chunk_key)
                    })
                    .for_each(|(_, entity)| {
                        chunk_send_emitter.emit(ChunkSendEntry {
                            entity: *entity,
                            chunk_key,
                        });
                    });
            },
        );

        // TODO: Don't send all changed blocks to all clients
        // Sync changed blocks
        if !terrain_changes.modified_blocks.is_empty() {
            // Collect the unique chunk keys that contain modified blocks so we can
            // skip clients whose view distance does not overlap any of them.
            let modified_block_chunks: Vec<Vec2<i32>> = terrain_changes
                .modified_blocks
                .keys()
                .map(|wpos| wpos.xy().wpos_to_cpos())
                .collect();

            let mut lazy_msg = None;
            for ((player_chunk_pos, player_vd_sqr), entity) in &presences_position_entities {
                let client = clients.get(*entity);
                if client.is_none() {
                    continue;
                }
                let client = client.unwrap();

                let player_in_range =
                    blocks_in_player_vd(*player_chunk_pos, *player_vd_sqr, &modified_block_chunks);

                if !player_in_range {
                    continue;
                }

                if lazy_msg.is_none() {
                    lazy_msg = Some(client.prepare(ServerGeneral::TerrainBlockUpdates(
                        CompressedData::compress(&terrain_changes.modified_blocks, 1),
                    )));
                }
                lazy_msg.as_ref().map(|msg| client.send_prepared(msg));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{blocks_in_player_vd, has_syncable_terrain_changes};
    use common::terrain::Block;
    use common_state::TerrainChanges;
    use vek::{Vec2, Vec3};

    #[test]
    fn terrain_sync_skips_when_no_chunks_or_blocks_changed() {
        let mut changes = TerrainChanges::default();
        assert!(!has_syncable_terrain_changes(&changes));

        changes.modified_chunks.insert(Vec2::zero());
        assert!(has_syncable_terrain_changes(&changes));

        changes.modified_chunks.clear();
        changes.modified_blocks.insert(Vec3::zero(), Block::empty());
        assert!(has_syncable_terrain_changes(&changes));
    }

    #[test]
    fn blocks_in_vd_returns_true_when_chunk_overlaps() {
        let player_chunk = Vec2::new(10, 10);
        let vd_sqr = 9; // VD radius of 3 chunks
        let modified_chunks = vec![Vec2::new(12, 10)]; // distance 2, within VD
        assert!(blocks_in_player_vd(player_chunk, vd_sqr, &modified_chunks));
    }

    #[test]
    fn blocks_in_vd_returns_false_when_chunk_out_of_range() {
        let player_chunk = Vec2::new(10, 10);
        let vd_sqr = 9; // VD radius of 3 chunks
        let modified_chunks = vec![Vec2::new(20, 10)]; // distance 10, outside VD
        assert!(!blocks_in_player_vd(player_chunk, vd_sqr, &modified_chunks));
    }

    #[test]
    fn blocks_in_vd_returns_false_for_empty_modified_blocks() {
        let player_chunk = Vec2::new(10, 10);
        let vd_sqr = 100;
        let modified_chunks: Vec<Vec2<i32>> = vec![];
        assert!(!blocks_in_player_vd(player_chunk, vd_sqr, &modified_chunks));
    }

    #[test]
    fn blocks_in_vd_returns_true_if_any_chunk_in_range() {
        let player_chunk = Vec2::new(0, 0);
        let vd_sqr = 4; // VD radius of 2 chunks
        let modified_chunks = vec![Vec2::new(100, 100), Vec2::new(1, 1)]; // far + near
        assert!(blocks_in_player_vd(player_chunk, vd_sqr, &modified_chunks));
    }
}
