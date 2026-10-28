//! `Sk8::BE::NISBackEnd` (`be_nis.cpp`): the pro table the NIS overlays
//! read.

/// One `mPros` entry. `GetProName` (`0x8255cb68`), `GetHometown`
/// (`0x8255cbd0`) and `GetIsGoofy` (`0x8255cc38`) index the table by
/// `eProChallengePros`, which `Init` fills in that order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pro {
    /// `mName`, a string label.
    pub name: &'static str,
    /// `mHometown`, a string label.
    pub hometown: &'static str,
    pub goofy: bool,
    /// `mProfileName`, quotes included.
    pub profile: &'static str,
}

const fn pro(name: &'static str, hometown: &'static str, goofy: bool, profile: &'static str) -> Pro {
    Pro { name, hometown, goofy, profile }
}

/// `NISBackEnd::Init` (`0x8255c558`).
pub const PROS: [Pro; 20] = [
    pro("ID_CHEAT_PROSKATER14", "ID_CHEAT_PRODETAIL_LOCATION6", false, "\"pj_ladd\""),
    pro("ID_CHEAT_PROSKATER17", "ID_CHEAT_PRODETAIL_LOCATION11", true, "\"paul_rodriguez\""),
    pro("ID_CHEAT_PROSKATER07", "ID_CHEAT_PRODETAIL_LOCATION6", true, "\"pat_duffy\""),
    pro("ID_CHEAT_PROSKATER05", "ID_CHEAT_PRODETAIL_LOCATION4", false, "\"chris_cole\""),
    pro("ID_CHEAT_PROSKATER06", "ID_CHEAT_PRODETAIL_LOCATION5", false, "\"jason_dill\""),
    pro("ID_CHEAT_PROSKATER03", "ID_CHEAT_PRODETAIL_LOCATION2", true, "\"mike_carroll\""),
    pro("ID_CHEAT_PROSKATER10", "ID_CHEAT_PRODETAIL_LOCATION5", true, "\"mark_gonzales\""),
    pro("ID_CHEAT_PROSKATER19", "ID_CHEAT_PRODETAIL_LOCATION13", false, "\"danny_way\""),
    pro("ID_CHEAT_PROSKATER11", "ID_CHEAT_PRODETAIL_LOCATION3", true, "\"chris_haslam\""),
    pro("ID_CHEAT_PROSKATER04", "ID_CHEAT_PRODETAIL_LOCATION3", true, "\"alex_chalmers\""),
    pro("ID_CHEAT_PROSKATER09", "ID_CHEAT_PRODETAIL_LOCATION7", true, "\"ryan_gallant\""),
    pro("ID_CHEAT_PROSKATER15", "ID_CHEAT_PRODETAIL_LOCATION3", true, "\"colin_mackay\""),
    pro("ID_CHEAT_PROSKATER18", "ID_CHEAT_PRODETAIL_LOCATION12", false, "\"ryan_smith\""),
    pro("ID_CHEAT_PROSKATER13", "ID_CHEAT_PRODETAIL_LOCATION9", false, "\"terry_kennedy\""),
    pro("ID_CHEAT_PROSKATER12", "ID_CHEAT_PRODETAIL_LOCATION8", true, "\"jerry_hsu\""),
    pro("ID_CHEAT_PROSKATER08", "ID_CHEAT_PRODETAIL_LOCATION6", true, "\"rob_dyrdek\""),
    pro("ID_CHEAT_PROSKATER20", "ID_CHEAT_PRODETAIL_LOCATION6", true, "\"chris_boykin\""),
    pro("ID_CHEAT_PROSKATER16", "ID_CHEAT_PRODETAIL_LOCATION10", false, "\"john_rattray\""),
    pro("ID_CHEAT_PROSKATER01", "ID_CHEAT_PRODETAIL_LOCATION1", true, "\"ali_boulala\""),
    pro("ID_CHEAT_PROSKATER02", "ID_CHEAT_PRODETAIL_LOCATION2", true, "\"dennis_busenitz\""),
];
