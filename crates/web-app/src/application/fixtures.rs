//! Trusted operator setup of all bundled fixtures in one write transaction.
use super::{
    authoring::{Authoring, Error, invalid},
    chart_fixtures::ChartFixtures,
    charts::Charts,
    stat_fixtures::StatFixtures,
    stats::Stats,
    types::{FixtureReport, ReferenceFixtures, Types, WorldFixtures},
};
use crate::{
    accounts::Accounts,
    auth::placeholder,
    persistence::{self as store, authoring::storage_error},
};
use game_data::{BUNDLED_FIXTURES, Catalog};

#[derive(Debug, Default)]
pub struct SeedOptions<'a> {
    pub owner_user_id: Option<&'a str>,
    pub admin_subject: Option<&'a str>,
    pub dev_auth: bool,
}

#[derive(Debug)]
pub struct SetupReport {
    pub fixtures: FixtureReport,
    pub owner_user_id: String,
    pub bootstrap_subject: Option<String>,
}

pub struct Fixtures {
    accounts: Accounts,
}

impl Fixtures {
    pub fn new(accounts: Accounts) -> Self {
        Self { accounts }
    }

    /// No session or role proof is accepted from the browser. Only the local
    /// operator CLI calls this use case; every helper shares the same connection.
    pub async fn seed(&self, options: SeedOptions<'_>) -> Result<SetupReport, Error> {
        if options.owner_user_id.is_some() && options.admin_subject.is_some() {
            return Err(invalid(
                "options",
                "Choose --owner-user-id or --admin-subject, not both.",
            ));
        }
        if options
            .owner_user_id
            .is_some_and(|id| !game_data::types::valid_id(id))
        {
            return Err(invalid(
                "owner_user_id",
                "Use the immutable user id from list-users.",
            ));
        }
        let subject = placeholder::normalize(options.admin_subject.unwrap_or("admin"))
            .map_err(|e| invalid("admin_subject", &e.to_string()))?;
        let catalog = Catalog::from_json(BUNDLED_FIXTURES).map_err(|e| invalid("fixtures", &e))?;
        let reference = ReferenceFixtures::load()?;
        let worlds = WorldFixtures::load(&reference)?;
        let chart_fixtures = ChartFixtures::load()?;
        let stat_fixtures = StatFixtures::load()?;
        let authoring = Authoring::new(self.accounts.clone());
        let types = Types::new(self.accounts.clone());
        let mut tx = store::begin_write(self.accounts.pool())
            .await
            .map_err(storage_error)?;
        let existing_owner = worlds.existing_owner(&mut tx).await?;
        let mut bootstrap_subject = None;
        let owner_user_id = if let Some(id) = options.owner_user_id {
            id.to_owned()
        } else if let Some(id) = existing_owner {
            id
        } else if !store::users_exist(&mut tx).await.map_err(storage_error)? {
            let id = Accounts::bootstrap_fixture_admin(&mut tx, &subject, options.dev_auth)
                .await
                .map_err(|e| invalid("admin_subject", &e))?;
            bootstrap_subject = Some(subject);
            id
        } else {
            let admins = store::enabled_administrator_ids(&mut tx)
                .await
                .map_err(storage_error)?;
            if admins.len() != 1 {
                return Err(invalid(
                    "owner_user_id",
                    "There is not exactly one enabled administrator. Choose an existing enabled editor or administrator with --owner-user-id from list-users.",
                ));
            }
            admins.into_iter().next().ok_or(Error::Unavailable)?
        };
        let catalog_report = authoring.seed_catalog_in(&mut tx, &catalog).await?;
        let mut fixtures = types
            .seed_world_fixtures_in(&mut tx, &reference, &worlds, &owner_user_id)
            .await?;
        fixtures.species_created += catalog_report.species_created;
        fixtures.forms_created += catalog_report.forms_created;
        fixtures.skipped += catalog_report.species_skipped * 2;
        chart_fixtures
            .seed_in(&mut tx, &Charts::new(self.accounts.clone()), &mut fixtures)
            .await?;
        stat_fixtures
            .seed_in(&mut tx, &Stats::new(self.accounts.clone()), &mut fixtures)
            .await?;
        tx.commit().await.map_err(storage_error)?;
        Ok(SetupReport {
            fixtures,
            owner_user_id,
            bootstrap_subject,
        })
    }
}
