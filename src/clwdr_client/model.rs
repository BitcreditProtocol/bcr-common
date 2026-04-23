// ----- standard library imports
// ----- extra library imports
use serde::{Deserialize, Serialize};
// ----- local imports
use crate::wire::clowder::messages;

// ----- end imports

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MintStream {
    Swap(messages::SwapRequest, messages::SwapResponse),
    MintOnchain(messages::MintOnchainRequest, messages::MintOnchainResponse),
    MintEiou(messages::MintEiouRequest, messages::MintEiouResponse),
    MintEbill(messages::MintEbillRequest, messages::MintEbillResponse),
    MintForeignEcash(
        messages::MintForeignEcashRequest,
        messages::MintForeignEcashResponse,
    ),
    MintForeignOfflineEcash(
        messages::MintForeignOfflineEcashRequest,
        messages::MintForeignOfflineEcashResponse,
    ),
    MeltOnchain(messages::MeltOnchainRequest),
    MeltQuoteOnchain(messages::MeltQuoteOnchainRequest),
    MintQuoteOnchain(messages::MintQuoteOnchainRequest),
    OfflineExchangeSign(messages::OfflineExchangeSignRequest),
    SwapCommitment(messages::SwapCommitmentRequest),
    CreateKeyset(
        messages::KeysetCreationRequest,
        messages::KeysetCreationResponse,
    ),
    BillRequestToPay(
        messages::RequestToPayEbillRequest,
        messages::RequestToPayEbillResponse,
    ),
    Heartbeat(messages::HeartbeatRequest, messages::HeartbeatResponse),
    DeactivateKeyset(messages::KeysetDeactivationRequest),
}
