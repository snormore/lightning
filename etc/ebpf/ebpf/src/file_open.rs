use aya_ebpf::cty::c_long;
use aya_ebpf::macros::lsm;
use aya_ebpf::programs::LsmContext;
use aya_log_ebpf::info;
use lightning_ebpf_common::{File, FileRule};

use crate::{access, maps, vmlinux};

pub const ALLOW: i32 = 0;
pub const DENY: i32 = -1;

#[lsm(hook = "file_open")]
pub fn file_open(ctx: LsmContext) -> i32 {
    unsafe { try_file_open(ctx).unwrap_or_else(|_| 0) }
}

unsafe fn try_file_open(ctx: LsmContext) -> Result<i32, c_long> {
    let ctx_file: *const vmlinux::file = ctx.arg(0);
    let inode = aya_ebpf::helpers::bpf_probe_read_kernel(access::file_inode(ctx_file))?;
    let inode_n = aya_ebpf::helpers::bpf_probe_read_kernel(access::inode_i_ino(inode))?;
    verify_permission(&ctx, inode_n)
}

unsafe fn verify_permission(ctx: &LsmContext, target_inode: u64) -> Result<i32, c_long> {
    let task_inode = get_inode_from_current_task()?;
    if let Some(rule_list) = maps::FILE_RULES.get(&File::new(task_inode)) {
        info!(
            ctx,
            "file_open attempt on {} by {}", target_inode, task_inode
        );

        // Todo: let's put this log behind a flag as it's for debugging.
        let pid = aya_ebpf::helpers::bpf_get_current_pid_tgid();
        info!(
            ctx,
            "Process {} running bin {} attempting to open file", pid, task_inode
        );

        if rule_list
            .rules
            .iter()
            .find(|rule| rule.inode == target_inode)
            .map(|rule| rule.permissions & FileRule::OPEN_MASK > 0)
            .unwrap_or(false)
        {
            return Ok(ALLOW);
        } else {
            // Todo: Send event about access that was not accounted for.
            return Ok(DENY);
        }
    }

    Ok(ALLOW)
}

/// Get the inode number of the current process's binary file.
unsafe fn get_inode_from_current_task() -> Result<u64, c_long> {
    let task = aya_ebpf::helpers::bpf_get_current_task() as *mut vmlinux::task_struct;
    let mm = aya_ebpf::helpers::bpf_probe_read_kernel(access::task_struct_mm(task))?;
    let file = aya_ebpf::helpers::bpf_probe_read_kernel(access::mm_exe_file(mm))?;
    let f_inode = aya_ebpf::helpers::bpf_probe_read_kernel(access::file_inode(file))?;
    aya_ebpf::helpers::bpf_probe_read_kernel(access::inode_i_ino(f_inode))
}
