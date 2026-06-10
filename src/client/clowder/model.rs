// ----- standard library imports
// ----- extra library imports
use serde::{Deserialize, Serialize};
// ----- local imports
use crate::wire::clowder as wire_clowder;

// ----- end imports

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MintStream {
    Swap(wire_clowder::SwapRequest, wire_clowder::SwapResponse),
    MintOnchain(
        wire_clowder::MintOnchainRequest,
        wire_clowder::MintOnchainResponse,
    ),
    MintEiou(
        wire_clowder::MintEiouRequest,
        wire_clowder::MintEiouResponse,
    ),
    MintEbill(
        wire_clowder::MintEbillRequest,
        wire_clowder::MintEbillResponse,
    ),
    MintForeignEcash(
        wire_clowder::MintForeignEcashRequest,
        wire_clowder::MintForeignEcashResponse,
    ),
    MintForeignOfflineEcash(
        wire_clowder::MintForeignOfflineEcashRequest,
        wire_clowder::MintForeignOfflineEcashResponse,
    ),
    MeltOnchain(wire_clowder::MeltOnchainRequest),
    MeltQuoteOnchain(wire_clowder::MeltQuoteOnchainRequest),
    MintQuoteOnchain(wire_clowder::MintQuoteOnchainRequest),
    OfflineExchangeSign(wire_clowder::OfflineExchangeSignRequest),
    SwapCommitment(wire_clowder::SwapCommitmentRequest),
    CreateKeyset(
        wire_clowder::KeysetCreationRequest,
        wire_clowder::KeysetCreationResponse,
    ),
    BillRequestToPay(
        wire_clowder::RequestToPayEbillRequest,
        wire_clowder::RequestToPayEbillResponse,
    ),
    Heartbeat(
        wire_clowder::HeartbeatRequest,
        wire_clowder::HeartbeatResponse,
    ),
    DeactivateKeyset(wire_clowder::KeysetDeactivationRequest),
}
