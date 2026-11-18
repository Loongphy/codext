//! Refreshes identity details omitted by account-update notifications without blocking terminal input.
//! Results carry a request ID so account changes and reconnects reject obsolete identities.

use super::App;
use super::AuthIdentity;
use crate::app_event::AppEvent;
use crate::app_server_session::AppServerSession;
use codex_app_server_protocol::Account;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::GetAccountParams;
use codex_app_server_protocol::GetAccountResponse;
use codex_app_server_protocol::RequestId;
use std::time::Duration;
use uuid::Uuid;

/// State of the identity resolution that follows an account change signal.
///
/// `Pending` is an in-flight `account/read` issued after an `AccountUpdated`
/// notification: the notification carries no email, so the refreshed identity
/// only becomes comparable once this read lands; `previous_identity` is the
/// snapshot taken before the display was updated, letting the response detect
/// a real account switch (and not a same-account token refresh) at that point.
/// `Fired` remembers the identity switch side effects already ran for, so a
/// second signal for the same change — the auth file watcher's `account/reload`
/// response or a reconnect bootstrap — does not emit a duplicate warning or
/// release parked work twice.
pub(super) enum AccountIdentityRefresh {
    Pending {
        request_id: Uuid,
        previous_identity: Option<AuthIdentity>,
    },
    Fired(AuthIdentity),
}

impl App {
    pub(super) fn refresh_account_email(
        &mut self,
        app_server: &AppServerSession,
        previous_identity: Option<AuthIdentity>,
    ) {
        let request_id = Uuid::new_v4();
        self.account_email_request_id = Some(AccountIdentityRefresh::Pending {
            request_id,
            previous_identity,
        });
        let request_handle = app_server.request_handle();
        let app_event_tx = self.app_event_tx.clone();
        tokio::spawn(async move {
            let result = tokio::time::timeout(
                Duration::from_secs(/*secs*/ 30),
                request_handle.request_typed::<GetAccountResponse>(ClientRequest::GetAccount {
                    request_id: RequestId::String(format!("account-status-{request_id}")),
                    params: GetAccountParams {
                        refresh_token: false,
                    },
                }),
            )
            .await;
            match result {
                Ok(Ok(response)) => {
                    if let Some(Account::Chatgpt {
                        email,
                        workspace_name,
                        ..
                    }) = response.account
                    {
                        app_event_tx.send(AppEvent::AccountEmailLoaded {
                            request_id,
                            email,
                            workspace: crate::app_server_session::workspace_display(
                                workspace_name,
                                response.workspace_routing,
                            ),
                        });
                    }
                }
                Ok(Err(_)) | Err(_) => {
                    tracing::warn!("failed to read account identity after account update");
                }
            }
        });
    }
}
