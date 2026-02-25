// ----- standard library imports
// ----- extra library imports
use thiserror::Error;
// ----- local imports
use crate::{
    core::BillId,
    wire::{bill as wire_bill, identity as wire_identity, quotes as wire_quotes},
};

// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Error)]
pub enum Error {
    #[error("resource not found {0}")]
    ResourceNotFound(String),
    #[error("invalid request")]
    InvalidRequest,
    #[error("invalid content type")]
    InvalidContentType,
    #[error("invalid bill id")]
    InvalidBillId,

    #[error("internal error {0}")]
    Reqwest(#[from] reqwest::Error),
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

    pub const VALIDATE_AND_DECRYPT_SHARED_BILL_EP_V1: &'static str =
        "/v1/admin/bill/validate_and_decrypt_shared_bill";
    pub async fn validate_and_decrypt_shared_bill(
        &self,
        shared_bill: &wire_quotes::SharedBill,
    ) -> Result<wire_quotes::BillInfo> {
        let url = self
            .base
            .join(Self::VALIDATE_AND_DECRYPT_SHARED_BILL_EP_V1)
            .expect("validate and decrypt shared bill relative path");
        let request = self.cl.post(url).json(shared_bill);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        let bill_info = response.json::<wire_quotes::BillInfo>().await?;
        Ok(bill_info)
    }

    pub const BACKUP_SEED_PHRASE_EP_V1: &'static str = "/v1/admin/identity/seed/backup";
    pub async fn backup_seed_phrase(&self) -> Result<wire_identity::SeedPhrase> {
        let url = self
            .base
            .join(Self::BACKUP_SEED_PHRASE_EP_V1)
            .expect("backup seed phrase relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        let seed_phrase = response.json::<wire_identity::SeedPhrase>().await?;
        Ok(seed_phrase)
    }

    pub const RESTORE_FROM_SEED_PHRASE_EP_V1: &'static str = "/v1/admin/identity/seed/recover";
    pub async fn restore_from_seed_phrase(
        &self,
        seed_phrase: &wire_identity::SeedPhrase,
    ) -> Result<()> {
        let url = self
            .base
            .join(Self::RESTORE_FROM_SEED_PHRASE_EP_V1)
            .expect("restore seed phrase relative path");
        let request = self.cl.put(url).json(seed_phrase);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        response.error_for_status()?;
        Ok(())
    }

    pub const GET_IDENTITY_EP_V1: &'static str = "/v1/admin/identity/detail";
    pub async fn get_identity(&self) -> Result<wire_identity::Identity> {
        let url = self
            .base
            .join(Self::GET_IDENTITY_EP_V1)
            .expect("identity detail relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound("identity".into()));
        }
        let identity = response.json::<wire_identity::Identity>().await?;
        Ok(identity)
    }

    pub const CREATE_IDENTITY_EP_V1: &'static str = "/v1/admin/identity/create";
    pub async fn create_identity(&self, payload: &wire_identity::NewIdentityPayload) -> Result<()> {
        let url = self
            .base
            .join(Self::CREATE_IDENTITY_EP_V1)
            .expect("create identity relative path");
        let request = self.cl.post(url).json(payload);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        response.error_for_status()?;
        Ok(())
    }

    pub const GET_BILLS_EP_V1: &'static str = "/v1/admin/bill/list";
    pub async fn get_bills(&self) -> Result<Vec<wire_bill::BitcreditBill>> {
        let url = self
            .base
            .join(Self::GET_BILLS_EP_V1)
            .expect("bill list relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound("bills".into()));
        }
        let bills = response
            .json::<wire_bill::BillsResponse<wire_bill::BitcreditBill>>()
            .await?;
        Ok(bills.bills)
    }

    pub const GET_BILL_EP_V1: &'static str = "/v1/admin/bill/detail/{bill_id}";
    pub async fn get_bill(&self, bill_id: &BillId) -> Result<wire_bill::BitcreditBill> {
        let url = self
            .base
            .join(&Self::GET_BILL_EP_V1.replace("{bill_id}", &bill_id.to_string()))
            .expect("bill detail relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(bill_id.to_string()));
        }
        let bill = response.json::<wire_bill::BitcreditBill>().await?;
        Ok(bill)
    }

    pub const GET_BILL_ENDORSEMENTS_EP_V1: &'static str = "/v1/admin/bill/endorsements/{bill_id}";
    pub async fn get_bill_endorsements(
        &self,
        bill_id: &BillId,
    ) -> Result<Vec<wire_bill::Endorsement>> {
        let url = self
            .base
            .join(&Self::GET_BILL_ENDORSEMENTS_EP_V1.replace("{bill_id}", &bill_id.to_string()))
            .expect("bill endorsements relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(bill_id.to_string()));
        }
        let endorsements = response.json::<Vec<wire_bill::Endorsement>>().await?;
        Ok(endorsements)
    }

    pub const GET_BILL_ATTACHMENT_EP_V1: &'static str =
        "/v1/admin/bill/attachment/{bill_id}/{file_name}";
    /// Returns the content type and the bytes of the file
    pub async fn get_bill_attachment(
        &self,
        bill_id: &BillId,
        file_name: &str,
    ) -> Result<(String, Vec<u8>)> {
        let url = self
            .base
            .join(
                &Self::GET_BILL_ATTACHMENT_EP_V1
                    .replace("{bill_id}", &bill_id.to_string())
                    .replace("{file_name}", file_name),
            )
            .expect("bill attachment relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(format!("{bill_id} - {file_name}",)));
        }
        let content_type: String = match response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .map(|h| h.to_str())
        {
            Some(Ok(content_type)) => content_type.to_owned(),
            _ => return Err(Error::InvalidContentType),
        };
        let bytes = response.bytes().await?;
        Ok((content_type, bytes.to_vec()))
    }

    pub const GET_BITCOIN_PRIVATE_DESCRIPTOR_FOR_BILL_EP_V1: &'static str =
        "/v1/admin/bill/bitcoin_key/{bill_id}";
    pub async fn get_bitcoin_private_descriptor_for_bill(
        &self,
        bill_id: &BillId,
    ) -> Result<wire_bill::BillCombinedBitcoinKey> {
        let url = self
            .base
            .join(
                &Self::GET_BITCOIN_PRIVATE_DESCRIPTOR_FOR_BILL_EP_V1
                    .replace("{bill_id}", &bill_id.to_string()),
            )
            .expect("bill bitcoin key relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(bill_id.to_string()));
        }
        let btc_key = response.json::<wire_bill::BillCombinedBitcoinKey>().await?;
        Ok(btc_key)
    }

    pub const REQUEST_TO_PAY_BILL_EP_V1: &'static str = "/v1/admin/bill/request_to_pay";
    pub async fn request_to_pay_bill(
        &self,
        payload: &wire_bill::RequestToPayBitcreditBillPayload,
    ) -> Result<()> {
        let url = self
            .base
            .join(Self::REQUEST_TO_PAY_BILL_EP_V1)
            .expect("req to pay bill relative path");
        let request = self.cl.put(url).json(payload);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(payload.bill_id.to_string()));
        }
        response.error_for_status()?;
        Ok(())
    }

    pub const GET_BILL_PAYMENT_STATUS_EP_V1: &'static str =
        "/v1/admin/bill/payment_status/{bill_id}";
    pub async fn get_payment_status(
        &self,
        bill_id: BillId,
    ) -> Result<wire_bill::SimplifiedBillPaymentStatus> {
        let url = self
            .base
            .join(&Self::GET_BILL_PAYMENT_STATUS_EP_V1.replace("{bill_id}", &bill_id.to_string()))
            .expect("bill payment status relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(bill_id.to_string()));
        }
        let status = response
            .json::<wire_bill::SimplifiedBillPaymentStatus>()
            .await?;
        Ok(status)
    }
}
