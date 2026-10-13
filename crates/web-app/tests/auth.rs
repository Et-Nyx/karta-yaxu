mod common;

use std::sync::Arc;

use async_trait::async_trait;
use axum::http::StatusCode;
use web_app::{
    accounts::Accounts,
    auth::{
        AuthError, Credential, IdentityProvider, Role, Subject, placeholder::PlaceholderProvider,
    },
    persistence,
};

const ORIGIN: &str = "http://127.0.0.1:3000";

fn subject(provider: &str, name: &str) -> Subject {
    Subject {
        provider: provider.to_owned(),
        provider_subject: name.to_owned(),
        display_name: name.to_owned(),
    }
}

#[tokio::test]
async fn placeholder_normalizes_and_rejects_bad_usernames() {
    let provider = PlaceholderProvider::new();
    let alice = provider
        .authenticate(Credential::DevUsername("  Alice ".to_owned()))
        .await
        .unwrap();
    assert_eq!(alice.provider, "placeholder");
    assert_eq!(alice.provider_subject, "alice");

    assert_eq!(
        provider
            .authenticate(Credential::DevUsername("   ".to_owned()))
            .await
            .unwrap_err(),
        AuthError::InvalidCredential("Enter a username.")
    );
    assert!(matches!(
        provider
            .authenticate(Credential::DevUsername("bad name".to_owned()))
            .await,
        Err(AuthError::InvalidCredential(_))
    ));
}

#[tokio::test]
async fn sign_in_requires_an_invite() {
    let app = common::app().await;
    assert_eq!(
        app.accounts.sign_in(subject("placeholder", "bob")).await,
        Err(AuthError::NotInvited)
    );
}

#[tokio::test]
async fn invited_user_is_created_once_and_reuses_its_id() {
    let app = common::app().await;
    app.accounts
        .invite("placeholder", "carol", Role::Editor, Some("first"))
        .await
        .unwrap();

    let first = app
        .accounts
        .sign_in(subject("placeholder", "carol"))
        .await
        .unwrap();
    assert_eq!(first.role, Role::Editor);
    assert_eq!(first.display_name, "carol");

    let invites = app.accounts.invites().await.unwrap();
    assert_eq!(invites.len(), 1);
    assert!(invites[0].consumed_at.is_some());

    let second = app
        .accounts
        .sign_in(subject("placeholder", "carol"))
        .await
        .unwrap();
    assert_eq!(first.user_id, second.user_id);
}

#[tokio::test]
async fn disabled_account_cannot_sign_in() {
    let app = common::app().await;
    app.accounts
        .invite("placeholder", "dave", Role::Viewer, None)
        .await
        .unwrap();
    let actor = app
        .accounts
        .sign_in(subject("placeholder", "dave"))
        .await
        .unwrap();
    app.accounts
        .set_disabled(&actor.user_id, true)
        .await
        .unwrap();
    assert_eq!(
        app.accounts.sign_in(subject("placeholder", "dave")).await,
        Err(AuthError::Disabled)
    );
}

#[tokio::test]
async fn sessions_are_hashed_and_revocable() {
    let app = common::app().await;
    app.accounts
        .invite("placeholder", "erin", Role::Viewer, None)
        .await
        .unwrap();
    let actor = app
        .accounts
        .sign_in(subject("placeholder", "erin"))
        .await
        .unwrap();
    let (token, expires_at) = app.accounts.start_session(&actor.user_id).await.unwrap();
    assert!(expires_at > 0);

    assert!(app.accounts.authenticate(&token).await.unwrap().is_some());

    let raw_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions WHERE token_hash = ?1")
        .bind(&token)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(raw_rows, 0, "raw session token must not be stored");

    app.accounts.sign_out(&token).await.unwrap();
    assert!(app.accounts.authenticate(&token).await.unwrap().is_none());
}

#[tokio::test]
async fn expired_sessions_do_not_authenticate() {
    let app = common::app().await;
    app.accounts
        .invite("placeholder", "frank", Role::Viewer, None)
        .await
        .unwrap();
    let actor = app
        .accounts
        .sign_in(subject("placeholder", "frank"))
        .await
        .unwrap();
    let (token, _) = app.accounts.start_session(&actor.user_id).await.unwrap();
    sqlx::query("UPDATE sessions SET expires_at = 0 WHERE user_id = ?1")
        .bind(&actor.user_id)
        .execute(&app.pool)
        .await
        .unwrap();
    assert!(app.accounts.authenticate(&token).await.unwrap().is_none());
}

#[tokio::test]
async fn invalid_lifetime_is_rejected_at_service_boundary() {
    let app = common::app().await;
    for lifetime in [-1, 0, 2_592_001, i64::MAX] {
        assert!(Accounts::new(app.pool.clone(), lifetime).is_err());
    }
}

#[tokio::test]
async fn account_page_requires_a_session() {
    let app = common::app().await;
    let response = common::send(&app.router, common::get("/account")).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(common::location(&response).as_deref(), Some("/login"));
}

#[tokio::test]
async fn login_post_requires_origin_and_csrf() {
    let app = common::app().await;
    app.accounts
        .invite("placeholder", "grace", Role::Viewer, None)
        .await
        .unwrap();

    let login = common::send(&app.router, common::get("/login")).await;
    let csrf = common::set_cookie(&login, "karta_csrf").unwrap();
    let cookie = format!("karta_csrf={csrf}");
    let form = format!("csrf={csrf}&username=grace");

    let no_origin = common::send(
        &app.router,
        common::post_form("/login", None, Some(&cookie), &form),
    )
    .await;
    assert_eq!(no_origin.status(), StatusCode::FORBIDDEN);

    // A missing CSRF field is a malformed form.
    let no_csrf = common::send(
        &app.router,
        common::post_form("/login", Some(ORIGIN), None, "username=grace"),
    )
    .await;
    assert_eq!(no_csrf.status(), StatusCode::BAD_REQUEST);

    // A CSRF value that does not match the cookie is forbidden.
    let mismatched = common::send(
        &app.router,
        common::post_form(
            "/login",
            Some(ORIGIN),
            Some(&cookie),
            &format!("csrf={}&username=grace", "0".repeat(64)),
        ),
    )
    .await;
    assert_eq!(mismatched.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn login_logout_roundtrip_and_uninvited_denied() {
    let app = common::app().await;
    app.accounts
        .invite("placeholder", "heidi", Role::Administrator, None)
        .await
        .unwrap();

    let login = common::send(&app.router, common::get("/login")).await;
    let csrf = common::set_cookie(&login, "karta_csrf").unwrap();
    let cookie = format!("karta_csrf={csrf}");

    // Uninvited username is rejected with a clear message.
    let denied = common::send(
        &app.router,
        common::post_form(
            "/login",
            Some(ORIGIN),
            Some(&cookie),
            &format!("csrf={csrf}&username=mallory"),
        ),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert!(
        common::read_body(denied)
            .await
            .contains("has not been invited")
    );

    // Invited user signs in; a session cookie is issued.
    let response = common::send(
        &app.router,
        common::post_form(
            "/login",
            Some(ORIGIN),
            Some(&cookie),
            &format!("csrf={csrf}&username=heidi"),
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(common::location(&response).as_deref(), Some("/account"));
    let session = common::set_cookie(&response, "karta_session").unwrap();

    let account = common::send(
        &app.router,
        common::with_cookie(
            common::get("/account"),
            format!("karta_session={session}; karta_csrf={csrf}"),
        ),
    )
    .await;
    assert_eq!(account.status(), StatusCode::OK);
    assert!(common::read_body(account).await.contains("heidi"));

    // Logout revokes the session.
    let logout = common::send(
        &app.router,
        common::post_form(
            "/logout",
            Some(ORIGIN),
            Some(&format!("karta_session={session}; karta_csrf={csrf}")),
            &format!("csrf={csrf}"),
        ),
    )
    .await;
    assert_eq!(logout.status(), StatusCode::SEE_OTHER);
    assert_eq!(common::location(&logout).as_deref(), Some("/"));

    let after = common::send(
        &app.router,
        common::with_cookie(common::get("/account"), format!("karta_session={session}")),
    )
    .await;
    assert_eq!(after.status(), StatusCode::SEE_OTHER);
    assert_eq!(common::location(&after).as_deref(), Some("/login"));
}

#[tokio::test]
async fn login_is_disabled_without_a_provider() {
    let app = common::app_with(None).await;
    let login = common::send(&app.router, common::get("/login")).await;
    let csrf = common::set_cookie(&login, "karta_csrf").unwrap();
    assert!(
        common::read_body(login)
            .await
            .contains("Authentication is disabled")
    );
    let response = common::send(
        &app.router,
        common::post_form(
            "/login",
            Some(ORIGIN),
            Some(&format!("karta_csrf={csrf}")),
            &format!("csrf={csrf}&username=anyone"),
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

/// A second provider exercises the port using the development credential flow.
/// This does not prove a real provider's browser bootstrap or identity migration.
struct FixedProvider;

#[async_trait]
impl IdentityProvider for FixedProvider {
    fn id(&self) -> &'static str {
        "fixed"
    }

    async fn authenticate(&self, _credential: Credential) -> Result<Subject, AuthError> {
        Ok(subject("fixed", "member"))
    }
}

#[tokio::test]
async fn second_provider_uses_the_existing_development_credential_flow() {
    let app = common::app_with(Some(Arc::new(FixedProvider))).await;
    app.accounts
        .invite("fixed", "member", Role::Editor, None)
        .await
        .unwrap();

    let login = common::send(&app.router, common::get("/login")).await;
    let csrf = common::set_cookie(&login, "karta_csrf").unwrap();
    let response = common::send(
        &app.router,
        common::post_form(
            "/login",
            Some(ORIGIN),
            Some(&format!("karta_csrf={csrf}")),
            &format!("csrf={csrf}&username=anything"),
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let session = common::set_cookie(&response, "karta_session").unwrap();

    let account = common::send(
        &app.router,
        common::with_cookie(common::get("/account"), format!("karta_session={session}")),
    )
    .await;
    assert_eq!(account.status(), StatusCode::OK);
    assert!(common::read_body(account).await.contains("member"));
}

#[tokio::test]
async fn invite_unique_pending_constraint_is_enforced() {
    let app = common::app().await;
    app.accounts
        .invite("placeholder", "ivan", Role::Viewer, None)
        .await
        .unwrap();
    let duplicate = app
        .accounts
        .invite("placeholder", "ivan", Role::Viewer, None)
        .await;
    assert!(duplicate.is_err());
}

#[tokio::test]
async fn foreign_keys_are_enabled() {
    let app = common::app().await;
    let mut connections = Vec::new();
    for _ in 0..5 {
        let mut connection = app.pool.acquire().await.unwrap();
        let enabled: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        assert_eq!(enabled, 1);
        let result =
            persistence::insert_session(&mut *connection, "deadbeef", "missing-user", 0, 0).await;
        assert!(
            result.is_err(),
            "orphan session must violate the foreign key"
        );
        connections.push(connection);
    }
}

async fn invited_actor(app: &common::TestApp, name: &str) -> web_app::auth::Actor {
    app.accounts
        .invite("placeholder", name, Role::Editor, None)
        .await
        .unwrap();
    app.accounts
        .sign_in(subject("placeholder", name))
        .await
        .unwrap()
}

#[tokio::test]
async fn concurrent_sign_ins_claim_each_invite_once() {
    let app = common::app().await;
    for name in ["same", "other", "third"] {
        app.accounts
            .invite("placeholder", name, Role::Editor, None)
            .await
            .unwrap();
    }
    let (first, second, other, third) = tokio::join!(
        app.accounts.sign_in(subject("placeholder", "same")),
        app.accounts.sign_in(subject("placeholder", "same")),
        app.accounts.sign_in(subject("placeholder", "other")),
        app.accounts.sign_in(subject("placeholder", "third")),
    );
    assert_eq!(first.unwrap().user_id, second.unwrap().user_id);
    assert_ne!(other.unwrap().user_id, third.unwrap().user_id);
    assert_eq!(app.accounts.users().await.unwrap().len(), 3);
    let invites = app.accounts.invites().await.unwrap();
    assert_eq!(invites.len(), 3);
    assert!(invites.iter().all(|invite| invite.consumed_at.is_some()));
}

#[tokio::test]
async fn failed_invite_claim_rolls_back_the_new_account() {
    let app = common::app().await;
    app.accounts
        .invite("placeholder", "rollback", Role::Viewer, None)
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER reject_claim BEFORE UPDATE ON invites BEGIN SELECT RAISE(ABORT, 'test failure'); END")
        .execute(&app.pool).await.unwrap();
    assert_eq!(
        app.accounts
            .sign_in(subject("placeholder", "rollback"))
            .await,
        Err(AuthError::Unavailable)
    );
    assert!(app.accounts.users().await.unwrap().is_empty());
    assert!(
        app.accounts.invites().await.unwrap()[0]
            .consumed_at
            .is_none()
    );
    sqlx::query("DROP TRIGGER reject_claim")
        .execute(&app.pool)
        .await
        .unwrap();
    assert!(
        app.accounts
            .sign_in(subject("placeholder", "rollback"))
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn reinviting_an_account_does_not_create_a_pending_role_change() {
    let app = common::app().await;
    let actor = invited_actor(&app, "existing").await;
    let error = app
        .accounts
        .invite("placeholder", "existing", Role::Administrator, None)
        .await
        .unwrap_err();
    assert!(error.contains("already exists"));
    assert_eq!(app.accounts.invites().await.unwrap().len(), 1);
    assert_eq!(
        app.accounts
            .sign_in(subject("placeholder", "existing"))
            .await
            .unwrap(),
        actor
    );
    assert!(
        app.accounts
            .set_disabled("missing", true)
            .await
            .unwrap_err()
            .contains("not found")
    );
}

#[tokio::test]
async fn disabling_revokes_all_sessions_and_reenable_requires_new_sign_in() {
    let app = common::app().await;
    let actor = invited_actor(&app, "disable").await;
    let other = invited_actor(&app, "unaffected").await;
    let (one, _) = app.accounts.start_session(&actor.user_id).await.unwrap();
    let (two, _) = app.accounts.start_session(&actor.user_id).await.unwrap();
    let (unaffected, _) = app.accounts.start_session(&other.user_id).await.unwrap();
    app.accounts
        .set_disabled(&actor.user_id, true)
        .await
        .unwrap();
    assert_eq!(
        app.accounts.start_session(&actor.user_id).await,
        Err(AuthError::Disabled)
    );
    assert!(app.accounts.authenticate(&one).await.unwrap().is_none());
    assert!(app.accounts.authenticate(&two).await.unwrap().is_none());
    assert!(
        app.accounts
            .authenticate(&unaffected)
            .await
            .unwrap()
            .is_some()
    );
    app.accounts
        .set_disabled(&actor.user_id, false)
        .await
        .unwrap();
    assert!(app.accounts.authenticate(&one).await.unwrap().is_none());
    assert!(app.accounts.authenticate(&two).await.unwrap().is_none());
    let signed_in = app
        .accounts
        .sign_in(subject("placeholder", "disable"))
        .await
        .unwrap();
    assert_eq!(signed_in.user_id, actor.user_id);
    let (fresh, _) = app
        .accounts
        .start_session(&signed_in.user_id)
        .await
        .unwrap();
    assert!(app.accounts.authenticate(&fresh).await.unwrap().is_some());
    app.accounts.sign_out(&fresh).await.unwrap();
    app.accounts.sign_out(&fresh).await.unwrap();
    app.accounts.sign_out("unknown-token").await.unwrap();
}

#[tokio::test]
async fn failed_revocation_rolls_back_disabling_and_preserves_sessions() {
    let app = common::app().await;
    let actor = invited_actor(&app, "atomic").await;
    let (token, _) = app.accounts.start_session(&actor.user_id).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_revoke BEFORE UPDATE ON sessions BEGIN SELECT RAISE(ABORT, 'test failure'); END")
        .execute(&app.pool).await.unwrap();
    assert!(
        app.accounts
            .set_disabled(&actor.user_id, true)
            .await
            .is_err()
    );
    assert!(app.accounts.authenticate(&token).await.unwrap().is_some());
    assert_eq!(
        app.accounts
            .sign_in(subject("placeholder", "atomic"))
            .await
            .unwrap(),
        actor
    );
    sqlx::query("DROP TRIGGER reject_revoke")
        .execute(&app.pool)
        .await
        .unwrap();
    app.accounts
        .set_disabled(&actor.user_id, true)
        .await
        .unwrap();
    assert!(app.accounts.authenticate(&token).await.unwrap().is_none());
}

#[tokio::test]
async fn session_creation_racing_with_disable_cannot_escape_revocation() {
    let app = common::app().await;
    let actor = invited_actor(&app, "race").await;
    let (created, disabled) = tokio::join!(
        app.accounts.start_session(&actor.user_id),
        app.accounts.set_disabled(&actor.user_id, true),
    );
    disabled.unwrap();
    app.accounts
        .set_disabled(&actor.user_id, false)
        .await
        .unwrap();
    match created {
        Ok((token, _)) => assert!(app.accounts.authenticate(&token).await.unwrap().is_none()),
        Err(error) => assert_eq!(error, AuthError::Disabled),
    }
    assert_eq!(
        app.accounts.start_session("missing").await,
        Err(AuthError::NotInvited)
    );
}

#[tokio::test]
async fn session_and_account_identity_survive_reopening_the_database() {
    let app = common::app().await;
    let actor = invited_actor(&app, "persistent").await;
    let (token, _) = app.accounts.start_session(&actor.user_id).await.unwrap();
    let url = format!(
        "sqlite://{}",
        app.pool.connect_options().get_filename().display()
    );
    app.pool.close().await;
    let pool = persistence::connect(&url).await.unwrap();
    persistence::require_schema(&pool).await.unwrap();
    let accounts = Accounts::new(pool.clone(), 3600).unwrap();
    assert_eq!(accounts.authenticate(&token).await.unwrap(), Some(actor));
    pool.close().await;
}

#[tokio::test]
async fn csrf_rejects_invalid_shapes_and_duplicate_cookies_for_login_and_logout() {
    let app = common::app().await;
    let valid = "a".repeat(64);
    for token in [
        "".to_owned(),
        "short".to_owned(),
        "g".repeat(64),
        "a".repeat(65),
    ] {
        for path in ["/login", "/logout"] {
            let response = common::send(
                &app.router,
                common::post_form(
                    path,
                    Some(ORIGIN),
                    Some(&format!("karta_csrf={token}")),
                    &format!("csrf={token}&username=anyone"),
                ),
            )
            .await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
    }
    for path in ["/login", "/logout"] {
        let duplicate = format!("karta_csrf={valid}; karta_csrf={valid}");
        let response = common::send(
            &app.router,
            common::post_form(
                path,
                Some(ORIGIN),
                Some(&duplicate),
                &format!("csrf={valid}&username=anyone"),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    for origin in [None, Some("https://another.example")] {
        let response = common::send(
            &app.router,
            common::post_form(
                "/logout",
                origin,
                Some(&format!("karta_csrf={valid}")),
                &format!("csrf={valid}"),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}

#[tokio::test]
async fn oversized_authentication_forms_are_rejected_without_sessions() {
    let app = common::app().await;
    let csrf = "a".repeat(64);
    for path in ["/login", "/logout"] {
        let response = common::send(
            &app.router,
            common::post_form(
                path,
                Some(ORIGIN),
                Some(&format!("karta_csrf={csrf}")),
                &format!("csrf={csrf}&username={}", "a".repeat(4096)),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert!(common::read_body(response).await.contains("4 KiB"));
    }
    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(sessions, 0);
}

#[tokio::test]
async fn login_budget_is_shared_across_router_clones_and_precedes_body_parsing() {
    let app = common::app().await;
    for _ in 0..30 {
        let clone = app.router.clone();
        let response = common::send(
            &clone,
            common::post_form("/login", Some(ORIGIN), None, "username=missing-csrf"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let response = common::send(
        &app.router,
        common::post_form("/login", Some(ORIGIN), None, &"a".repeat(5000)),
    )
    .await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry: u64 = response.headers()["retry-after"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((1..=60).contains(&retry));
    // The login page and logout remain available even when POST login is limited.
    assert_eq!(
        common::send(&app.router, common::get("/login"))
            .await
            .status(),
        StatusCode::OK
    );
    let csrf = "a".repeat(64);
    let logout = common::send(
        &app.router,
        common::post_form(
            "/logout",
            Some(ORIGIN),
            Some(&format!("karta_csrf={csrf}")),
            &format!("csrf={csrf}"),
        ),
    )
    .await;
    assert_eq!(logout.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn storage_failure_is_not_reported_as_signed_out_and_preserves_login_input() {
    let app = common::app().await;
    let actor = invited_actor(&app, "outage").await;
    let (token, _) = app.accounts.start_session(&actor.user_id).await.unwrap();
    let csrf = "a".repeat(64);
    let cookie = format!("karta_session={token}; karta_csrf={csrf}");
    app.pool.close().await;
    for path in ["/login", "/account"] {
        let response = common::send(
            &app.router,
            common::with_cookie(common::get(path), cookie.clone()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(common::location(&response).is_none());
        assert!(common::set_cookie(&response, "karta_session").is_none());
    }
    let response = common::send(
        &app.router,
        common::post_form(
            "/login",
            Some(ORIGIN),
            Some(&cookie),
            &format!("csrf={csrf}&username=outage"),
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(
        common::read_body(response)
            .await
            .contains("value=\"outage\"")
    );
}

#[tokio::test]
async fn failed_logout_preserves_cookies_and_can_be_retried() {
    let app = common::app().await;
    let actor = invited_actor(&app, "logout-failure").await;
    let (token, _) = app.accounts.start_session(&actor.user_id).await.unwrap();
    let csrf = "a".repeat(64);
    let cookie = format!("karta_session={token}; karta_csrf={csrf}");
    sqlx::query("CREATE TRIGGER reject_logout BEFORE UPDATE ON sessions BEGIN SELECT RAISE(ABORT, 'test failure'); END")
        .execute(&app.pool).await.unwrap();
    let response = common::send(
        &app.router,
        common::post_form(
            "/logout",
            Some(ORIGIN),
            Some(&cookie),
            &format!("csrf={csrf}"),
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(common::location(&response).is_none());
    assert!(common::set_cookie(&response, "karta_session").is_none());
    assert!(common::set_cookie(&response, "karta_csrf").is_none());
    let body = common::read_body(response).await;
    assert!(body.contains("session may still be active"));
    assert!(body.contains("Try signing out again"));
    assert!(app.accounts.authenticate(&token).await.unwrap().is_some());
    sqlx::query("DROP TRIGGER reject_logout")
        .execute(&app.pool)
        .await
        .unwrap();
    let response = common::send(
        &app.router,
        common::post_form(
            "/logout",
            Some(ORIGIN),
            Some(&cookie),
            &format!("csrf={csrf}"),
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        common::set_cookie(&response, "karta_session").as_deref(),
        Some("")
    );
    assert!(app.accounts.authenticate(&token).await.unwrap().is_none());
}

#[tokio::test]
async fn ambiguous_session_cookies_do_not_claim_successful_logout() {
    let app = common::app().await;
    let actor = invited_actor(&app, "duplicate-session").await;
    let (token, _) = app.accounts.start_session(&actor.user_id).await.unwrap();
    let csrf = "a".repeat(64);
    let cookie = format!("karta_csrf={csrf}; karta_session={token}; karta_session={token}");
    let response = common::send(
        &app.router,
        common::post_form(
            "/logout",
            Some(ORIGIN),
            Some(&cookie),
            &format!("csrf={csrf}"),
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(common::set_cookie(&response, "karta_session").is_none());
    assert!(
        common::read_body(response)
            .await
            .contains("Sign-out was not completed")
    );
    assert!(app.accounts.authenticate(&token).await.unwrap().is_some());
}

#[tokio::test]
async fn secure_cookie_attributes_and_expiry_survive_login_logout() {
    let app = common::app_with_options(
        game_data::Catalog::from_json(game_data::BUNDLED_FIXTURES).unwrap(),
        Some(Arc::new(PlaceholderProvider::new())),
        true,
    )
    .await;
    invited_actor(&app, "secure").await;
    let login = common::send(&app.router, common::get("/login")).await;
    let csrf = common::set_cookie(&login, "karta_csrf").unwrap();
    for value in login.headers().get_all("set-cookie") {
        let value = value.to_str().unwrap();
        for attribute in ["HttpOnly", "SameSite=Strict", "; Secure", "Max-Age=3600"] {
            assert!(value.contains(attribute));
        }
    }
    let response = common::send(
        &app.router,
        common::post_form(
            "/login",
            Some(ORIGIN),
            Some(&format!("karta_csrf={csrf}")),
            &format!("csrf={csrf}&username=secure"),
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let token = common::set_cookie(&response, "karta_session").unwrap();
    let value = response.headers()["set-cookie"].to_str().unwrap();
    for attribute in ["HttpOnly", "SameSite=Strict", "; Secure", "Max-Age=3600"] {
        assert!(value.contains(attribute));
    }
    let lifetime: i64 = sqlx::query_scalar("SELECT expires_at - created_at FROM sessions")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(lifetime, 3600);
    let response = common::send(
        &app.router,
        common::post_form(
            "/logout",
            Some(ORIGIN),
            Some(&format!("karta_session={token}; karta_csrf={csrf}")),
            &format!("csrf={csrf}"),
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    for value in response.headers().get_all("set-cookie") {
        let value = value.to_str().unwrap();
        for attribute in ["HttpOnly", "SameSite=Strict", "; Secure", "Max-Age=0"] {
            assert!(value.contains(attribute));
        }
    }
}
