//! Trusted local operator CLI. Validate all inputs before database access.
//! Operators are authorized by access to the executable/database, not web roles.

use game_data::{BUNDLED_FIXTURES, Catalog};
use std::{collections::BTreeMap, process::ExitCode};
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{Authoring, Error},
        card_workshop::CardWorkshop,
        fixtures::{Fixtures, SeedOptions},
        types::{FixtureReport, Types},
    },
    auth::{Role, placeholder},
    config::Config,
    persistence,
};

const USAGE: &str = "\
Usage: admin <command> [options]

Commands:
  migrate                 Create the database and apply pending migrations.
  seed-fixtures           Initialize all bundled content atomically; never overwrite edits.
      --owner-user-id <id>  Existing eligible owner; otherwise retain fixture owner/sole admin.
      --admin-subject <name> Bootstrap username if no users exist (default: admin).
                           Bootstrap requires KARTA_DEV_AUTH=1; options are mutually exclusive.
  seed-creature-fixtures  Initialize only the original species/default forms.
  seed-reference-fixtures  Initialize the cached Type/Charizard reference subset.
  seed-world-fixtures --owner-user-id <id>
                          Initialize two synthetic worlds owned by an existing account.
  seed-card-workshop --owner-user-id <id>
                          Initialize the original Card Workshop atomically (15 objects).
                          Requires an existing enabled editor/administrator; creates no accounts.
  invite-user             Pre-authorize an identity for first sign-in.
      --subject <name>      Required. Placeholder username, or provider subject.
      --role <role>         viewer | editor | administrator (default: viewer).
      --provider <id>       Identity provider id (default: placeholder).
      --note <text>         Optional note (up to 1,000 characters).
  disable-user --id <id>   Disable an account and permanently revoke its sessions.
  enable-user --id <id>    Re-enable an account; a fresh sign-in is required.
  list-users              List local accounts and immutable ids.
  list-invites            List invites and whether they were consumed.
  --help                  Show this help without database access.";

enum Command {
    Help,
    Migrate,
    SeedFixtures {
        owner_user_id: Option<String>,
        admin_subject: Option<String>,
    },
    SeedCreatureFixtures,
    SeedReferenceFixtures,
    SeedWorldFixtures {
        owner_user_id: String,
    },
    SeedCardWorkshop {
        owner_user_id: String,
    },
    Invite {
        provider: String,
        subject: String,
        role: Role,
        note: Option<String>,
    },
    SetDisabled {
        id: String,
        disabled: bool,
    },
    ListUsers,
    ListInvites,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = parse(&args)?;
    if matches!(command, Command::Help) {
        println!("{USAGE}");
        return Ok(());
    }
    let config = Config::from_env()?;
    let pool = if matches!(command, Command::Migrate) {
        persistence::connect_for_migration(&config.database_url).await?
    } else {
        let pool = persistence::connect(&config.database_url).await?;
        persistence::require_schema(&pool).await?;
        pool
    };
    let accounts = Accounts::new(pool.clone(), config.session_ttl_seconds)?;
    match command {
        Command::Migrate => {
            persistence::migrate(&pool).await?;
            println!("Migrations applied.");
        }
        Command::SeedFixtures {
            owner_user_id,
            admin_subject,
        } => {
            let report = Fixtures::new(accounts.clone())
                .seed(SeedOptions {
                    owner_user_id: owner_user_id.as_deref(),
                    admin_subject: admin_subject.as_deref(),
                    dev_auth: config.dev_auth,
                })
                .await
                .map_err(authoring_error)?;
            print_fixture_report(report.fixtures);
            println!("Sample-world owner: {}", report.owner_user_id);
            if let Some(subject) = report.bootstrap_subject {
                println!(
                    "Created development administrator. Sign in as {subject} with KARTA_DEV_AUTH=1."
                );
            }
        }
        Command::SeedCreatureFixtures => {
            let catalog = Catalog::from_json(BUNDLED_FIXTURES)?;
            let report = Authoring::new(accounts.clone())
                .seed_catalog(&catalog)
                .await
                .map_err(|error| error.to_string())?;
            println!(
                "Initialized {} species and {} default forms; skipped {} existing species.",
                report.species_created, report.forms_created, report.species_skipped
            );
        }
        Command::SeedReferenceFixtures => {
            let report = Types::new(accounts.clone())
                .seed_reference_fixtures()
                .await
                .map_err(authoring_error)?;
            print_fixture_report(report);
        }
        Command::SeedWorldFixtures { owner_user_id } => {
            let report = Types::new(accounts.clone())
                .seed_world_fixtures(&owner_user_id)
                .await
                .map_err(authoring_error)?;
            print_fixture_report(report);
        }
        Command::SeedCardWorkshop { owner_user_id } => {
            let report = CardWorkshop::new(accounts.clone())
                .seed(&owner_user_id)
                .await
                .map_err(authoring_error)?;
            println!(
                "Card Workshop {}: created {} objects; skipped {} matching objects ({} cards, {} fields).",
                report.world_id, report.created, report.skipped, report.cards, report.fields
            );
        }
        Command::Invite {
            provider,
            subject,
            role,
            note,
        } => {
            let id = accounts
                .invite(&provider, &subject, role, note.as_deref())
                .await?;
            println!(
                "Invited {provider}:{subject} as {} (invite {id})",
                role.as_str()
            );
        }
        Command::SetDisabled { id, disabled } => {
            accounts.set_disabled(&id, disabled).await?;
            if disabled {
                println!("Disabled account {id}; all its sessions are revoked.");
            } else {
                println!("Enabled account {id}; revoked sessions are not restored.");
            }
        }
        Command::ListUsers => {
            for user in accounts.users().await? {
                let state = if user.is_disabled != 0 {
                    "disabled"
                } else {
                    "active"
                };
                println!(
                    "{} [{state}] role={} provider={} subject={} id={}",
                    user.display_name, user.role, user.provider, user.provider_subject, user.id
                );
            }
        }
        Command::ListInvites => {
            for invite in accounts.invites().await? {
                let state = if invite.consumed_at.is_some() {
                    "consumed"
                } else {
                    "pending"
                };
                println!(
                    "{} [{state}] role={} provider={} subject={} note={}",
                    invite.id,
                    invite.role,
                    invite.provider,
                    invite.provider_subject,
                    invite.note.unwrap_or_default()
                );
            }
        }
        Command::Help => unreachable!(),
    }
    pool.close().await;
    Ok(())
}

fn parse(args: &[String]) -> Result<Command, String> {
    let name = args.first().ok_or(USAGE)?;
    if args.len() == 1 && matches!(name.as_str(), "--help" | "-h" | "help") {
        return Ok(Command::Help);
    }
    let allowed: &[&str] = match name.as_str() {
        "migrate"
        | "seed-creature-fixtures"
        | "seed-reference-fixtures"
        | "list-users"
        | "list-invites" => &[],
        "seed-fixtures" => &["--owner-user-id", "--admin-subject"],
        "seed-world-fixtures" | "seed-card-workshop" => &["--owner-user-id"],
        "invite-user" => &["--subject", "--role", "--provider", "--note"],
        "disable-user" | "enable-user" => &["--id"],
        _ => return Err(format!("Unknown command.\n{USAGE}")),
    };
    if args.len() == 2 && args[1] == "--help" {
        return Ok(Command::Help);
    }
    let mut options = BTreeMap::new();
    let mut index = 1;
    while index < args.len() {
        let key = args[index].as_str();
        if !allowed.contains(&key) {
            return Err(format!(
                "Unsupported option or unexpected argument for {name}.\n{USAGE}"
            ));
        }
        let value = args
            .get(index + 1)
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("Missing value for {key}"))?;
        if options.insert(key, value.clone()).is_some() {
            return Err(format!("Duplicate option {key}"));
        }
        index += 2;
    }
    match name.as_str() {
        "migrate" => Ok(Command::Migrate),
        "seed-fixtures" => {
            let owner_user_id = options.remove("--owner-user-id");
            let admin_subject = options
                .remove("--admin-subject")
                .map(|raw| placeholder::normalize(&raw).map_err(|e| e.to_string()))
                .transpose()?;
            if owner_user_id.is_some() && admin_subject.is_some() {
                return Err("Choose --owner-user-id or --admin-subject, not both.".into());
            }
            if let Some(id) = &owner_user_id {
                validate_id(id)?;
            }
            Ok(Command::SeedFixtures {
                owner_user_id,
                admin_subject,
            })
        }
        "seed-creature-fixtures" => Ok(Command::SeedCreatureFixtures),
        "seed-reference-fixtures" => Ok(Command::SeedReferenceFixtures),
        "seed-world-fixtures" | "seed-card-workshop" => {
            let id = options
                .remove("--owner-user-id")
                .ok_or("Missing --owner-user-id")?;
            validate_id(&id)?;
            if name == "seed-card-workshop" {
                Ok(Command::SeedCardWorkshop { owner_user_id: id })
            } else {
                Ok(Command::SeedWorldFixtures { owner_user_id: id })
            }
        }
        "list-users" => Ok(Command::ListUsers),
        "list-invites" => Ok(Command::ListInvites),
        "disable-user" | "enable-user" => {
            let id = options.remove("--id").ok_or("Missing --id")?;
            validate_id(&id)?;
            Ok(Command::SetDisabled {
                id,
                disabled: name == "disable-user",
            })
        }
        "invite-user" => {
            let provider = options
                .remove("--provider")
                .unwrap_or_else(|| placeholder::PROVIDER_ID.to_owned());
            if provider.is_empty()
                || provider.len() > 64
                || !provider
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
            {
                return Err("Provider id must contain 1–64 letters, digits, hyphens, underscores, or periods.".to_owned());
            }
            let raw = options.remove("--subject").ok_or("Missing --subject")?;
            let subject = if provider == placeholder::PROVIDER_ID {
                placeholder::normalize(&raw).map_err(|error| error.to_string())?
            } else {
                if raw.trim().is_empty()
                    || raw.chars().count() > 256
                    || raw.chars().any(char::is_control)
                {
                    return Err("Provider subject must contain 1–256 characters without control characters.".to_owned());
                }
                raw
            };
            let role = match options.remove("--role") {
                None => Role::Viewer,
                Some(raw) => Role::parse(&raw)
                    .ok_or("Invalid --role; use viewer, editor, or administrator")?,
            };
            let note = options.remove("--note");
            if note.as_ref().is_some_and(|value| {
                value.chars().count() > 1000 || value.chars().any(char::is_control)
            }) {
                return Err(
                    "Note must be at most 1,000 characters without control characters.".to_owned(),
                );
            }
            Ok(Command::Invite {
                provider,
                subject,
                role,
                note,
            })
        }
        _ => unreachable!(),
    }
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.len() != 32
        || !id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("Use the 32-character immutable user id from list-users.".to_owned());
    }
    Ok(())
}

fn authoring_error(error: Error) -> String {
    match error {
        Error::Invalid(errors) => errors
            .into_iter()
            .map(|(field, message)| format!("{field}: {message}"))
            .collect::<Vec<_>>()
            .join("\n"),
        error => error.to_string(),
    }
}

fn print_fixture_report(report: FixtureReport) {
    println!(
        "Initialized {} shared Types, {} species, {} forms, {} worlds, {} world Types, {} variants; skipped {} matching objects. Initialized {} shared charts and {} world charts. Initialized {} shared Stats, {} world Stats and {} Stat rosters. Initialized {} move attributes, {} shared moves, {} world moves, {} rulesets, {} learnsets and {} moveset presets.",
        report.types_created,
        report.species_created,
        report.forms_created,
        report.worlds_created,
        report.world_types_created,
        report.variants_created,
        report.skipped,
        report.shared_charts_created,
        report.world_charts_created,
        report.shared_stats_created,
        report.world_stats_created,
        report.stat_rosters_created,
        report.move_attributes_created,
        report.shared_moves_created,
        report.world_moves_created,
        report.rulesets_created,
        report.learnsets_created,
        report.movesets_created
    );
}
