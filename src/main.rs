use clap::Parser;
use color_eyre::eyre::{Context, OptionExt};
use ureq::Agent;

mod crate_metadata;

use crate::crate_metadata::{fetch_cargo_toml, is_old_edition, look_for_outdated_dependencies};

#[derive(Parser, Debug)]
#[command(version, about, arg_required_else_help = true)]
struct Args {
    /// either <USER>/<REPO> or full URL
    github_project_or_url: String,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .display_location_section(cfg!(debug_assertions))
        .install()?;

    let args = Args::parse();

    let github_project = parse_github_project(&args.github_project_or_url)?;

    println!("checking 'https://github.com/{}'", github_project);

    let agent: Agent = Agent::config_builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .into();

    let cargo_toml = fetch_cargo_toml(github_project, &agent)?;

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
