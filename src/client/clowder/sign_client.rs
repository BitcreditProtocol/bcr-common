// ----- standard library imports
// ----- extra library imports
use bitcoin::{
    hashes::{Hash, sha256::Hash as Sha256},
    secp256k1::{PublicKey, schnorr::Signature},
};
use bytes::Bytes;
// ----- local imports
use crate::client::clowder::{
    error::{ClowderClientError, Result},
    model::{
        CommitmentsResponse, FrostRequest, FrostResponse, KeysetConfigRequest,
        KeysetConfigResponse, PublicKeyPackageRequest, PublicKeyPackageResponse, SignedSwapRequest,
    },
};
// ----- end imports

pub struct SignatoryNatsClient {
    client: async_nats::Client,
}

impl SignatoryNatsClient {
    pub const SCHNORR_TOPIC: &'static str = "signatory.schnorr";
    pub const ECDSA_TOPIC: &'static str = "signatory.ecdsa";
    pub const PUBKEY_TOPIC: &'static str = "signatory.pubkey";
    pub const VRF_TOPIC: &'static str = "signatory.vrf";
    pub const COORDS_MAC_TOPIC: &'static str = "signatory.coords_mac";
    // Generates nonces and commitments according to frost
    pub const COMMITMENT_TOPIC: &'static str = "signatory.commitment";
    pub const FROST_TOPIC: &'static str = "signatory.frost";
    pub const PUBKEY_PACKAGE_TOPIC: &'static str = "signatory.pubkey_package";
    pub const KEYSET_CONFIG_TOPIC: &'static str = "signatory.keyset_config";
    pub const HASH_SIZE: usize = 32;
    pub const PUBLIC_KEY_SIZE: usize = 33;
    pub const SCHNORR_SIGNATURE_SIZE: usize = 64;
    pub const ECDSA_SIGNATURE_SIZE: usize = 64;
    pub const VRF_PROOF_SIZE: usize = 81;
    pub const VRF_HASH_SIZE: usize = 32;
    pub const VRF_TOTAL_SIZE: usize = Self::VRF_PROOF_SIZE + Self::VRF_HASH_SIZE;

    /// New Signatory client with a five second default timeout
    pub async fn new(
        nats_url: reqwest::Url,
        timeout_override: Option<std::time::Duration>,
    ) -> Result<Self> {
        let timeout = timeout_override.unwrap_or(std::time::Duration::from_secs(5));
        let client = async_nats::connect_with_options(
            nats_url.to_string(),
            async_nats::ConnectOptions::new().request_timeout(Some(timeout)),
        )
        .await?;

        Ok(Self { client })
    }

    pub async fn sign_swap_request(
        &self,
        inputs: &[cashu::Proof],
        outputs: &[cashu::BlindedMessage],
        commitment: bitcoin::secp256k1::schnorr::Signature,
    ) -> Result<SignedSwapRequest> {
        let hash = SignedSwapRequest::msg_to_sign(inputs, outputs);

        let hash_bytes = hash.to_byte_array();

        let response = self
            .client
            .request(Self::SCHNORR_TOPIC, Bytes::from(hash_bytes.to_vec()))
            .await?;

        let signature_bytes: [u8; Self::SCHNORR_SIGNATURE_SIZE] = response
            .payload
            .as_ref()
            .try_into()
            .map_err(|_| ClowderClientError::InvalidSignature)?;

        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|_| ClowderClientError::InvalidSignature)?;

        let pubkey = self.public_key().await?;

        Ok(SignedSwapRequest {
            inputs: inputs.to_vec(),
            outputs: outputs.to_vec(),
            commitment,
            pubkey,
            signature,
        })
    }

    pub async fn sign_schnorr_preimage(&self, preimage: &[u8]) -> Result<Signature> {
        let hash_bytes = Sha256::hash(preimage).to_byte_array();

        let response = self
            .client
            .request(Self::SCHNORR_TOPIC, Bytes::from(hash_bytes.to_vec()))
            .await?;

        let signature_bytes: [u8; Self::SCHNORR_SIGNATURE_SIZE] = response
            .payload
            .as_ref()
            .try_into()
            .map_err(|_| ClowderClientError::InvalidSignature)?;

        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|_| ClowderClientError::InvalidSignature)?;

        Ok(signature)
    }

    pub async fn sign_schnorr_hash(&self, hash: &[u8; 32]) -> Result<Signature> {
        let response = self
            .client
            .request(Self::SCHNORR_TOPIC, Bytes::from(hash.to_vec()))
            .await?;

        let signature_bytes: [u8; Self::SCHNORR_SIGNATURE_SIZE] = response
            .payload
            .as_ref()
            .try_into()
            .map_err(|_| ClowderClientError::InvalidSignature)?;

        Signature::from_slice(&signature_bytes).map_err(|_| ClowderClientError::InvalidSignature)
    }

    pub async fn sign_ecdsa_message(
        &self,
        msg: &bitcoin::secp256k1::Message,
    ) -> Result<bitcoin::secp256k1::ecdsa::Signature> {
        let msg_bytes: &[u8; Self::HASH_SIZE] = msg.as_ref();
        let response = self
            .client
            .request(Self::ECDSA_TOPIC, Bytes::from(msg_bytes.to_vec()))
            .await?;

        let signature_bytes: [u8; Self::ECDSA_SIGNATURE_SIZE] = response
            .payload
            .as_ref()
            .try_into()
            .map_err(|_| ClowderClientError::InvalidSignature)?;

        let signature = bitcoin::secp256k1::ecdsa::Signature::from_compact(&signature_bytes)
            .map_err(|_| ClowderClientError::InvalidSignature)?;

        Ok(signature)
    }

    pub async fn compute_coords_mac(&self, payload: &[u8]) -> Result<[u8; 32]> {
        let response = self
            .client
            .request(Self::COORDS_MAC_TOPIC, Bytes::from(payload.to_vec()))
            .await?;
        response
            .payload
            .as_ref()
            .try_into()
            .map_err(|_| ClowderClientError::InvalidSignature)
    }

    pub async fn sign_vrf_hash(&self, hash: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        let response = self
            .client
            .request(Self::VRF_TOPIC, Bytes::from(hash.to_vec()))
            .await?;

        let vrf_proof_hash: [u8; Self::VRF_TOTAL_SIZE] = response
            .payload
            .as_ref()
            .try_into()
            .map_err(|_| ClowderClientError::InvalidSignature)?;

        // split into 81,32
        let proof = &vrf_proof_hash[..Self::VRF_PROOF_SIZE];
        let hash = &vrf_proof_hash[Self::VRF_PROOF_SIZE..];

        Ok((proof.to_vec(), hash.to_vec()))
    }

    pub async fn public_key(&self) -> Result<PublicKey> {
        let response = self.client.request(Self::PUBKEY_TOPIC, "".into()).await?;

        let pubkey_bytes: [u8; Self::PUBLIC_KEY_SIZE] = response
            .payload
            .as_ref()
            .try_into()
            .map_err(|_| ClowderClientError::InvalidPublicKey)?;

        let public_key = PublicKey::from_slice(&pubkey_bytes)
            .map_err(|_| ClowderClientError::InvalidPublicKey)?;

        Ok(public_key)
    }

    pub async fn generate_commitments(
        &self,
        aggregated_key: &bitcoin::secp256k1::PublicKey,
        count: usize,
    ) -> Result<CommitmentsResponse> {
        let request = super::model::CommitmentsRequest {
            aggregated_key: *aggregated_key,
            count,
        };

        let mut payload = Vec::new();
        ciborium::into_writer(&request, &mut payload)?;

        let response = self
            .client
            .request(Self::COMMITMENT_TOPIC, payload.into())
            .await?;

        let commitments_response: CommitmentsResponse =
            ciborium::from_reader(response.payload.as_ref())?;

        Ok(commitments_response)
    }

    pub async fn sign_frost_request(&self, request: &FrostRequest) -> Result<FrostResponse> {
        let mut serialized = Vec::new();
        ciborium::into_writer(request, &mut serialized)?;

        let response = self
            .client
            .request(Self::FROST_TOPIC, Bytes::from(serialized))
            .await?;

        let frost_response: FrostResponse = ciborium::from_reader(response.payload.as_ref())?;

        Ok(frost_response)
    }

    pub async fn get_pubkey_package(
        &self,
        request: &PublicKeyPackageRequest,
    ) -> Result<PublicKeyPackageResponse> {
        let mut serialized = Vec::new();
        ciborium::into_writer(request, &mut serialized)?;

        let response = self
            .client
            .request(Self::PUBKEY_PACKAGE_TOPIC, Bytes::from(serialized))
            .await?;

        let package_response: PublicKeyPackageResponse =
            ciborium::from_reader(response.payload.as_ref())?;

        Ok(package_response)
    }

    pub async fn get_keyset_config(
        &self,
        request: &KeysetConfigRequest,
    ) -> Result<KeysetConfigResponse> {
        let mut serialized = Vec::new();
        ciborium::into_writer(request, &mut serialized)?;

        let response = self
            .client
            .request(Self::KEYSET_CONFIG_TOPIC, Bytes::from(serialized))
            .await?;

        let config_response: KeysetConfigResponse =
            ciborium::from_reader(response.payload.as_ref())?;

        Ok(config_response)
    }
}
