//! Friends system for Voxtera
//!
//! Provides friend request, accept, and list functionality.
//! Tracks online players and notifies friends when someone comes online.

use common::{
    comp::{ChatType, Content},
    uuid::Uuid,
};
use common_net::msg::ServerGeneral;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use specs::{Entity as EcsEntity, WorldExt};
use std::path::Path;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Status of a friend relationship from a player's perspective.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FriendStatus {
    /// We sent a request and are waiting for them to accept.
    PendingOutgoing,
    /// They sent us a request and we haven't accepted yet.
    PendingIncoming,
    /// An accepted friend.
    Accepted,
}

/// A single friend entry stored per player.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FriendEntry {
    pub uuid: Uuid,
    pub alias: String,
    pub status: FriendStatus,
}

/// Result of a friend list query.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FriendListResult {
    pub friends: Vec<FriendEntry>,
}

/// Result of sending a friend request.
#[derive(Clone, Debug)]
pub enum FriendRequestResult {
    /// Request sent successfully.
    Sent { to_alias: String },
    /// Target player not found (alias not registered).
    PlayerNotFound,
    /// Already friends with this player.
    AlreadyFriends { alias: String },
    /// A pending request already exists in this direction.
    AlreadyPending { alias: String },
    /// Cannot send a friend request to yourself.
    CannotFriendSelf,
}

/// Result of accepting a friend request.
#[derive(Clone, Debug)]
pub enum FriendAcceptResult {
    /// Friend request accepted.
    Accepted { alias: String },
    /// No pending incoming request from that player.
    NoPendingRequest,
    /// Player not found.
    PlayerNotFound,
}

/// Result of rejecting / removing a friend.
#[derive(Clone, Debug)]
pub enum FriendRemoveResult {
    Removed { alias: String },
    NotFriends,
    PlayerNotFound,
}

// ---------------------------------------------------------------------------
// ECS Resource
// ---------------------------------------------------------------------------

/// Central friends resource inserted into the ECS world.
///
/// Stores friend lists per-player UUID, tracks which players are currently
/// online, and maps ECS entities ↔ UUIDs so we can send notifications.
///
/// Insert into the ECS world during server init:
/// ```ignore
/// state.ecs_mut().insert(FriendsResource::default());
/// ```
pub struct FriendsResource {
    /// uuid → list of friend entries (includes both pending and accepted).
    friend_lists: HashMap<Uuid, Vec<FriendEntry>>,

    /// UUIDs of players currently online.
    online_players: HashSet<Uuid>,

    /// uuid → current alias (kept up to date on login).
    aliases: HashMap<Uuid, String>,

    /// uuid → ECS entity for online players (so we can send messages).
    uuid_to_entity: HashMap<Uuid, EcsEntity>,

    /// Pending notifications to deliver: (target_uuid, message).
    pending_notifications: Vec<(Uuid, String)>,

    /// Whether the serializable state changed since the last successful save.
    persistence_dirty: bool,
}

impl Default for FriendsResource {
    fn default() -> Self {
        Self {
            friend_lists: HashMap::new(),
            online_players: HashSet::new(),
            aliases: HashMap::new(),
            uuid_to_entity: HashMap::new(),
            pending_notifications: Vec::new(),
            persistence_dirty: false,
        }
    }
}

impl FriendsResource {
    // -- Online player tracking --------------------------------------------

    /// Call when a player logs in. Registers them as online and notifies
    /// accepted friends.
    pub fn player_online(&mut self, uuid: Uuid, alias: String, entity: EcsEntity) {
        let alias_changed = self.aliases.get(&uuid) != Some(&alias);
        self.aliases.insert(uuid, alias.clone());
        self.persistence_dirty |= alias_changed;
        self.uuid_to_entity.insert(uuid, entity);
        self.online_players.insert(uuid);

        info!(?uuid, ?alias, "Player marked online in friends system");

        // Notify accepted friends that this player came online.
        if let Some(friends) = self.friend_lists.get(&uuid) {
            let notify_uuids: Vec<Uuid> = friends
                .iter()
                .filter(|f| {
                    f.status == FriendStatus::Accepted && self.online_players.contains(&f.uuid)
                })
                .map(|f| f.uuid)
                .collect();

            for friend_uuid in notify_uuids {
                self.pending_notifications
                    .push((friend_uuid, format!("Seu amigo {} ficou online!", alias)));
            }

            // Also notify this player about friends already online.
            let online_friend_uuids: Vec<Uuid> = friends
                .iter()
                .filter(|f| {
                    f.status == FriendStatus::Accepted && self.online_players.contains(&f.uuid)
                })
                .map(|f| f.uuid)
                .collect();

            for friend_uuid in online_friend_uuids {
                if let Some(friend_alias) = self.aliases.get(&friend_uuid) {
                    self.pending_notifications
                        .push((uuid, format!("Seu amigo {} está online.", friend_alias)));
                }
            }
        }
    }

    /// Call when a player logs out.
    pub fn player_offline(&mut self, uuid: &Uuid) {
        self.online_players.remove(uuid);
        self.uuid_to_entity.remove(uuid);
        debug!(?uuid, "Player marked offline in friends system");
    }

    /// Returns true if the given UUID is currently online.
    pub fn is_online(&self, uuid: &Uuid) -> bool { self.online_players.contains(uuid) }

    // -- Friend operations -------------------------------------------------

    /// Send a friend request from `from` to `to`.
    ///
    /// If `to` already sent a pending request to `from`, this auto-accepts
    /// both sides.
    pub fn send_request(&mut self, from: Uuid, to: Uuid) -> FriendRequestResult {
        if from == to {
            return FriendRequestResult::CannotFriendSelf;
        }

        // Look up target alias.
        let to_alias = match self.aliases.get(&to) {
            Some(a) => a.clone(),
            None => return FriendRequestResult::PlayerNotFound,
        };
        let from_alias = self
            .aliases
            .get(&from)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());

        // Check if already accepted friends.
        if let Some(list) = self.friend_lists.get(&from) {
            if list
                .iter()
                .any(|f| f.uuid == to && f.status == FriendStatus::Accepted)
            {
                return FriendRequestResult::AlreadyFriends { alias: to_alias };
            }
        }

        // Check reverse pending (they already sent us a request → auto-accept).
        // This must come before the AlreadyPending check so that mutual requests
        // resolve automatically. We look for PendingOutgoing on `to`'s side because
        // that means `to` already sent a request to `from`.
        if let Some(list) = self.friend_lists.get(&to) {
            if list
                .iter()
                .any(|f| f.uuid == from && f.status == FriendStatus::PendingOutgoing)
            {
                // Auto-accept: promote both sides to Accepted.
                self.promote_to_accepted(from, to);
                self.persistence_dirty = true;
                return FriendRequestResult::Sent {
                    to_alias: format!("{} (auto-accepted)", to_alias),
                };
            }
        }

        // Check if we already have an outgoing pending request to this player.
        if let Some(list) = self.friend_lists.get(&from) {
            if list
                .iter()
                .any(|f| f.uuid == to && f.status == FriendStatus::PendingOutgoing)
            {
                return FriendRequestResult::AlreadyPending { alias: to_alias };
            }
        }

        // Add outgoing pending entry on sender's side.
        self.friend_lists
            .entry(from)
            .or_default()
            .push(FriendEntry {
                uuid: to,
                alias: to_alias.clone(),
                status: FriendStatus::PendingOutgoing,
            });

        // Add incoming pending entry on receiver's side so they can see the request.
        self.friend_lists.entry(to).or_default().push(FriendEntry {
            uuid: from,
            alias: from_alias.clone(),
            status: FriendStatus::PendingIncoming,
        });
        self.persistence_dirty = true;

        // Notify the target if online.
        if self.online_players.contains(&to) {
            self.pending_notifications
                .push((to, format!("{} sent you a friend request!", from_alias)));
        }

        FriendRequestResult::Sent { to_alias }
    }

    /// Accept a pending friend request from `requester` as seen by `acceptor`.
    pub fn accept_request(&mut self, acceptor: Uuid, requester: Uuid) -> FriendAcceptResult {
        let requester_alias = match self.aliases.get(&requester) {
            Some(a) => a.clone(),
            None => return FriendAcceptResult::PlayerNotFound,
        };

        // Verify there is a pending incoming request from requester → acceptor.
        let has_pending = self
            .friend_lists
            .get(&acceptor)
            .map(|list| {
                list.iter()
                    .any(|f| f.uuid == requester && f.status == FriendStatus::PendingIncoming)
            })
            .unwrap_or(false);

        if !has_pending {
            return FriendAcceptResult::NoPendingRequest;
        }

        self.promote_to_accepted(acceptor, requester);
        self.persistence_dirty = true;

        // Notify requester if online.
        let acceptor_alias = self
            .aliases
            .get(&acceptor)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());
        if self.online_players.contains(&requester) {
            self.pending_notifications.push((
                requester,
                format!("{} accepted your friend request!", acceptor_alias),
            ));
        }

        FriendAcceptResult::Accepted {
            alias: requester_alias,
        }
    }

    /// Remove a friend (or reject a pending request).
    pub fn remove_friend(&mut self, player: Uuid, target: Uuid) -> FriendRemoveResult {
        let target_alias = match self.aliases.get(&target) {
            Some(a) => a.clone(),
            None => return FriendRemoveResult::PlayerNotFound,
        };

        let removed = if let Some(list) = self.friend_lists.get_mut(&player) {
            let before = list.len();
            list.retain(|f| f.uuid != target);
            list.len() < before
        } else {
            false
        };

        // Also remove from the other side.
        let target_removed = if let Some(list) = self.friend_lists.get_mut(&target) {
            let before = list.len();
            list.retain(|f| f.uuid != player);
            list.len() < before
        } else {
            false
        };

        if removed || target_removed {
            self.persistence_dirty = true;
        }

        if removed {
            FriendRemoveResult::Removed {
                alias: target_alias,
            }
        } else {
            FriendRemoveResult::NotFriends
        }
    }

    /// Get the full friend list for a player (pending + accepted).
    pub fn get_friends(&self, uuid: &Uuid) -> FriendListResult {
        let friends = self.friend_lists.get(uuid).cloned().unwrap_or_default();
        FriendListResult { friends }
    }

    /// Get only accepted friends for a player.
    pub fn get_accepted_friends(&self, uuid: &Uuid) -> Vec<FriendEntry> {
        self.friend_lists
            .get(uuid)
            .map(|list| {
                list.iter()
                    .filter(|f| f.status == FriendStatus::Accepted)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get pending incoming friend requests for a player (requests they
    /// haven't accepted yet).
    pub fn get_pending_requests(&self, uuid: &Uuid) -> Vec<FriendEntry> {
        self.friend_lists
            .get(uuid)
            .map(|list| {
                list.iter()
                    .filter(|f| f.status == FriendStatus::PendingIncoming)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Build a stable, UI-ready snapshot sorted by relationship status and
    /// alias.
    pub fn ui_snapshot(&self, uuid: &Uuid) -> Vec<common_net::msg::FriendInfo> {
        let mut snapshot = self
            .friend_lists
            .get(uuid)
            .into_iter()
            .flatten()
            .map(|entry| common_net::msg::FriendInfo {
                alias: entry.alias.clone(),
                status: match entry.status {
                    FriendStatus::PendingOutgoing => common_net::msg::FriendStatus::PendingOutgoing,
                    FriendStatus::PendingIncoming => common_net::msg::FriendStatus::PendingIncoming,
                    FriendStatus::Accepted => common_net::msg::FriendStatus::Accepted,
                },
                online: self.online_players.contains(&entry.uuid),
            })
            .collect::<Vec<_>>();
        snapshot.sort_by_key(|entry| entry.alias.to_lowercase());
        snapshot
    }

    /// Look up a UUID by alias (case-insensitive). Returns the first match.
    pub fn uuid_from_alias(&self, alias: &str) -> Option<Uuid> {
        let lower = alias.to_lowercase();
        self.aliases
            .iter()
            .find(|(_, a)| a.to_lowercase() == lower)
            .map(|(uuid, _)| *uuid)
    }

    // -- Notification drain ------------------------------------------------

    /// Drain all pending notifications. Returns `(target_uuid, message)` pairs.
    pub fn drain_notifications(&mut self) -> Vec<(Uuid, String)> {
        std::mem::take(&mut self.pending_notifications)
    }

    /// Drain all pending notifications and resolve target UUIDs to ECS
    /// entities. Returns `(target_entity, message)` pairs ready for
    /// immediate delivery.
    pub fn drain_notifications_with_entities(&mut self) -> Vec<(EcsEntity, String)> {
        let notifications = std::mem::take(&mut self.pending_notifications);
        notifications
            .into_iter()
            .filter_map(|(uuid, msg)| self.uuid_to_entity.get(&uuid).map(|&entity| (entity, msg)))
            .collect()
    }

    // -- Internal helpers --------------------------------------------------

    /// Promote a pending relationship to Accepted on both sides.
    fn promote_to_accepted(&mut self, a: Uuid, b: Uuid) {
        // Promote on A's side.
        if let Some(list) = self.friend_lists.get_mut(&a) {
            for entry in list.iter_mut() {
                if entry.uuid == b {
                    entry.status = FriendStatus::Accepted;
                }
            }
        }
        // Ensure B has an Accepted entry for A (replace any pending).
        if let Some(list) = self.friend_lists.get_mut(&b) {
            list.retain(|f| f.uuid != a);
            let alias_a = self
                .aliases
                .get(&a)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());
            list.push(FriendEntry {
                uuid: a,
                alias: alias_a,
                status: FriendStatus::Accepted,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

/// Serializable subset of FriendsResource that gets saved to disk.
/// Runtime-only fields (online_players, uuid_to_entity, pending_notifications)
/// are rebuilt on load.
#[derive(Serialize, Deserialize, Default)]
struct SerializableFriends {
    friend_lists: HashMap<Uuid, Vec<FriendEntry>>,
    aliases: HashMap<Uuid, String>,
}

impl FriendsResource {
    /// Save friend lists and aliases to a .ron file atomically.
    ///
    /// Returns immediately when no serializable state changed since the last
    /// successful save. Failed writes leave the dirty flag set for retry.
    pub fn save_to_file(&mut self, path: &Path) {
        if !self.persistence_dirty {
            return;
        }
        let data = SerializableFriends {
            friend_lists: self.friend_lists.clone(),
            aliases: self.aliases.clone(),
        };
        let ron_str = match ron::ser::to_string_pretty(&data, ron::ser::PrettyConfig::default()) {
            Ok(s) => s,
            Err(e) => {
                warn!(?e, "Failed to serialize friends data");
                return;
            },
        };
        let atomic_file =
            atomicwrites::AtomicFile::new(path, atomicwrites::OverwriteBehavior::AllowOverwrite);
        if let Err(e) = atomic_file.write(|file| {
            use std::io::Write;
            file.write_all(ron_str.as_bytes())
        }) {
            warn!(?e, "Failed to save friends.ron");
        } else {
            self.persistence_dirty = false;
            debug!("Saved friends.ron ({} players)", data.friend_lists.len());
        }
    }

    /// Load friend lists and aliases from a .ron file.
    /// Returns a FriendsResource with runtime fields cleared.
    pub fn load_from_file(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }
        let contents = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                warn!(?e, "Failed to read friends.ron");
                return Self::default();
            },
        };
        match ron::from_str::<SerializableFriends>(&contents) {
            Ok(data) => {
                info!(players = data.friend_lists.len(), "Loaded friends.ron");
                Self {
                    friend_lists: data.friend_lists,
                    aliases: data.aliases,
                    online_players: HashSet::new(),
                    uuid_to_entity: HashMap::new(),
                    pending_notifications: Vec::new(),
                    persistence_dirty: false,
                }
            },
            Err(e) => {
                warn!(?e, "Failed to parse friends.ron, starting fresh");
                Self::default()
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Notification tick helper
// ---------------------------------------------------------------------------

/// Drain pending friend notifications and send them as in-game chat messages.
///
/// Call this once per server tick after `FriendsResource` has been updated.
/// Reads from the ECS world directly so it works with the existing server
/// tick structure.
pub fn tick_friends_notifications(ecs: &specs::World) {
    let notifications = {
        let mut friends = ecs.write_resource::<FriendsResource>();
        friends.drain_notifications()
    };

    if notifications.is_empty() {
        return;
    }

    // Build a snapshot of uuid → entity for delivery.
    let uuid_to_entity: HashMap<Uuid, EcsEntity> = {
        let friends = ecs.read_resource::<FriendsResource>();
        friends.uuid_to_entity.clone()
    };

    for (target_uuid, message) in notifications {
        if let Some(&entity) = uuid_to_entity.get(&target_uuid) {
            let clients = ecs.read_storage::<crate::Client>();
            if let Some(client) = clients.get(entity) {
                let msg = ServerGeneral::server_msg(ChatType::Meta, Content::Plain(message));
                client.send_fallible(msg);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_friend_request_and_accept() {
        let mut res = FriendsResource::default();
        let alice = Uuid::from_u128(1);
        let bob = Uuid::from_u128(2);

        res.aliases.insert(alice, "Alice".into());
        res.aliases.insert(bob, "Bob".into());

        // Alice sends request to Bob.
        let result = res.send_request(alice, bob);
        assert!(matches!(result, FriendRequestResult::Sent { .. }));

        // Alice should see PendingOutgoing, Bob should see PendingIncoming.
        assert_eq!(res.get_friends(&alice).friends.len(), 1);
        assert_eq!(res.get_friends(&bob).friends.len(), 1);
        assert_eq!(
            res.get_friends(&alice).friends[0].status,
            FriendStatus::PendingOutgoing
        );
        assert_eq!(
            res.get_friends(&bob).friends[0].status,
            FriendStatus::PendingIncoming
        );

        // Bob should see one pending incoming request.
        assert_eq!(res.get_pending_requests(&bob).len(), 1);
        // Alice should have no pending incoming requests.
        assert_eq!(res.get_pending_requests(&alice).len(), 0);

        // Bob accepts.
        let result = res.accept_request(bob, alice);
        assert!(matches!(result, FriendAcceptResult::Accepted { .. }));

        // Both should now be Accepted.
        assert_eq!(
            res.get_friends(&alice).friends[0].status,
            FriendStatus::Accepted
        );
        assert_eq!(
            res.get_friends(&bob).friends[0].status,
            FriendStatus::Accepted
        );
    }

    #[test]
    fn test_cannot_friend_self() {
        let mut res = FriendsResource::default();
        let alice = Uuid::from_u128(1);
        res.aliases.insert(alice, "Alice".into());

        let result = res.send_request(alice, alice);
        assert!(matches!(result, FriendRequestResult::CannotFriendSelf));
    }

    #[test]
    fn test_auto_accept_reverse_request() {
        let mut res = FriendsResource::default();
        let alice = Uuid::from_u128(1);
        let bob = Uuid::from_u128(2);
        res.aliases.insert(alice, "Alice".into());
        res.aliases.insert(bob, "Bob".into());

        // Alice → Bob, then Bob → Alice (should auto-accept).
        res.send_request(alice, bob);
        let result = res.send_request(bob, alice);
        assert!(matches!(result, FriendRequestResult::Sent { .. }));

        // Both should be accepted.
        assert_eq!(
            res.get_friends(&alice).friends[0].status,
            FriendStatus::Accepted
        );
        assert_eq!(
            res.get_friends(&bob).friends[0].status,
            FriendStatus::Accepted
        );
    }

    #[test]
    fn test_remove_friend() {
        let mut res = FriendsResource::default();
        let alice = Uuid::from_u128(1);
        let bob = Uuid::from_u128(2);
        res.aliases.insert(alice, "Alice".into());
        res.aliases.insert(bob, "Bob".into());

        res.send_request(alice, bob);
        res.accept_request(bob, alice);

        let result = res.remove_friend(alice, bob);
        assert!(matches!(result, FriendRemoveResult::Removed { .. }));
        assert!(res.get_friends(&alice).friends.is_empty());
        assert!(res.get_friends(&bob).friends.is_empty());
    }

    #[test]
    fn test_duplicate_request_rejected() {
        let mut res = FriendsResource::default();
        let alice = Uuid::from_u128(1);
        let bob = Uuid::from_u128(2);
        res.aliases.insert(alice, "Alice".into());
        res.aliases.insert(bob, "Bob".into());

        res.send_request(alice, bob);
        let result = res.send_request(alice, bob);
        assert!(matches!(result, FriendRequestResult::AlreadyPending { .. }));
    }

    #[test]
    fn test_alias_lookup() {
        let mut res = FriendsResource::default();
        let alice = Uuid::from_u128(1);
        res.aliases.insert(alice, "Alice".into());

        assert_eq!(res.uuid_from_alias("Alice"), Some(alice));
        assert_eq!(res.uuid_from_alias("alice"), Some(alice));
        assert_eq!(res.uuid_from_alias("Bob"), None);
    }

    #[test]
    fn test_pending_requests_are_incoming_only() {
        let mut res = FriendsResource::default();
        let alice = Uuid::from_u128(1);
        let bob = Uuid::from_u128(2);
        res.aliases.insert(alice, "Alice".into());
        res.aliases.insert(bob, "Bob".into());

        res.send_request(alice, bob);

        // Bob should see one pending incoming request.
        let pending = res.get_pending_requests(&bob);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].uuid, alice);
        assert_eq!(pending[0].status, FriendStatus::PendingIncoming);

        // Alice should NOT see a pending incoming request (she sent it).
        let pending_alice = res.get_pending_requests(&alice);
        assert!(pending_alice.is_empty());
    }

    #[test]
    fn test_ui_snapshot_contains_status_and_online_state() {
        let mut res = FriendsResource::default();
        let alice = Uuid::from_u128(1);
        let bob = Uuid::from_u128(2);
        let carol = Uuid::from_u128(3);
        res.aliases.insert(alice, "Alice".into());
        res.aliases.insert(bob, "Bob".into());
        res.aliases.insert(carol, "Carol".into());

        res.send_request(alice, bob);
        res.send_request(carol, alice);
        res.online_players.insert(bob);

        let snapshot = res.ui_snapshot(&alice);
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].alias, "Bob");
        assert_eq!(
            snapshot[0].status,
            common_net::msg::FriendStatus::PendingOutgoing
        );
        assert!(snapshot[0].online);
        assert_eq!(snapshot[1].alias, "Carol");
        assert_eq!(
            snapshot[1].status,
            common_net::msg::FriendStatus::PendingIncoming
        );
        assert!(!snapshot[1].online);
    }

    #[test]
    fn test_already_friends_rejected() {
        let mut res = FriendsResource::default();
        let alice = Uuid::from_u128(1);
        let bob = Uuid::from_u128(2);
        res.aliases.insert(alice, "Alice".into());
        res.aliases.insert(bob, "Bob".into());

        res.send_request(alice, bob);
        res.accept_request(bob, alice);

        let result = res.send_request(alice, bob);
        assert!(matches!(result, FriendRequestResult::AlreadyFriends { .. }));
    }

    #[test]
    fn friend_persistence_starts_clean() {
        let res = FriendsResource::default();
        assert!(!res.persistence_dirty);
    }

    #[test]
    fn friend_relationship_changes_mark_persistence_dirty() {
        let mut res = FriendsResource::default();
        let alice = Uuid::from_u128(1);
        let bob = Uuid::from_u128(2);
        res.aliases.insert(alice, "Alice".into());
        res.aliases.insert(bob, "Bob".into());

        res.send_request(alice, bob);

        assert!(res.persistence_dirty);
    }

    #[test]
    fn successful_friend_save_clears_persistence_dirty() {
        let mut res = FriendsResource::default();
        let alice = Uuid::from_u128(1);
        let bob = Uuid::from_u128(2);
        res.aliases.insert(alice, "Alice".into());
        res.aliases.insert(bob, "Bob".into());
        res.send_request(alice, bob);

        let path = std::env::temp_dir().join(format!("voxtera-friends-{alice}.ron"));
        res.save_to_file(&path);

        assert!(!res.persistence_dirty);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn failed_friend_save_keeps_persistence_dirty() {
        let mut res = FriendsResource::default();
        let alice = Uuid::from_u128(1);
        let bob = Uuid::from_u128(2);
        res.aliases.insert(alice, "Alice".into());
        res.aliases.insert(bob, "Bob".into());
        res.send_request(alice, bob);

        let parent = std::env::temp_dir().join(format!("voxtera-friends-parent-{alice}"));
        std::fs::write(&parent, b"not a directory").expect("create invalid save parent");
        res.save_to_file(&parent.join("friends.ron"));

        assert!(res.persistence_dirty);
        let _ = std::fs::remove_file(parent);
    }
}
