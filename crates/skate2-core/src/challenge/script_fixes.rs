//! Corrections to the original challenge scripts, made to a message handler's chunk before
//! `cMessageHandler::Attach` (`0x82ce38c8`) loads it.

use std::borrow::Cow;

/// S.K.A.T.E.'s `prepare_follower` `stateenter`, which every S.K.A.T.E. challenge shares.
pub(crate) const SKATE_PREPARE_FOLLOWER: u64 = 0x416F_B2B7_F3E1_08CD;

/// Its online search for the next connected follower, at the step past the end of
/// `FollowerOrder`: `CALL r9 2 2` (`table.getn(FollowerOrder)`), `LT 0 r9 r8`, `JMP 1`,
/// `SETTABLE r2 "FollowerIndex" 1.0`, `FORLOOP r4 -46`.
const FOLLOWER_SEARCH: [u32; 5] = [0x0100_825C, 0x0482_0018, 0x8000_0016, 0x90C2_C089, 0x7FF4_411F];
/// The `SETTABLE` of that search, which the correction replaces.
const FOLLOWER_SEARCH_RESTART: usize = 3;
/// `SETTABLE r2 "FollowerIndex" r9`: `FollowerIndex` stays at `table.getn(FollowerOrder)`.
const FOLLOWER_SEARCH_END: u32 = 0x9082_4089;

/// The handler's chunk with the corrections that apply to it.
///
/// `prepare_follower` online skips each disconnected follower, and past the end of
/// `FollowerOrder` the original starts again at the first one. With the last follower
/// disconnected before its turn, the followers before it take the same set trick again and
/// again: the round never reaches `process_round`, so no one else sets a trick. Corrected, the
/// search stops at the last follower, which ends the followers' turns as when every follower
/// left (`process_round`); the next round's `round_setup` leaves the disconnected player out.
pub(crate) fn corrected(handler: u64, chunk: &[u8]) -> Cow<'_, [u8]> {
    if handler != SKATE_PREPARE_FOLLOWER {
        return Cow::Borrowed(chunk);
    }
    let search: Vec<u8> = FOLLOWER_SEARCH.iter().flat_map(|word| word.to_le_bytes()).collect();
    let mut found = chunk.windows(search.len()).enumerate().filter(|(_, window)| *window == search.as_slice()).map(|(at, _)| at);
    let (Some(at), None) = (found.next(), found.next()) else {
        return Cow::Borrowed(chunk);
    };
    let mut fixed = chunk.to_vec();
    let restart = at + 4 * FOLLOWER_SEARCH_RESTART;
    fixed[restart..restart + 4].copy_from_slice(&FOLLOWER_SEARCH_END.to_le_bytes());
    Cow::Owned(fixed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk_with(words: &[u32]) -> Vec<u8> {
        let mut chunk = vec![0x1B, b'L', b'u', b'a', 0x51, 0, 1, 4, 4, 4, 4, 0];
        chunk.extend(words.iter().flat_map(|word| word.to_le_bytes()));
        chunk.extend([0xAA; 8]);
        chunk
    }

    #[test]
    fn the_follower_search_stops_at_the_last_follower() {
        let chunk = chunk_with(&FOLLOWER_SEARCH);
        let fixed = corrected(SKATE_PREPARE_FOLLOWER, &chunk);
        let mut expected = FOLLOWER_SEARCH;
        expected[FOLLOWER_SEARCH_RESTART] = FOLLOWER_SEARCH_END;
        assert_eq!(fixed.as_ref(), chunk_with(&expected).as_slice());
    }

    #[test]
    fn other_handlers_and_other_code_load_as_shipped() {
        let chunk = chunk_with(&FOLLOWER_SEARCH);
        assert!(matches!(corrected(0x1234, &chunk), Cow::Borrowed(_)));
        let mut changed = FOLLOWER_SEARCH;
        changed[0] ^= 1;
        assert!(matches!(corrected(SKATE_PREPARE_FOLLOWER, &chunk_with(&changed)), Cow::Borrowed(_)));
        let twice = [FOLLOWER_SEARCH, FOLLOWER_SEARCH].concat();
        assert!(matches!(corrected(SKATE_PREPARE_FOLLOWER, &chunk_with(&twice)), Cow::Borrowed(_)));
    }

    /// The shipped handler under stand-ins for its natives: an online round of four followers
    /// at the fourth's turn, the third having just tried, with the given followers disconnected.
    /// What `prepare_follower` does: the follower it moves to the attempt spawn, or the state
    /// it changes to, and `FollowerIndex` after it.
    fn prepare_follower(chunk: &[u8], disconnected: &[i32], index: i32, previous: i32) -> (Option<f32>, Option<String>, f32) {
        use super::super::lua::{Lua, Value};
        let mut lua = Lua::<()>::new();
        let entries: String = disconnected.iter().map(|player| format!("[{player}] = true, ")).collect();
        let stubs = format!(
            "disconnected = {{ {entries}}}
             Challenge = {{ GameMode = {{ PartyPlay = 1, SinglePlayer = 2, OnlineChallenge = 3 }} }}
             Skate = {{ ResetAttemptItem = function() end }}
             HUD2 = {{ ABDHUDRefreshCopyTrick = function() end }}
             ChallengeManager = {{ SetPlayer = function() end }}
             Online = {{ IsPlayerDisconnected = function(player) return disconnected[player] == true end }}
             AI = {{ RepositionAI = function(player) repositioned = player end, UnpinAI = function() end, PinAI = function() end }}
             function ChangeState(state) changed = state end
             function print() end
             data = {{ Name = 'skate', SpotVolumes = {{ Count = 0 }}, TurnBasedWaitingLocation = 0, TurnBasedAttemptSpawnPoint = 0 }}
             vars = {{ GameMode = 3, CachedTeleportID = -1, FollowerIndex = {index}, PreviousFollowerIndex = {previous}, FollowerOrder = {{ 1, 2, 3, 4 }}, SpotVolumeEntered = {{}} }}"
        );
        let setup = lua.load_source(stubs.as_bytes(), "stubs").expect("stubs compile");
        lua.call(&mut (), &setup, &[]).expect("stubs run");
        let main = lua.load(chunk, "prepare_follower").expect("handler loads");
        lua.call(&mut (), &main, &[]).expect("handler defines itself");
        let handler = lua.get_global(&format!("h_{SKATE_PREPARE_FOLLOWER:016X}"));
        let (data, vars) = (lua.get_global("data"), lua.get_global("vars"));
        let message = Value::Table(lua.create_table());
        lua.call(&mut (), &handler, &[data, message, vars.clone()]).expect("prepare_follower runs");
        let number = |value: Value| if let Value::Number(n) = value { Some(n) } else { None };
        let repositioned = number(lua.get_global("repositioned"));
        let changed = match lua.get_global("changed") {
            Value::Nil => None,
            state => Some(lua.display(&state)),
        };
        let Value::Table(vars) = vars else { unreachable!("vars is a table") };
        let index = number(lua.table(vars).get_str("FollowerIndex")).expect("FollowerIndex");
        (repositioned, changed, index)
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn a_disconnected_last_follower_ends_the_followers_turns() {
        use super::super::attrib_db::{AttribDb, CAREER_VAULTS};
        use super::super::keys::class;
        use super::super::runtime::field;
        use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
        let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
        let archive = SKATE2_ARCHIVES.iter().find(|archive| archive.relative_path == "data\\db\\db.big").expect("db.big");
        let mut mounts = MountSet::new();
        mounts.push(MountedBig::open(archive.path_under(std::path::Path::new(&root)), archive.use_external_header).expect("db.big opens"));
        let db = AttribDb::load(&mounts, &CAREER_VAULTS).expect("vaults load");
        let shipped = db.attribute(class::CHALLENGE_MESSAGE_HANDLERS, SKATE_PREPARE_FOLLOWER, field::HANDLER_BLOB).and_then(|blob| blob.blob()).expect("prepare_follower chunk");
        let fixed = corrected(SKATE_PREPARE_FOLLOWER, shipped);
        assert!(matches!(fixed, Cow::Owned(_)), "the shipped chunk holds the follower search once");

        assert_eq!(prepare_follower(shipped, &[4], 4, 3), (Some(1.0), None, 1.0), "shipped: the first follower takes the set trick again");
        assert_eq!(prepare_follower(&fixed, &[4], 4, 3), (None, Some("process_round".into()), 4.0));
        assert_eq!(prepare_follower(&fixed, &[3, 4], 3, 2), (None, Some("process_round".into()), 4.0));
        assert_eq!(prepare_follower(&fixed, &[2], 2, 1), (Some(3.0), None, 3.0), "a disconnected follower before the last is skipped as shipped");
        assert_eq!(prepare_follower(&fixed, &[], 4, 3), (Some(4.0), None, 4.0));
    }
}
