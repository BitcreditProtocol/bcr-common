// ----- standard library imports
// ----- extra library imports
// ----- local imports

// ----- end imports

pub trait MintConnectorExt: cdk::wallet::MintConnector + Send + Sync {}
impl MintConnectorExt for cdk::HttpClient {}

#[cfg(all(feature = "test-utils", not(target_arch = "wasm32")))]
pub mod test_utils {
    use async_trait::async_trait;

    type CdkResult<T> = std::result::Result<T, cdk::Error>;

    mockall::mock! {
        pub MintConnector {
        }
        impl std::fmt::Debug for MintConnector {
            fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> std::fmt::Result;
        }

        #[async_trait]
        impl cdk::wallet::MintConnector for MintConnector {
            async fn get_mint_keys(&self) -> CdkResult<Vec<cashu::KeySet>>;
            async fn get_mint_keyset(&self, keyset_id: cashu::Id) -> CdkResult<cashu::KeySet>;
            async fn get_mint_keysets(&self) -> CdkResult<cashu::KeysetResponse>;
            async fn post_mint_quote(
                &self,
                request: cashu::MintQuoteBolt11Request,
            ) -> CdkResult<cashu::MintQuoteBolt11Response<String>>;
            async fn get_mint_quote_status(
                &self,
                quote_id: &str,
            ) -> CdkResult<cashu::MintQuoteBolt11Response<String>>;
            async fn post_mint(&self, request: cashu::MintRequest<String>) -> CdkResult<cashu::MintResponse>;
            async fn post_melt_quote(
                &self,
                request: cashu::MeltQuoteBolt11Request,
            ) -> CdkResult<cashu::MeltQuoteBolt11Response<String>>;
            async fn get_melt_quote_status(
                &self,
                quote_id: &str,
            ) -> CdkResult<cashu::MeltQuoteBolt11Response<String>>;
            async fn post_melt(
                &self,
                request: cashu::MeltRequest<String>,
            ) -> CdkResult<cashu::MeltQuoteBolt11Response<String>>;
            async fn post_swap(&self, request: cashu::SwapRequest) -> CdkResult<cashu::SwapResponse>;
            async fn get_mint_info(&self) -> CdkResult<cashu::MintInfo>;
            async fn post_check_state(
                &self,
                request: cashu::CheckStateRequest,
            ) -> CdkResult<cashu::CheckStateResponse>;
            async fn post_restore(&self, request: cashu::RestoreRequest) -> CdkResult<cashu::RestoreResponse>;
            async fn post_mint_bolt12_quote(
                &self,
                request: cashu::MintQuoteBolt12Request,
            ) -> CdkResult<cashu::MintQuoteBolt12Response<String>>;
            async fn get_mint_quote_bolt12_status(
                &self,
                quote_id: &str,
            ) -> CdkResult<cashu::MintQuoteBolt12Response<String>>;
            async fn post_melt_bolt12_quote(
                &self,
                request: cashu::MeltQuoteBolt12Request,
            ) -> CdkResult<cashu::MeltQuoteBolt11Response<String>>;
            async fn get_melt_bolt12_quote_status(
                &self,
                quote_id: &str,
            ) -> CdkResult<cashu::MeltQuoteBolt11Response<String>>;
            async fn post_melt_bolt12(
                &self,
                request: cashu::MeltRequest<String>,
            ) -> CdkResult<cashu::MeltQuoteBolt11Response<String>>;
        }

        impl super::MintConnectorExt for MintConnector {
        }
    }
}
