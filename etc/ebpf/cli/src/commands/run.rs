use std::process::Command;

use crate::cli::Target;
use crate::commands::build;

fn build_userspace_application(release: bool) -> anyhow::Result<()> {
    let mut args = vec!["build"];

    if release {
        args.push("--release")
    }

    let status = Command::new("cargo").args(&args).status()?;

    if !status.success() {
        anyhow::bail!("failed to build eBFP program");
    }

    Ok(())
}

pub fn run(target: Target, release: bool, xdp_args: Vec<String>) -> Result<(), anyhow::Error> {
    build::build_bpf_program(target, release)?;
    build_userspace_application(release)?;

    let mode = if release { "release" } else { "debug" };
    let bin_path = format!("target/{mode}/xdp-simple");

    let mut xdp_args = xdp_args.iter().map(String::as_str).collect();

    let mut args: Vec<_> = vec!["sudo", "-E"];
    args.push(bin_path.as_str());
    args.append(&mut xdp_args);

    let status = Command::new(args.first().expect("args are hardcoded"))
        .args(args.iter().skip(1))
        .status()?;

    if !status.success() {
        anyhow::bail!("failed to run `{}`", args.join(" "));
    }

    Ok(())
}