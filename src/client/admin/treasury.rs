// ----- standard library imports
// ----- extra library imports
use bitcoin::Amount;
use thiserror::Error;
// ----- local imports
use crate::{
    cashu,
    core::BillId,
    wire::{exchange as wire_exchange, treasury as wire_treasury},
};

// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("mint operation not found {0}")]
    MintOpNotFound(uuid::Uuid),

    #[error("internal error {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("sign error {0}")]
    NUT20(#[from] cashu::nut20::Error),
}

#[derive(Debug, Clone)]
pub struct Client {
    cl: reqwest::Client,
    base: reqwest::Url,
}

impl Client {
    pub fn new(base: reqwest::Url) -> Self {
        Self {
            cl: reqwest::Client::new(),
            base,
        }
    }

    pub const REQTOPAY_EP_V1: &'static str = "/v1/admin/treasury/request_to_pay_ebill";
    pub async fn request_to_pay_ebill(
        &self,
        ebill_id: BillId,
        amount: Amount,
        deadline: chrono::DateTime<chrono::Utc>,
    ) -> Result<wire_treasury::RequestToPayFromEBillResponse> {
        let request = wire_treasury::RequestToPayFromEBillRequest {
            ebill_id,
            amount,
            deadline,
        };
        let url = self
            .base
            .join(Self::REQTOPAY_EP_V1)
            .expect("request_to_pay_ebill relative path");
        let request = self.cl.post(url).json(&request);
        let response: wire_treasury::RequestToPayFromEBillResponse =
            request.send().await?.json().await?;
        Ok(response)
    }

    pub const TRYHTLC_EP_V1: &'static str = "/v1/admin/treasury/try_htlc_swap";
    pub async fn try_htlc(&self, preimage: String) -> Result<cashu::Amount> {
        let url = self
            .base
            .join(Self::TRYHTLC_EP_V1)
            .expect("try_htlc relative path");
        let msg = wire_exchange::HtlcSwapAttemptRequest { preimage };
        let request = self.cl.post(url).json(&msg);
        let response = request.send().await?.json().await?;
        Ok(response)
    }

    pub const NEWEBILLMINTOP_EP_V1: &'static str = "/v1/admin/treasury/ebill/mintop";
    pub async fn new_ebill_mint_operation(
        &self,
        qid: uuid::Uuid,
        kid: cashu::Id,
        pk: cashu::PublicKey,
        target: cashu::Amount,
        bill_id: BillId,
    ) -> Result<()> {
        let url = self
            .base
            .join(Self::NEWEBILLMINTOP_EP_V1)
            .expect("ebill mint operation relative path");
        let msg = wire_treasury::NewMintOperationRequest {
            quote_id: qid,
            kid,
            pub_key: pk,
            target,
            bill_id,
        };
        let request = self.cl.post(url).json(&msg);
        let _ = request
            .send()
            .await?
            .json::<wire_treasury::NewMintOperationResponse>()
            .await?;
        Ok(())
    }

    pub const EBILLMINTOPSTATUS_EP_V1: &'static str = "/v1/admin/treasury/ebill/mintop/{qid}";
    pub async fn ebill_mint_operation_status(
        &self,
        qid: uuid::Uuid,
    ) -> Result<wire_treasury::MintOperationStatus> {
        let url = self
            .base
            .join(&Self::EBILLMINTOPSTATUS_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("ebill mint operation status relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::MintOpNotFound(qid));
        }
        let response = response
            .json::<wire_treasury::MintOperationStatus>()
            .await?;
        Ok(response)
    }

    pub const LISTEBILLMINTOPS_EP_V1: &'static str = "/v1/admin/treasury/ebill/mintops/{kid}";
    pub async fn list_ebill_mint_operations(&self, kid: cashu::Id) -> Result<Vec<uuid::Uuid>> {
        let url = self
            .base
            .join(&Self::LISTEBILLMINTOPS_EP_V1.replace("{kid}", &kid.to_string()))
            .expect("list ebill mint operations relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        let response = response.json::<Vec<uuid::Uuid>>().await?;
        Ok(response)
    }
}
