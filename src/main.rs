use clap::Parser;
use color_eyre::eyre::{Context, OptionExt};
use semver::{Version, VersionReq};
use serde::Deserialize;
use toml::Table;
use ureq::Agent;

#[derive(Parser, Debug)]
#[command(version, about, arg_required_else_help = true)]
struct Args {
    /// either <USER>/<REPO> or full URL
    github_project_or_url: String,
}

#[derive(Deserialize, Debug)]
struct CargoToml {
    workspace: Option<Workspace>,
    package: Option<Package>,
    dependencies: Option<Dependencies>,
}

type Dependencies = Table;

#[derive(Deserialize, Debug)]
struct Workspace {
    resolver: Option<String>,
    package: Option<Package>,
    dependencies: Option<Dependencies>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
struct Package {
    edition: Option<String>,
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

pub fn is_old_edition(edition_str: &str) -> color_eyre::Result<bool> {
    Ok(edition_str
        .parse::<u16>()
        .wrap_err("edition wasn't possible to parse as a year")?
        < 2024)
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

pub fn parse_github_project(github_project_or_url: &str) -> color_eyre::Result<&str> {
    if !github_project_or_url.starts_with("http") {
        // already what we want! hopefully..
        return Ok(github_project_or_url);
    }

    let (_, rest) = github_project_or_url
        .split_once("github.com/")
        .ok_or_eyre("not a GitHub URL!")?;

    let end_index = rest
        .match_indices('/')
        .nth(1)
        .map_or(rest.len(), |(i, _)| i);
    Ok(&rest[..end_index])
}

fn main() -> color_eyre::Result<()> {
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .display_location_section(cfg!(debug_assertions))
        .install()?;

    let args = Args::parse();

    let github_project = parse_github_project(&args.github_project_or_url)?;

    println!("checking 'https://github.com/{}'", github_project);

    let cargo_toml_raw_url = format!(
        "https://raw.githubusercontent.com/{}/HEAD/Cargo.toml",
        github_project
    );

    let agent: Agent = Agent::config_builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .into();

    let cargo_toml_str = agent
        .get(cargo_toml_raw_url)
        .call()?
        .body_mut()
        .read_to_string()?;

    let cargo_toml: CargoToml = toml::from_str(&cargo_toml_str)?;

    let mut slop_score_motivations = Vec::new();
    let mut num_outdated_dependencies = 0;

    if let Some(package) = cargo_toml.package
        && let Some(edition) = package.edition
        && is_old_edition(&edition)?
    {
        slop_score_motivations.push(format!("using old Rust edition ({})", edition));
    }

    if let Some(dependencies) = cargo_toml.dependencies {
        look_for_outdated_dependencies(dependencies, &mut num_outdated_dependencies, &agent)?;
    }

    if let Some(workspace) = cargo_toml.workspace {
        if let Some(package) = workspace.package
            && let Some(edition) = package.edition
            && is_old_edition(&edition)?
        {
            slop_score_motivations.push(format!("using old Rust edition ({})", edition));
        }

        if let Some(resolver) = workspace.resolver
            && resolver.parse::<u8>().unwrap() < 3
        {
            slop_score_motivations.push(format!("using old workspace resolver ({})", resolver));
        }

        if let Some(dependencies) = workspace.dependencies {
            look_for_outdated_dependencies(dependencies, &mut num_outdated_dependencies, &agent)?;
        }
    }

    let slop_score = num_outdated_dependencies
        + u16::try_from(slop_score_motivations.len())
            .wrap_err("THE AMOUNT OF SLOP IS OVERWHELMING!!!")?;

    println!("\nslop score: {}", slop_score);

    for motivation in slop_score_motivations {
        println!("- {}", motivation);
    }
    if num_outdated_dependencies > 0 {
        println!(
            "- using {} outdated dependencies",
            num_outdated_dependencies
        );
    }

    Ok(())
}
