use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use regex::Regex;
use std::process::Command;

#[derive(Parser)]
#[command(name = "ndpfix")]
#[command(version, about = "Auto-fix IPv6 relay with /128 routes and NDP proxy")] 
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Oneshot { wan: String, lan: String, ip: String },
    Scan { wan: String, lan: String },
}

fn run(cmd: &mut Command) -> Result<String> {
    let out = cmd.output().context("exec failed")?;
    if out.status.success() { 
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(anyhow!(String::from_utf8_lossy(&out.stderr).to_string()))
    }
}

fn enable_proxy_ndp(wan: &str) -> Result<()> {
    run(Command::new("sysctl").args(["-w", "net.ipv6.conf.all.proxy_ndp=1"]))?;
    run(Command::new("sysctl").args(["-w", &format!("net.ipv6.conf.{wan}.proxy_ndp=1")]))?;
    Ok(())
}

fn add_host_route(ip: &str, lan: &str) -> Result<()> {
    run(Command::new("ip").args(["-6", "route", "replace", &format!("{ip}/128"), "dev", lan]))?;
    Ok(())
}

fn add_ndp_proxy(ip: &str, wan: &str) -> Result<()> {
    run(Command::new("ip").args(["-6", "neigh", "replace", "proxy", ip, "dev", wan]))?;
    Ok(())
}

fn is_global_ipv6(ip: &str) -> bool {
    !(ip.starts_with("fe80:") || ip.starts_with("fc") || ip.starts_with("fd"))
}

fn scan_neighbors(lan: &str) -> Result<Vec<String>> {
    let out = run(Command::new("ip").args(["-6", "neigh", "show", "dev", lan]))?;
    let re = Regex::new(r"([0-9a-fA-F:]+)\s+dev\s+\S+").unwrap();
    let mut ips = Vec::new();
    for line in out.lines() {
        if let Some(c) = re.captures(line) { 
            let ip = c.get(1).unwrap().as_str().to_string();
            if is_global_ipv6(&ip) { ips.push(ip); }
        }
    }
    Ok(ips)
}

fn apply_for_ip(wan: &str, lan: &str, ip: &str) -> Result<()> {
    enable_proxy_ndp(wan)?;
    add_host_route(ip, lan)?;
    add_ndp_proxy(ip, wan)?;
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Oneshot { wan, lan, ip } => {
            apply_for_ip(&wan, &lan, &ip)?;
        }
        Cmd::Scan { wan, lan } => {
            let ips = scan_neighbors(&lan)?;
            for ip in ips { let _ = apply_for_ip(&wan, &lan, &ip); }
        }
    }
    Ok(())
}
