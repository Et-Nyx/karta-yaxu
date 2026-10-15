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
        vec!["seed-fixtures", "--subject", "alice"],
        vec!["seed-reference-fixtures", "extra"],
        vec!["seed-world-fixtures"],
        vec!["seed-world-fixtures", "--owner-user-id"],
        vec!["seed-world-fixtures", "--owner-user-id", "invalid"],
        vec!["seed-world-fixtures", "--owner", "alice"],
        vec![
            "seed-world-fixtures",
            "--owner-user-id",
            "00000000000000000000000000000000",
            "--owner-user-id",
            "11111111111111111111111111111111",
        ],
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
    assert!(!admin(&["seed-fixtures"], &url).status.success());
    assert!(
        !parent.exists(),
        "initialization must require a migrated database"
    );
    assert!(!admin(&["seed-reference-fixtures"], &url).status.success());
    assert!(
        !admin(
            &[
                "seed-world-fixtures",
                "--owner-user-id",
                "00000000000000000000000000000000"
            ],
            &url
        )
        .status
        .success()
    );
    assert!(!parent.exists());
}

#[tokio::test]
async fn reference_and_world_cli_fixtures_require_real_owner_and_preserve_reruns() {
    use web_app::{
        application::{
            authoring::{Authoring, Edit},
            types::Types,
        },
        auth::Role,
    };
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}/fixtures.db", dir.path().display());
    assert!(admin(&["migrate"], &url).status.success());
    let pool = persistence::connect(&url).await.unwrap();
    let accounts = Accounts::new(pool.clone(), 3600).unwrap();
    let mut actors = Vec::new();
    for (name, role) in [
        ("owner", Role::Editor),
        ("viewer", Role::Viewer),
        ("other", Role::Administrator),
    ] {
        accounts.invite("test", name, role, None).await.unwrap();
        actors.push(
            accounts
                .sign_in(Subject {
                    provider: "test".into(),
                    provider_subject: name.into(),
                    display_name: name.into(),
                })
                .await
                .unwrap(),
        );
    }
    for id in [
        "00000000000000000000000000000000",
        actors[1].user_id.as_str(),
    ] {
        let result = admin(&["seed-world-fixtures", "--owner-user-id", id], &url);
        assert!(!result.status.success());
        assert!(
            String::from_utf8(result.stderr)
                .unwrap()
                .contains("existing enabled editor or administrator")
        );
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM types")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        count, 0,
        "invalid owner must not create reference prerequisites"
    );
    assert!(admin(&["seed-reference-fixtures"], &url).status.success());
    let result = admin(
        &["seed-world-fixtures", "--owner-user-id", &actors[0].user_id],
        &url,
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("2 worlds, 7 world Types, 5 variants")
    );
    let (token, _) = accounts.start_session(&actors[0].user_id).await.unwrap();
    let service = Authoring::new(accounts.clone());
    let world = service.worlds().await.unwrap().remove(0);
    let access = service.access(&token, &world.id).await.unwrap();
    assert_eq!(access.history[0].actor_name, "bundled-fixture-loader");
    let mut fields = world.fields.clone();
    fields.display_name = "Owner's edited lab".into();
    let changed = service
        .save_world(
            &token,
            &world.id,
            Edit {
                expected_revision: 1,
                fields,
                reason: Some("Keep this change".into()),
            },
        )
        .await
        .unwrap();
    let audit_count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    let result = admin(
        &["seed-world-fixtures", "--owner-user-id", &actors[0].user_id],
        &url,
    );
    assert!(result.status.success());
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("skipped 20 matching objects")
    );
    assert_eq!(service.world_detail(&world.id).await.unwrap(), changed);
    assert!(
        !admin(
            &["seed-world-fixtures", "--owner-user-id", &actors[2].user_id],
            &url
        )
        .status
        .success()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_events")
            .fetch_one(&pool)
            .await
            .unwrap(),
        audit_count
    );
    assert_eq!(
        accounts.users().await.unwrap().len(),
        3,
        "fixtures must never create accounts"
    );
    assert_eq!(Types::new(accounts.clone()).types().await.unwrap().len(), 3);
    pool.close().await;
}

#[tokio::test]
async fn explicit_initialization_is_idempotent_and_does_not_create_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}/content.db", dir.path().display());
    assert!(admin(&["migrate"], &url).status.success());
    let first = admin(&["seed-fixtures"], &url);
    assert!(first.status.success());
    assert!(
        String::from_utf8(first.stdout)
            .unwrap()
            .contains("Initialized 3 species and 3 default forms")
    );
    let second = admin(&["seed-fixtures"], &url);
    assert!(second.status.success());
    assert!(
        String::from_utf8(second.stdout)
            .unwrap()
            .contains("skipped 3 existing species")
    );
    let pool = persistence::connect(&url).await.unwrap();
    let users: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    let events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_events WHERE system_key = 'bundled-fixture-loader'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(users, 0);
    assert_eq!(events, 6);
    pool.close().await;
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
