use serde::{Deserialize, Serialize};

use crate::artifacts::{IntentPacket, OpRecord};
use crate::session::Room;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload", rename_all = "snake_case")]
pub enum SessionEvent {
    FormalizeIntent(Box<IntentPacket>),
    DraftSpec,
    ApproveSpec,
    ProposeRealization,
    AcceptRealization,
    VerificationPassed,
    VerificationFailed,
    RepairSpecPreserving,
    RepairSpecChanging,
    ApproveTrust,
    CertificationPassed,
    IngestIncident,
    MoveRoom(Room),
    AppendOp(Box<OpRecord>),
}
