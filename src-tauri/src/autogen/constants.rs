#![allow(non_camel_case_types)]

// Autogenerated file do not edit !!!!!

pub const STATE_CHANGE_EVENT: &str = "state_change_event";
pub const STATE_SYNC_EVENT: &str = "state_sync_event";
pub enum GLOBAL_APP_STATE_MACRO {
    TRACKS = 0,
    CHECK = 1,
}

impl From<u32> for GLOBAL_APP_STATE_MACRO {
    fn from(item: u32) -> Self {
        match item {
            0 => GLOBAL_APP_STATE_MACRO::TRACKS,
            1 => GLOBAL_APP_STATE_MACRO::CHECK,
            _ => panic!("Not a valid value for the enum GLOBAL_APP_STATE_MACRO"),
        }
    }
}

impl Into<u32> for GLOBAL_APP_STATE_MACRO {
    fn into(self) -> u32 {
        match self {
            GLOBAL_APP_STATE_MACRO::TRACKS => 0,
            GLOBAL_APP_STATE_MACRO::CHECK => 1,
        }
    }
}
