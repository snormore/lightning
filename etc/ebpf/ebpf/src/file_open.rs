use aya_bpf::cty::c_long;
use aya_bpf::macros::lsm;
use aya_bpf::programs::LsmContext;
use aya_log_ebpf::info;
use common::File;

use crate::{maps, vmlinux};

#[lsm(hook = "file_open")]
pub fn file_open(ctx: LsmContext) -> i32 {
    unsafe { try_file_open(ctx).unwrap_or_else(|_| 0) }
}

unsafe fn try_file_open(ctx: LsmContext) -> Result<i32, c_long> {
    let kfile: *const vmlinux::file = ctx.arg(0);
    let inode = aya_bpf::helpers::bpf_probe_read_kernel(&(*kfile).f_inode)?;
    let inode_n = aya_bpf::helpers::bpf_probe_read_kernel(&(*inode).i_ino)?;
    // Todo: Get device ID.
    let file = File {
        inode: inode_n,
        dev: 0,
    };

    verify_permission(&ctx, &file)
}

unsafe fn verify_permission(ctx: &LsmContext, file: &File) -> Result<i32, c_long> {
    let binfile = get_current_process_binfile()?;
    let pid = aya_bpf::helpers::bpf_get_current_pid_tgid();
    info!(
        ctx,
        "Process {} running bin {} attempting to open file", pid, binfile.inode
    );

    if let Some(rule_list) = maps::FILE_RULES.get(&binfile) {
        if binfile.dev == file.dev {
            if let Some(rule) = rule_list.rules.iter().find(|rule| rule.inode == file.inode) {
                return Ok(rule.allow);
            }
        }
    }

    // Todo: Send event about access that was not accounted for.
    Ok(0)
}

// Todo: these accesses are not CO-RE compatible.
unsafe fn get_current_process_binfile() -> Result<File, c_long> {
    let task = aya_bpf::helpers::bpf_get_current_task() as *mut vmlinux::task_struct;
    let mm = aya_bpf::helpers::bpf_probe_read_kernel(&(*task).mm)?;
    let file = aya_bpf::helpers::bpf_probe_read_kernel(&(*mm).__bindgen_anon_1.exe_file)?;
    let f_inode = aya_bpf::helpers::bpf_probe_read_kernel(&(*file).f_inode)?;
    // Get the inode number.
    let inode_n = aya_bpf::helpers::bpf_probe_read_kernel(&(*f_inode).i_ino)?;
    // Get the device ID from the SuperBlock obj.
    let super_block = aya_bpf::helpers::bpf_probe_read_kernel(&(*f_inode).i_sb)?;
    let dev = aya_bpf::helpers::bpf_probe_read_kernel(&(*super_block).s_dev)?;
    Ok(File {
        inode: inode_n,
        dev,
    })
}