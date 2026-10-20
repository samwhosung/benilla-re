//! The raid roster (`systems/ui/raid.md`): the rebuild from the group list, `0x4ba5f0`, and the
//! count of names it waits for, which the name answer `0x4bad90` and the own-row fill `0x4ba140`
//! keep.
//!
//! The roster is 40 records of 0x160 bytes behind the pointers at `0xb712a8`, the member count
//! `0xb713e0`, the pending-name count `0xb713f0` and the ready check deadline `0xb713f4`. Every
//! call out of these functions goes through [`RaidHost`] in the client's order.

/// The records behind `0xb712a8`.
pub const ROWS: usize = 40;
/// The member block at `+0x10`, of the party record kind (`objectlayer/party`).
pub const BLOCK_LEN: usize = 0x148;
/// The status byte inside the block (record `+0x18`); bit 0 is online.
pub const BLOCK_STATUS: usize = 0x08;

/// Tutorial 45, fired by a list with members (`0x4ba60d`).
pub const TUTORIAL_RAID: u32 = 45;
/// `ERR_RAID_YOU_JOINED`, `ERR_RAID_MEMBER_ADDED_S` and `ERR_RAID_MEMBER_REMOVED_S`.
pub const MSG_YOU_JOINED: u32 = 0x194;
pub const MSG_MEMBER_ADDED: u32 = 0x196;
pub const MSG_MEMBER_REMOVED: u32 = 0x197;
/// `RAID_ROSTER_UPDATE` and `PARTY_MEMBERS_CHANGED`.
pub const EVENT_RAID_ROSTER_UPDATE: u32 = 0x1f3;
pub const EVENT_PARTY_MEMBERS_CHANGED: u32 = 0xda;
/// `CMSG_REQUEST_PARTY_MEMBER_STATS` and `MSG_RAID_TARGET_UPDATE`.
pub const CMSG_REQUEST_PARTY_MEMBER_STATS: u32 = 0x27f;
pub const MSG_RAID_TARGET_UPDATE: u32 = 0x321;
/// The unit token flags of a raid member and of its pet.
pub const TOKEN_RAID: u32 = 0x100;
pub const TOKEN_RAID_PET: u32 = 0x200;
/// The `ClntObjMgrObjectPtr` type masks: a player, a unit.
pub const MASK_PLAYER: u32 = 0x10;
pub const MASK_UNIT: u32 = 0x8;

/// One roster record, `0x160` bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// `+0x00`.
    pub guid: u64,
    /// `+0x08`, the flag dword AND 0x7f.
    pub subgroup: u32,
    /// `+0x0c`: 2 the leader, 1 assistant, 0.
    pub rank: u32,
    /// `+0x10`.
    pub block: [u8; BLOCK_LEN],
    /// `+0x158`, the ready check word.
    pub ready: u32,
    /// `+0x15c`, which nothing here writes.
    pub tail: u32,
}

/// The roster's state in the client's globals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Roster {
    /// The 40 records.
    pub rows: Vec<Row>,
    /// `0xb713e0`.
    pub count: u32,
    /// `0xb713f0`, the names asked for and not yet answered.
    pub pending: u32,
    /// `0xb713f4`, 0 when no ready check runs.
    pub deadline: u32,
    /// `0xbc75f8`, the group leader cell, read only.
    pub leader: u64,
}

/// The completion callback a name request carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameCallback {
    /// `0x4bacd0`, the joined notice.
    MemberAdded,
    /// `0x4bad30`, the left notice.
    MemberRemoved,
    /// `0x4ba1b0`, the pending-name answer, which runs [`on_roster_name`].
    RosterName,
}

/// The owner pointer a name request carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameOwner {
    /// The GUID's entry in the group list's array.
    ListEntry(usize),
    /// A roster record.
    Row(usize),
}

/// The calls out of the roster code. A request's data and dedupe arguments are always 0.
pub trait RaidHost {
    /// `FireTutorial(id, 0)` (`0x4b5390`).
    fn fire_tutorial(&mut self, id: u32);
    /// `DBCache::NameCache::CancelCallback(guid, callback, 0)` (`0x55f870`).
    fn cancel_name(&mut self, guid: u64, callback: NameCallback);
    /// `DBCache::NameCache::GetRecord(guid, owner, callback, 0, 0)` (`0x55f080`): whether the
    /// name is cached; a miss keeps the callback.
    fn name_record(&mut self, guid: u64, owner: NameOwner, callback: NameCallback) -> bool;
    /// `CGGameUI::DisplayError(id, record)` (`0x496720`), with the name record of `name_of`.
    fn display_error(&mut self, id: u32, name_of: Option<u64>);
    /// `ClntObjMgrGetActivePlayer()` (`0x468550`).
    fn active_player(&mut self) -> u64;
    /// `ClntObjMgrObjectPtr(mask, guid)` (`0x468460`): the object's `UNIT_FIELD_CHARM` and
    /// `UNIT_FIELD_SUMMON`, or `None`.
    fn object(&mut self, mask: u32, guid: u64) -> Option<[u64; 2]>;
    /// `CGUnit_C::RemoveUnitTokenFlags(flags)` (`0x60c4f0`) on the object `guid` names.
    fn remove_token_flags(&mut self, guid: u64, flags: u32);
    /// `CGUnit_C::AddUnitTokenFlags(flags)` (`0x60c4a0`) on the object `guid` names.
    fn add_token_flags(&mut self, guid: u64, flags: u32);
    /// `CGGameUI::ReleaseTarget(sendSelection, guid)` (`0x493910`).
    fn release_target(&mut self, send_selection: u32, guid: u64);
    /// `ClientServices::Send` (`0x5ab630`) of a message: the opcode dword, then the body.
    fn send(&mut self, message: &[u8]);
    /// `FrameScript_SignalEvent(event)` (`0x703e50`).
    fn signal_event(&mut self, event: u32);
    /// `OsGetAsyncTimeMs()` (`0x42c010`).
    fn now_ms(&mut self) -> u32;
    /// `ShowReadyCheckSummary()` (`0x4bb310`).
    fn ready_check_summary(&mut self);
}

/// A member's pet as the client picks it: `UNIT_FIELD_CHARM`, else `UNIT_FIELD_SUMMON`.
#[must_use]
pub fn pet_guid(fields: [u64; 2]) -> u64 {
    if fields[0] != 0 {
        fields[0]
    } else {
        fields[1]
    }
}

/// `0x4ba5f0(guids, flags, online, count)`: replaces the roster with the group list's raid
/// members, in list order. A GUID already on the roster keeps its block, a new one gets a zeroed
/// block whose u16 health, maximum, power and maximum power are 1. Rows at and past the new count
/// are left as they are.
pub fn rebuild(
    roster: &mut Roster,
    guids: &[u64],
    flags: &[u32],
    online: &[u8],
    host: &mut impl RaidHost,
) {
    let count = guids.len();
    if count > 0 {
        host.fire_tutorial(TUTORIAL_RAID);
    }
    let old = roster.count as usize;
    let empty_on_entry = old == 0;
    if roster.pending != 0 {
        for i in 0..old {
            host.cancel_name(roster.rows[i].guid, NameCallback::RosterName);
        }
    }
    // The new blocks, the old one by GUID or a fresh one (`0x4ba6b6` to `0x4ba784`).
    let mut blocks = vec![[0u8; BLOCK_LEN]; count];
    let mut joined = Vec::new();
    for (j, &guid) in guids.iter().enumerate() {
        if let Some(i) = (0..old).find(|&i| roster.rows[i].guid == guid) {
            blocks[j] = roster.rows[i].block;
            continue;
        }
        for at in [0x0a, 0x0c, 0x0e, 0x10] {
            blocks[j][at..at + 2].copy_from_slice(&1u16.to_le_bytes());
        }
        if old != 0 && host.name_record(guid, NameOwner::ListEntry(j), NameCallback::MemberAdded) {
            host.display_error(MSG_MEMBER_ADDED, Some(guid));
        }
        joined.push(guid);
    }
    // The members gone (`0x4ba7a0` to `0x4ba830`).
    let mut gone = Vec::new();
    for i in 0..old {
        let guid = roster.rows[i].guid;
        if guids.contains(&guid) {
            continue;
        }
        if host.name_record(guid, NameOwner::Row(i), NameCallback::MemberRemoved)
            && host.active_player() != 0
        {
            host.display_error(MSG_MEMBER_REMOVED, Some(guid));
        }
        gone.push(guid);
    }
    if old == 0 {
        host.display_error(MSG_YOU_JOINED, None);
    }
    let saved: Vec<(u64, u32)> = roster.rows[..old]
        .iter()
        .map(|r| (r.guid, r.ready))
        .collect();
    roster.pending = 0;
    roster.count = count as u32;
    // The rows (`0x4ba8c3` to `0x4ba9ea`).
    for j in 0..count {
        let leader = roster.leader;
        let row = &mut roster.rows[j];
        row.guid = guids[j];
        row.subgroup = flags[j] & 0x7f;
        row.rank = if row.guid == leader {
            2
        } else if flags[j] as u8 & 0x80 != 0 {
            1
        } else {
            0
        };
        row.block = blocks[j];
        if online[j] != 0 {
            row.block[BLOCK_STATUS] |= 1;
        } else {
            row.block[BLOCK_STATUS] &= !1;
        }
        row.ready = saved.iter().find(|s| s.0 == row.guid).map_or(0, |s| s.1);
        let guid = row.guid;
        if guid != 0 && !host.name_record(guid, NameOwner::Row(j), NameCallback::RosterName) {
            roster.pending += 1;
        }
    }
    // Token flags of the members gone (`0x4baa00` to `0x4baae4`).
    for &guid in &gone {
        match host.object(MASK_PLAYER, guid) {
            Some(fields) => {
                let pet = pet_guid(fields);
                if pet != 0 {
                    if host.object(MASK_UNIT, pet).is_some() {
                        host.remove_token_flags(pet, TOKEN_RAID_PET);
                    } else {
                        host.release_target(1, pet);
                    }
                }
                host.remove_token_flags(guid, TOKEN_RAID);
            }
            None => host.release_target(1, guid),
        }
    }
    // Token flags of the new members, or a stats request (`0x4bab00` to `0x4babd9`).
    for &guid in &joined {
        match host.object(MASK_PLAYER, guid) {
            Some(fields) => {
                host.add_token_flags(guid, TOKEN_RAID);
                let pet = pet_guid(fields);
                if host.object(MASK_UNIT, pet).is_some() {
                    host.add_token_flags(pet, TOKEN_RAID_PET);
                }
            }
            None => {
                let mut message = CMSG_REQUEST_PARTY_MEMBER_STATS.to_le_bytes().to_vec();
                message.extend_from_slice(&guid.to_le_bytes());
                host.send(&message);
            }
        }
    }
    if roster.pending == 0 {
        host.signal_event(EVENT_RAID_ROSTER_UPDATE);
    }
    if empty_on_entry {
        let mut message = MSG_RAID_TARGET_UPDATE.to_le_bytes().to_vec();
        message.push(0xff);
        host.send(&message);
    }
    // The ready check ends when a member left and no online member is unanswered (`0x4bac55`).
    let active = host.active_player();
    if roster.deadline != 0 && !gone.is_empty() && active != 0 && active == roster.leader {
        let open = roster.rows[..count]
            .iter()
            .any(|r| r.ready != 0 && r.block[BLOCK_STATUS] & 1 != 0);
        if !open {
            roster.deadline = host.now_ms().wrapping_sub(1);
            host.ready_check_summary();
        }
    }
}

/// `0x4bad90`, which the name answer `0x4ba1b0` runs: lowers the pending count and, once it is 0,
/// or when it already was, signals `RAID_ROSTER_UPDATE` and then `PARTY_MEMBERS_CHANGED`.
pub fn on_roster_name(roster: &mut Roster, host: &mut impl RaidHost) {
    if roster.pending != 0 {
        roster.pending -= 1;
        if roster.pending != 0 {
            return;
        }
    }
    host.signal_event(EVENT_RAID_ROSTER_UPDATE);
    host.signal_event(EVENT_PARTY_MEMBERS_CHANGED);
}

/// `0x4ba140`, run on entering the world: the first row below the count whose GUID is 0 takes the
/// active player's GUID and asks for its name; a cached name signals `RAID_ROSTER_UPDATE`, a miss
/// raises the pending count.
pub fn fill_own_row(roster: &mut Roster, host: &mut impl RaidHost) {
    let count = roster.count as usize;
    let Some(i) = roster.rows[..count].iter().position(|r| r.guid == 0) else {
        return;
    };
    let guid = host.active_player();
    roster.rows[i].guid = guid;
    if host.name_record(guid, NameOwner::Row(i), NameCallback::RosterName) {
        host.signal_event(EVENT_RAID_ROSTER_UPDATE);
    } else {
        roster.pending += 1;
    }
}
