use std::net::SocketAddrV4;

use anyhow::Context;
use aya::maps::HashMap;
use aya::programs::{Xdp, XdpFlags};
use aya::{include_bytes_aligned, Ebpf};
use aya_log::EbpfLogger;
use clap::Parser;
use lightning_ebpf_common::PacketFilter;
use tokio::signal;

#[derive(Debug, Parser)]
struct Opts {
    /// Interface to attach xdp program to.
    #[clap(short, long, default_value = "eth0")]
    iface: String,
    /// Ip and port to block.
    #[clap(short, long)]
    block: Option<SocketAddrV4>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let opt = Opts::parse();

    env_logger::init();

    #[cfg(debug_assertions)]
    let mut handle = Ebpf::load(include_bytes_aligned!(
        "../../../ebpf/target/bpfel-unknown-none/debug/ebpf"
    ))?;
    #[cfg(not(debug_assertions))]
    let mut handle = Ebpf::load(include_bytes_aligned!(
        "../../../ebpf/target/bpfel-unknown-none/release/ebpf"
    ))?;

    if let Err(e) = EbpfLogger::init(&mut handle) {
        log::warn!("failed to initialize logger: {}", e);
    }

    let program: &mut Xdp = handle
        .program_mut("xdp_packet_filter")
        .unwrap()
        .try_into()?;
    program.load()?;
    program
        .attach(&opt.iface, XdpFlags::default())
        .context("failed to attach the XDP program")?;

    let mut packet_filters: HashMap<_, PacketFilter, u32> =
        HashMap::try_from(handle.map_mut("PACKET_FILTERS").unwrap())?;

    if let Some(address) = opt.block {
        let ip: u32 = (*address.ip()).into();
        let port = address.port() as u32;
        packet_filters.insert(
            PacketFilter {
                ip,
                port: port as u16,
                proto: u16::MAX,
            },
            0,
            0,
        )?;
    }

    log::info!("Enter Ctrl-C to shutdown");
    signal::ctrl_c().await?;

    Ok(())
}
