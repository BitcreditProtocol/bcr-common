// ----- standard library imports
use std::time::Duration;
// ----- extra library imports
use async_nats::{Client, ConnectOptions, ServerAddr};
use bytes::Bytes;
// ----- project imports
use super::Url;
use super::error::Result;
use super::model::MintStream;
use crate::wire::clowder::messages;
// ----- end imports

#[derive(Clone)]
pub struct ClowderNatsClient {
    client: Client,
}

impl ClowderNatsClient {
    pub async fn new(nats_url: Url) -> Result<Self> {
        let nats_addr = ServerAddr::from_url(nats_url)?;

        let options = ConnectOptions::default().request_timeout(Some(Duration::from_secs(30)));
        let client = async_nats::connect_with_options(nats_addr, options).await?;

        Ok(Self { client })
    }

    // Mints
    pub const ONCHAIN_TOPIC: &'static str = "clowder.mint_onchain";
    pub const EIOU_TOPIC: &'static str = "clowder.mint_eiou";
    pub const EBILL_TOPIC: &'static str = "clowder.mint_ebill";
    pub const FOREIGN_ECASH_TOPIC: &'static str = "clowder.mint_foreign_ecash";
    pub const MINT_QUOTE_ONCHAIN_TOPIC: &'static str = "clowder.mint_quote_onchain";
    pub const FOREIGN_OFFLINE_ECASH_TOPIC: &'static str = "clowder.mint_foreign_offline_ecash";
    // Swaps
    pub const SWAP_TOPIC: &'static str = "clowder.swap";
    pub const SWAP_COMMITMENT_TOPIC: &'static str = "clowder.swap_commitment";
    // Keysets
    pub const KEYSET_TOPIC: &'static str = "clowder.keyset";
    // Melts
    pub const MELT_ONCHAIN_TOPIC: &'static str = "clowder.melt_onchain";
    pub const MELT_QUOTE_ONCHAIN_TOPIC: &'static str = "clowder.melt_quote_onchain";
    // Misc
    pub const DEACTIVATE_KEYSET_TOPIC: &'static str = "clowder.deactivate_keyset";
    pub const HEARTBEAT_TOPIC: &'static str = "clowder.heartbeat";
    pub const BILLREQUESTTOPAY_TOPIC: &'static str = "clowder.billrequesttopay";
    pub const OFFLINE_EXCHANGE_SIGN_TOPIC: &'static str = "clowder.offline_exchange_sign";

    pub async fn swap_commitment(
        &self,
        req: messages::SwapCommitmentRequest,
    ) -> Result<messages::SwapCommitmentResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(&MintStream::SwapCommitment(req), &mut payload)?;

        let response = self
            .client
            .request(Self::SWAP_COMMITMENT_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::SwapCommitmentResponse =
            ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }

    pub async fn melt_quote_onchain(
        &self,
        req: messages::MeltQuoteOnchainRequest,
    ) -> Result<messages::MeltQuoteOnchainResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(&MintStream::MeltQuoteOnchain(req), &mut payload)?;

        let response = self
            .client
            .request(Self::MELT_QUOTE_ONCHAIN_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::MeltQuoteOnchainResponse =
            ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }

    pub async fn mint_quote_onchain(
        &self,
        req: messages::MintQuoteOnchainRequest,
    ) -> Result<messages::MintQuoteOnchainResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(&MintStream::MintQuoteOnchain(req), &mut payload)?;

        let response = self
            .client
            .request(Self::MINT_QUOTE_ONCHAIN_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::MintQuoteOnchainResponse =
            ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }

    pub async fn sign_offline_exchange(
        &self,
        req: messages::OfflineExchangeSignRequest,
    ) -> Result<messages::OfflineExchangeSignResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(&MintStream::OfflineExchangeSign(req), &mut payload)?;

        let response = self
            .client
            .request(Self::OFFLINE_EXCHANGE_SIGN_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::OfflineExchangeSignResponse =
            ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }

    pub async fn mint_swap(
        &self,
        req: messages::SwapRequest,
        resp: messages::SwapResponse,
    ) -> Result<messages::SwapResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(&MintStream::Swap(req, resp), &mut payload)?;

        let response = self
            .client
            .request(Self::SWAP_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::SwapResponse = ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }

    pub async fn mint_onchain(
        &self,
        req: messages::MintOnchainRequest,
        resp: messages::MintOnchainResponse,
    ) -> Result<messages::MintOnchainResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(&MintStream::MintOnchain(req, resp), &mut payload)?;

        let response = self
            .client
            .request(Self::ONCHAIN_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::MintOnchainResponse =
            ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }

    pub async fn mint_bill(
        &self,
        req: messages::MintEbillRequest,
        resp: messages::MintEbillResponse,
    ) -> Result<messages::MintEbillResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(&MintStream::MintEbill(req, resp), &mut payload)?;

        let response = self
            .client
            .request(Self::EBILL_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::MintEbillResponse = ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }

    pub async fn request_to_pay_bill(
        &self,
        req: messages::RequestToPayEbillRequest,
        resp: messages::RequestToPayEbillResponse,
    ) -> Result<messages::RequestToPayEbillResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(&MintStream::BillRequestToPay(req, resp), &mut payload)?;

        let response = self
            .client
            .request(Self::BILLREQUESTTOPAY_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::RequestToPayEbillResponse =
            ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }

    pub async fn mint_foreign_ecash(
        &self,
        req: messages::MintForeignEcashRequest,
        resp: messages::MintForeignEcashResponse,
    ) -> Result<messages::MintForeignEcashResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(&MintStream::MintForeignEcash(req, resp), &mut payload)?;

        let response = self
            .client
            .request(Self::FOREIGN_ECASH_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::MintForeignEcashResponse =
            ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }

    pub async fn mint_offline_foreign_ecash(
        &self,
        req: messages::MintForeignOfflineEcashRequest,
        resp: messages::MintForeignOfflineEcashResponse,
    ) -> Result<messages::MintForeignOfflineEcashResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(
            &MintStream::MintForeignOfflineEcash(req, resp),
            &mut payload,
        )?;

        let response = self
            .client
            .request(Self::FOREIGN_OFFLINE_ECASH_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::MintForeignOfflineEcashResponse =
            ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }

    pub async fn mint_eiou(
        &self,
        req: messages::MintEiouRequest,
        resp: messages::MintEiouResponse,
    ) -> Result<messages::MintEiouResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(&MintStream::MintEiou(req, resp), &mut payload)?;

        let response = self
            .client
            .request(Self::EIOU_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::MintEiouResponse = ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }

    pub async fn new_keyset(
        &self,
        req: messages::KeysetCreationRequest,
        resp: messages::KeysetCreationResponse,
    ) -> Result<messages::KeysetCreationResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(&MintStream::CreateKeyset(req, resp), &mut payload)?;

        let response = self
            .client
            .request(Self::KEYSET_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::KeysetCreationResponse =
            ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }

    pub async fn melt_onchain(
        &self,
        req: messages::MeltOnchainRequest,
    ) -> Result<messages::MeltOnchainResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(&MintStream::MeltOnchain(req), &mut payload)?;

        let response = self
            .client
            .request(Self::MELT_ONCHAIN_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::MeltOnchainResponse =
            ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }

    pub async fn deactivate_keyset(
        &self,
        req: messages::KeysetDeactivationRequest,
    ) -> Result<messages::KeysetDeactivationResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(&MintStream::DeactivateKeyset(req), &mut payload)?;

        let response = self
            .client
            .request(Self::DEACTIVATE_KEYSET_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::KeysetDeactivationResponse =
            ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }

    pub async fn heartbeat(
        &self,
        req: messages::HeartbeatRequest,
        resp: messages::HeartbeatResponse,
    ) -> Result<messages::HeartbeatResponse> {
        let mut payload = Vec::new();
        ciborium::into_writer(&MintStream::Heartbeat(req, resp), &mut payload)?;

        let response = self
            .client
            .request(Self::HEARTBEAT_TOPIC, Bytes::from(payload))
            .await?;

        let result: messages::HeartbeatResponse = ciborium::from_reader(response.payload.as_ref())?;
        Ok(result)
    }
}
