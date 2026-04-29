use anyhow::{Result, anyhow};
use flutter_rust_bridge::frb;
use std::future::Future;
use std::sync::Mutex;

use sia_core::types::Hash256;

use super::io::run_local;
use super::keys::AppKey;
use super::sdk::Sdk;
use super::types::AppMetadata;

enum BuilderState {
    Disconnected(sia_storage::Builder<sia_storage::DisconnectedState>),
    RequestingApproval(sia_storage::Builder<sia_storage::RequestingApprovalState>),
    Approved(sia_storage::Builder<sia_storage::ApprovedState>),
    Finalized,
}

/// Builder onboards an application to a Sia indexer.
///
/// Two flows are supported:
///
/// 1. **Reconnect**: call [Builder::connected] with an existing [AppKey]. If
///    the key is registered the SDK is returned; otherwise `None`.
/// 2. **Approve & register**: call [Builder::request_connection], read
///    [Builder::response_url] to display the approval URL, then
///    [Builder::wait_for_approval], then [Builder::register] with the user's
///    BIP-39 mnemonic.
///
/// Each successful state transition consumes the builder; subsequent calls
/// from a wrong state error out.
#[frb(opaque)]
pub struct Builder {
    state: Mutex<Option<BuilderState>>,
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for Builder {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for Builder {}

impl Builder {
    async fn with_state_transition<F, Fut, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(BuilderState) -> Fut,
        Fut: Future<Output = Result<(BuilderState, R)>>,
    {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("builder mutex poisoned"))?
            .take();
        match state {
            Some(state) => {
                let (next, result) = f(state).await?;
                *self
                    .state
                    .lock()
                    .map_err(|_| anyhow!("builder mutex poisoned"))? = Some(next);
                Ok(result)
            }
            None => Err(anyhow!("builder is in an invalid state")),
        }
    }

    /// Creates a new SDK builder targeting the provided indexer URL.
    #[frb(sync)]
    pub fn new(indexer_url: String, app_meta: AppMetadata) -> Result<Self> {
        let id_arr: [u8; 32] = app_meta
            .id
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("app ID must be 32 bytes"))?;
        // sia_storage::AppMetadata holds &'static str fields, so we leak the
        // owned strings. A Builder is typically constructed once per app
        // session, so this leak is bounded.
        let static_meta = sia_storage::AppMetadata {
            id: Hash256::from(id_arr),
            name: Box::leak(app_meta.name.into_boxed_str()),
            description: Box::leak(app_meta.description.into_boxed_str()),
            service_url: Box::leak(app_meta.service_url.into_boxed_str()),
            logo_url: app_meta
                .logo_url
                .map(|s| Box::leak(s.into_boxed_str()) as &'static str),
            callback_url: app_meta
                .callback_url
                .map(|s| Box::leak(s.into_boxed_str()) as &'static str),
        };
        let builder = sia_storage::Builder::new(indexer_url, static_meta)
            .map_err(|e| anyhow!("{e}"))?;
        Ok(Builder {
            state: Mutex::new(Some(BuilderState::Disconnected(builder))),
        })
    }

    /// Attempts to connect using the provided app key. Returns the SDK if the
    /// key is recognized by the indexer, otherwise `None`.
    pub async fn connected(&self, app_key: &AppKey) -> Result<Option<Sdk>> {
        run_local(async {
            let ak = app_key.inner.clone();
            self.with_state_transition(|state| async move {
                match state {
                    BuilderState::Disconnected(builder) => {
                        match builder.connected(&ak).await.map_err(|e| anyhow!("{e}"))? {
                            Some(sdk) => Ok((BuilderState::Finalized, Some(Sdk { inner: sdk }))),
                            None => Ok((BuilderState::Disconnected(builder), None)),
                        }
                    }
                    _ => Err(anyhow!("builder is in an invalid state")),
                }
            })
            .await
        })
        .await
    }

    /// Requests connection approval for the application. Transitions the
    /// builder into the requesting-approval state.
    pub async fn request_connection(&self) -> Result<()> {
        run_local(self.with_state_transition(|state| async move {
            match state {
                BuilderState::Disconnected(builder) => {
                    let next = builder
                        .request_connection()
                        .await
                        .map_err(|e| anyhow!("{e}"))?;
                    Ok((BuilderState::RequestingApproval(next), ()))
                }
                _ => Err(anyhow!("builder is in an invalid state")),
            }
        }))
        .await
    }

    /// Returns the URL at which the user must approve the connection request.
    /// Only valid after [Builder::request_connection].
    #[frb(sync)]
    pub fn response_url(&self) -> Result<String> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("builder mutex poisoned"))?;
        match state.as_ref() {
            Some(BuilderState::RequestingApproval(b)) => Ok(b.response_url().to_owned()),
            _ => Err(anyhow!("builder is in an invalid state")),
        }
    }

    /// Blocks until the connection request is approved by the user.
    pub async fn wait_for_approval(&self) -> Result<()> {
        run_local(self.with_state_transition(|state| async move {
            match state {
                BuilderState::RequestingApproval(builder) => {
                    let next = builder
                        .wait_for_approval()
                        .await
                        .map_err(|e| anyhow!("{e}"))?;
                    Ok((BuilderState::Approved(next), ()))
                }
                _ => Err(anyhow!("builder is in an invalid state")),
            }
        }))
        .await
    }

    /// Registers the application with the indexer using the provided
    /// recovery phrase. Returns the live SDK.
    pub async fn register(&self, mnemonic: String) -> Result<Sdk> {
        run_local(self.with_state_transition(|state| async move {
            match state {
                BuilderState::Approved(builder) => {
                    let sdk = builder
                        .register(&mnemonic)
                        .await
                        .map_err(|e| anyhow!("{e}"))?;
                    Ok((BuilderState::Finalized, Sdk { inner: sdk }))
                }
                _ => Err(anyhow!("builder is in an invalid state")),
            }
        }))
        .await
    }
}
