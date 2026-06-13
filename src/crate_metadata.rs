use color_eyre::eyre::Context;
use semver::{Version, VersionReq};
use serde::Deserialize;
use toml::Table;
use ureq::Agent;

#[derive(Deserialize, Debug)]
pub struct CargoToml {
    pub workspace: Option<Workspace>,
    pub package: Option<Package>,
    pub dependencies: Option<Dependencies>,
}

type Dependencies = Table;

#[derive(Deserialize, Debug)]
pub struct Workspace {
    pub resolver: Option<String>,
    pub package: Option<Package>,
    pub dependencies: Option<Dependencies>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Package {
    pub edition: Option<String>,
    // rust_version: Option<String>, // TODO:
}

#[derive(Deserialize, Debug)]
struct RegistryVersionsResponse {
    // TODO: stop allocing the entire vec.. just parse out the latest version, nothign more...
    // maybe even get rid of this serde struct thingy, just rawdog it?
    versions: Vec<RegistryCrateVersion>,
}

#[derive(Deserialize, Debug)]
struct RegistryCrateVersion {
    num: String,
}

// TODO: do the requests concurrently
// it's embarrasingly parallell....
// maybe switch to reqwest and have a tokio runtime..
pub fn look_for_outdated_dependencies(
    dependencies: Dependencies,
    num_outdated_dependencies: &mut u16,
    agent: &Agent,
) -> color_eyre::Result<()> {
    println!("\nlooking for outdated dependencies");

    for (crate_name, value) in dependencies {
        let version_str = match &value {
            toml::Value::Table(table) if let Some(version_entry) = table.get("version") => {
                match version_entry {
                    toml::Value::String(version_str) => version_str,
                    _ => unreachable!("we are fucked"),
                }
            }
            toml::Value::String(version_str) => version_str,
            _ => continue,
        };
        let version_req = VersionReq::parse(version_str)?;

        let versions_response: RegistryVersionsResponse = agent
            .get(format!(
                "https://crates.io/api/v1/crates/{}/versions",
                crate_name
            ))
            .call()?
            .body_mut()
            .read_json()?;

        let latest_version: Version = versions_response.versions[0].num.parse()?;

        if !version_req.matches(&latest_version) {
            println!(
                "- {}: using {} but latest is {}",
                crate_name, version_req, latest_version
            );
            *num_outdated_dependencies += 1;
        }
    }

    Ok(())
}

pub fn is_old_edition(edition_str: &str) -> color_eyre::Result<bool> {
    Ok(edition_str
        .parse::<u16>()
        .wrap_err("edition wasn't possible to parse as a year")?
        < 2024)
}

pub fn fetch_cargo_toml(github_project: &str, agent: &Agent) -> color_eyre::Result<CargoToml> {
    let cargo_toml_str = agent
        .get(format!(
            "https://raw.githubusercontent.com/{}/HEAD/Cargo.toml",
            github_project
        ))
        .call()?
        .body_mut()
        .read_to_string()?;

    toml::from_str(&cargo_toml_str).map_err(color_eyre::Report::from)
}
