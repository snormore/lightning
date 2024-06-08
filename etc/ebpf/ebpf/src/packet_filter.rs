use core::mem;

use aya_ebpf::bindings::xdp_action;
use aya_ebpf::macros::xdp;
use aya_ebpf::maps::lpm_trie::Key;
use aya_ebpf::programs::XdpContext;
use lightning_ebpf_common::{PacketFilter, PacketFilterParams, SubnetFilterParams};
use memoffset::offset_of;
use network_types::eth::{EthHdr, EtherType};
use network_types::ip::{IpProto, Ipv4Hdr};
use network_types::tcp::TcpHdr;
use network_types::udp::UdpHdr;

use crate::maps;

type XdpAction = xdp_action::Type;

#[xdp]
pub fn xdp_packet_filter(ctx: XdpContext) -> u32 {
    match unsafe { filter(ctx) } {
        Ok(ret) => ret,
        Err(_) => xdp_action::XDP_PASS,
    }
}

unsafe fn filter(ctx: XdpContext) -> Result<u32, ()> {
    let h_proto = unsafe { *ptr_at::<EtherType>(&ctx, offset_of!(EthHdr, ether_type))? };
    match h_proto {
        EtherType::Ipv4 => process_ipv4(&ctx),
        _ => Ok(xdp_action::XDP_PASS),
    }
}

fn process_ipv4(ctx: &XdpContext) -> Result<XdpAction, ()> {
    let ip =
        u32::from_be_bytes(unsafe { *ptr_at(ctx, EthHdr::LEN + offset_of!(Ipv4Hdr, src_addr))? });

    if let Some(params) = try_match_only_ip(ip) {
        return Ok(params.action);
    }

    let proto = unsafe { *ptr_at::<IpProto>(ctx, EthHdr::LEN + offset_of!(Ipv4Hdr, proto))? };
    let port = match proto {
        IpProto::Tcp => u16::from_be_bytes(unsafe {
            *ptr_at(ctx, EthHdr::LEN + Ipv4Hdr::LEN + offset_of!(TcpHdr, dest))?
        }),
        IpProto::Udp => u16::from_be_bytes(unsafe {
            *ptr_at(ctx, EthHdr::LEN + Ipv4Hdr::LEN + offset_of!(UdpHdr, dest))?
        }),
        _ => {
            return Ok(xdp_action::XDP_PASS);
        },
    };

    if let Some(params) = try_match(PacketFilter {
        ip,
        port,
        proto: proto as u16,
    }) {
        return Ok(params.action);
    }

    if let Some(params) = try_match_subnet(ip, port, proto as u16) {
        return Ok(params.extra.action);
    }

    Ok(xdp_action::XDP_PASS)
}

// Before any data access, the verifier requires us to do a bound check.
fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    Ok((start + offset) as *const T)
}

fn try_match(filter: PacketFilter) -> Option<PacketFilterParams> {
    unsafe {
        // Try a specific match.
        let mut result = maps::PACKET_FILTERS.get(&filter).copied();

        // Try for any port.
        if result.is_none() {
            result = maps::PACKET_FILTERS
                .get(&PacketFilter {
                    ip: filter.ip,
                    port: 0,
                    proto: filter.proto,
                })
                .copied();
        }

        // Try for any protocol.
        if result.is_none() {
            result = maps::PACKET_FILTERS
                .get(&PacketFilter {
                    ip: filter.ip,
                    port: filter.port,
                    proto: u16::MAX,
                })
                .copied()
        }

        result
    }
}

fn try_match_only_ip(ip: u32) -> Option<PacketFilterParams> {
    unsafe {
        maps::PACKET_FILTERS
            .get(&PacketFilter {
                ip,
                port: 0,
                proto: u16::MAX,
            })
            .copied()
    }
}

fn try_match_subnet(ip: u32, port: u16, proto: u16) -> Option<SubnetFilterParams> {
    let subnet_filter = maps::SUBNET_FILTER
        .get(&Key {
            prefix_len: 32,
            data: ip,
        })
        .copied()?;

    if (subnet_filter.port == port
        && (subnet_filter.proto == proto || subnet_filter.proto == u16::MAX))
        || (subnet_filter.proto == proto && subnet_filter.port == 0)
    {
        Some(subnet_filter)
    } else {
        None
    }
}
