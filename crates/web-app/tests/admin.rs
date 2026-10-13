use std::process::{Command, Output};
use web_app::{accounts::Accounts, auth::Subject, persistence};

fn admin(args: &[&str], url: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_admin"))
        .args(args)
        .env("KARTA_DATABASE_URL", url)
        .env("KARTA_SESSION_TTL_SECONDS", "3600")
        .output()
        .unwrap()
}

#[test]
fn invalid_commands_and_help_have_no_database_side_effects() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("must-not-exist");
    let url = format!("sqlite://{}/auth.db", parent.display());
    for args in [
        vec![],
        vec!["unknown"],
        vec!["migrate", "extra"],
        vec!["invite-user", "--subject", "--role", "administrator"],
        vec!["invite-user", "--subject", "alice", "--provider"],
        vec!["invite-user", "--subject", "alice", "--role"],
        vec!["invite-user", "--subject", "alice", "--subject", "bob"],
        vec!["invite-user", "--subject", "alice", "--rol", "editor"],
        vec!["invite-user", "--subject", "alice", "--role", "owner"],
        vec!["disable-user", "--id", "not-an-id"],
        vec!["list-users", "--subject", "alice"],
    ] {
        let result = admin(&args, &url);
        assert!(!result.status.success(), "accepted {args:?}");
        assert!(
            !parent.exists(),
            "invalid command created database directory"
        );
    }
    for args in [vec!["--help"], vec!["invite-user", "--help"]] {
        let result = Command::new(env!("CARGO_BIN_EXE_admin"))
            .args(args)
            .env("KARTA_DATABASE_URL", &url)
            .env("KARTA_SESSION_TTL_SECONDS", "invalid")
            .output()
            .unwrap();
        assert!(result.status.success());
        assert!(String::from_utf8(result.stdout).unwrap().contains("Usage:"));
        assert!(!parent.exists());
    }
    let result = admin(&["list-users"], &url);
    assert!(!result.status.success());
    assert!(!parent.exists(), "listing must not create a database");
}

#[tokio::test]
async fn operator_workflow_migrates_invites_disables_and_reenables() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}/auth.db", dir.path().display());
    for _ in 0..2 {
        let result = admin(&["migrate"], &url);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert!(
        admin(
            &["invite-user", "--subject", "Alice", "--role", "editor"],
            &url
        )
        .status
        .success()
    );
    let pool = persistence::connect(&url).await.unwrap();
    let accounts = Accounts::new(pool.clone(), 3600).unwrap();
    let identity = Subject {
        provider: "placeholder".into(),
        provider_subject: "alice".into(),
        display_name: "alice".into(),
    };
    let actor = accounts.sign_in(identity.clone()).await.unwrap();
    let (token, _) = accounts.start_session(&actor.user_id).await.unwrap();
    assert!(admin(&["list-users"], &url).status.success());
    assert!(
        !admin(
            &[
                "invite-user",
                "--subject",
                "alice",
                "--role",
                "administrator"
            ],
            &url
        )
        .status
        .success()
    );
    assert!(
        !admin(
            &["disable-user", "--id", "00000000000000000000000000000000"],
            &url
        )
        .status
        .success()
    );
    assert!(
        admin(&["disable-user", "--id", &actor.user_id], &url)
            .status
            .success()
    );
    assert!(accounts.authenticate(&token).await.unwrap().is_none());
    assert!(
        admin(&["enable-user", "--id", &actor.user_id], &url)
            .status
            .success()
    );
    assert!(accounts.authenticate(&token).await.unwrap().is_none());
    assert_eq!(
        accounts.sign_in(identity).await.unwrap().user_id,
        actor.user_id
    );
    let (fresh, _) = accounts.start_session(&actor.user_id).await.unwrap();
    assert!(accounts.authenticate(&fresh).await.unwrap().is_some());
    pool.close().await;
}

#[test]
fn server_rejects_missing_database_and_invalid_lifetime_before_listening() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("missing");
    let url = format!("sqlite://{}/auth.db", parent.display());
    for ttl in ["3600", "9223372036854775807"] {
        let result = Command::new(env!("CARGO_BIN_EXE_web-app"))
            .env("KARTA_DATABASE_URL", &url)
            .env("KARTA_SESSION_TTL_SECONDS", ttl)
            .env("KARTA_BIND", "127.0.0.1:0")
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(!parent.exists());
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(error.contains(if ttl == "3600" {
            "migrate"
        } else {
            "Session lifetime"
        }));
    }
}
