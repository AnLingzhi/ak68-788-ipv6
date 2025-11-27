use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, Args};
use clap::error::ErrorKind;
use regex::Regex;
use std::process::Command;

#[derive(Parser)]
#[command(name = "ndpfix")]
#[command(version, about = "Auto-fix IPv6 relay with /128 routes and NDP proxy", arg_required_else_help = true)] 
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
    #[arg(short, long, global = true, default_value_t = false)]
    verbose: bool,
}

#[derive(Args, Clone)]
struct Common {
    #[arg(short = 'w', long = "wan")]
    wan: String,
    #[arg(short = 'l', long = "lan")]
    lan: String,
}

#[derive(Subcommand)]
enum Cmd {
    Oneshot { #[command(flatten)] common: Common, #[arg(short = 'i', long = "ip")] ip: String },
    Scan { #[command(flatten)] common: Common },
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

fn apply_for_ip(wan: &str, lan: &str, ip: &str, verbose: bool) -> Result<()> {
    if verbose { println!("enable proxy_ndp on {wan}"); }
    enable_proxy_ndp(wan)?;
    if verbose { println!("route {ip}/128 -> {lan}"); }
    add_host_route(ip, lan)?;
    if verbose { println!("ndp proxy {ip} on {wan}"); }
    add_ndp_proxy(ip, wan)?;
    Ok(())
}

fn fallback(args: &[String], verbose: bool) -> Result<()> {
    if args.len() >= 4 && args[1] == "scan" {
        let wan = args[2].clone();
        let lan = args[3].clone();
        let ips = scan_neighbors(&lan)?;
        for ip in ips { let _ = apply_for_ip(&wan, &lan, &ip, verbose); }
        return Ok(())
    }
    if args.len() >= 5 && args[1] == "oneshot" {
        let wan = args[2].clone();
        let lan = args[3].clone();
        let ip = args[4].clone();
        apply_for_ip(&wan, &lan, &ip, verbose)?;
        return Ok(())
    }
    Err(anyhow!("usage: ndpfix scan <wan> <lan> | ndpfix oneshot <wan> <lan> <ip>"))
}

fn main() -> Result<()> {
    let raw: Vec<String> = std::env::args().collect();
    let parsed = Cli::try_parse_from(&raw);
    match parsed {
        Ok(cli) => match cli.cmd {
            Cmd::Oneshot { common, ip } => {
                apply_for_ip(&common.wan, &common.lan, &ip, cli.verbose)?;
            }
            Cmd::Scan { common } => {
                let ips = scan_neighbors(&common.lan)?;
                for ip in ips { let _ = apply_for_ip(&common.wan, &common.lan, &ip, cli.verbose); }
            }
        },
        Err(e) => {
            match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    e.print().ok();
                    return Ok(())
                }
                _ => fallback(&raw, false)?,
            }
        }
    }
    Ok(())
}
