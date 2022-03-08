use std::ffi::CString;
use std::io::Error as IOError;
use std::os::raw::{c_int};
use std::ptr::NonNull;

use error::IbvContextError;
pub mod error;
mod ffi;
type IbvDeviceAttr = ffi::ibv_device_attr;
#[derive(Clone)]
pub struct IbvContext {
    ibv_context: NonNull<ffi::ibv_context>,
}

impl IbvContext {
    pub fn new(dev_name: Option<&str>) -> Result<IbvContext, IbvContextError> {
        let mut num_devs: c_int = 0;
        let dev_list_ptr = unsafe { ffi::ibv_get_device_list(&mut num_devs) };
        // if there isn't any IB device in host
        debug_assert_ne!(num_devs, 0, "found {} device(s)", num_devs);
        if num_devs == 0 {
            return Err(IbvContextError::NoDevice);
        }
        let ib_dev = match dev_name {
            None => unsafe { *dev_list_ptr },
            Some(dev_name) => {
                let dev_name_cstr = CString::new(dev_name).unwrap();
                let dev_list =
                    unsafe { std::slice::from_raw_parts(dev_list_ptr, num_devs as usize) };
                let mut tmp_dev = std::ptr::null_mut::<ffi::ibv_device>();
                for i in 0..(num_devs as usize) {
                    unsafe {
                        if libc::strcmp(
                            ffi::ibv_get_device_name(dev_list[i]),
                            dev_name_cstr.as_ptr(),
                        ) == 0
                        {
                            tmp_dev = dev_list[i];
                            break;
                        }
                    }
                }
                tmp_dev
            }
        };
        // get device handle
        let ibv_context = unsafe { ffi::ibv_open_device(ib_dev) };
        if ibv_context.is_null() {
            unsafe { ffi::ibv_free_device_list(dev_list_ptr) };
            return Err(IbvContextError::OpenDeviceError);
        }
        // free the device list
        unsafe { ffi::ibv_free_device_list(dev_list_ptr) };
        Ok(Self {
            ibv_context: NonNull::new(ibv_context).unwrap(),
        })
    }
    pub fn query_device(&self) -> Result<IbvDeviceAttr, IOError> {
        let mut device_attr = unsafe { std::mem::zeroed::<IbvDeviceAttr>() };
        let ret = unsafe { ffi::ibv_query_device(self.ibv_context.as_ptr(), &mut device_attr) };
        if ret == -1 {
            return Err(IOError::last_os_error());
        }
        Ok(device_attr)
    }
    pub fn get_lid(&self, port_num: u8) -> Result<u16, IOError> {
        let mut port_attr = unsafe { std::mem::zeroed::<ffi::ibv_port_attr>() };
        let ret = unsafe {
            ffi::ibv_query_port(
                self.ibv_context.as_ptr(),
                port_num,
                &mut port_attr as *mut _ as *mut ffi::_compat_ibv_port_attr,
            )
        };
        if ret == -1 {
            return Err(IOError::last_os_error());
        }
        Ok(port_attr.lid)
    }
}

impl Drop for IbvContext {
    fn drop(&mut self) {
        let ret = unsafe { ffi::ibv_close_device(self.ibv_context.as_ptr()) };
        if ret == -1 {
            panic!("ibv_close_device(). errno: {}", IOError::last_os_error());
        }
    }
}
unsafe impl Send for IbvContext {}
unsafe impl Sync for IbvContext {}

impl IbvDeviceAttr {
    #[inline(always)]
    pub fn get_max_qp(&self) -> i32 {
        self.max_qp
    }
    #[inline(always)]
    pub fn get_max_qp_wr(&self) -> i32 {
        self.max_qp_wr
    }
    #[inline(always)]
    pub fn get_device_cap_flags(&self) -> u32 {
        self.device_cap_flags
    }
    #[inline(always)]
    pub fn get_max_sge(&self) -> i32 {
        self.max_sge
    }
    #[inline(always)]
    pub fn get_max_sge_rd(&self) -> i32 {
        self.max_sge_rd
    }
    #[inline(always)]
    pub fn get_max_cq(&self) -> i32 {
        self.max_cq
    }
    #[inline(always)]
    pub fn get_max_cqe(&self) -> i32 {
        self.max_cqe
    }
    #[inline(always)]
    pub fn get_max_mr(&self) -> i32 {
        self.max_mr
    }
    #[inline(always)]
    pub fn get_max_pd(&self) -> i32 {
        self.max_pd
    }
    #[inline(always)]
    pub fn get_max_qp_rd_atom(&self) -> i32 {
        self.max_qp_rd_atom
    }
    #[inline(always)]
    pub fn get_max_ee_rd_atom(&self) -> i32 {
        self.max_ee_rd_atom
    }
    #[inline(always)]
    pub fn get_max_res_rd_atom(&self) -> i32 {
        self.max_res_rd_atom
    }
    #[inline(always)]
    pub fn get_max_qp_init_rd_atom(&self) -> i32 {
        self.max_qp_init_rd_atom
    }
    #[inline(always)]
    pub fn get_max_ee_init_rd_atom(&self) -> i32 {
        self.max_ee_init_rd_atom
    }
    #[inline(always)]
    pub fn get_atomic_cap(&self) -> u32 {
        self.atomic_cap
    }
    #[inline(always)]
    pub fn get_max_ee(&self) -> i32 {
        self.max_ee
    }
    #[inline(always)]
    pub fn get_max_rdd(&self) -> i32 {
        self.max_rdd
    }
    #[inline(always)]
    pub fn get_max_mw(&self) -> i32 {
        self.max_mw
    }
    #[inline(always)]
    pub fn get_max_raw_ipv6_pq(&self) -> i32 {
        self.max_raw_ipv6_qp
    }
    #[inline(always)]
    pub fn get_max_raw_ethy_qp(&self) -> i32 {
        self.max_raw_ethy_qp
    }
    #[inline(always)]
    pub fn get_max_mcast_grp(&self) -> i32 {
        self.max_mcast_grp
    }
    #[inline(always)]
    pub fn get_max_mcast_qp_attach(&self) -> i32 {
        self.max_mcast_qp_attach
    }
    #[inline(always)]
    pub fn get_max_total_mcast_qp_attach(&self) -> i32 {
        self.max_total_mcast_qp_attach
    }
    #[inline(always)]
    pub fn get_max_ah(&self) -> i32 {
        self.max_ah
    }
    #[inline(always)]
    pub fn get_max_fmr(&self) -> i32 {
        self.max_fmr
    }
    #[inline(always)]
    pub fn get_max_map_per_fmr(&self) -> i32 {
        self.max_map_per_fmr
    }
    #[inline(always)]
    pub fn get_max_srq(&self) -> i32 {
        self.max_srq
    }
    #[inline(always)]
    pub fn get_max_srq_wr(&self) -> i32 {
        self.max_srq_wr
    }
    #[inline(always)]
    pub fn get_max_srq_sge(&self) -> i32 {
        self.max_srq_sge
    }
    #[inline(always)]
    pub fn get_max_pkeys(&self) -> u16 {
        self.max_pkeys
    }
    #[inline(always)]
    pub fn get_local_ca_ack_delay(&self) -> u8 {
        self.local_ca_ack_delay
    }
    #[inline(always)]
    pub fn get_phys_port_cnt(&self) -> u8 {
        self.phys_port_cnt
    }
}
pub mod ibv_device_cap_flags {
    pub const IBV_DEVICE_RESIZE_MAX_WR: u32 = 1;
    pub const IBV_DEVICE_BAD_PKEY_CNTR: u32 = 2;
    pub const IBV_DEVICE_BAD_QKEY_CNTR: u32 = 4;
    pub const IBV_DEVICE_RAW_MULTI: u32 = 8;
    pub const IBV_DEVICE_AUTO_PATH_MIG: u32 = 16;
    pub const IBV_DEVICE_CHANGE_PHY_PORT: u32 = 32;
    pub const IBV_DEVICE_UD_AV_PORT_ENFORCE: u32 = 64;
    pub const IBV_DEVICE_CURR_QP_STATE_MOD: u32 = 128;
    pub const IBV_DEVICE_SHUTDOWN_PORT: u32 = 256;
    pub const IBV_DEVICE_INIT_TYPE: u32 = 512;
    pub const IBV_DEVICE_PORT_ACTIVE_EVENT: u32 = 1024;
    pub const IBV_DEVICE_SYS_IMAGE_GUID: u32 = 2048;
    pub const IBV_DEVICE_RC_RNR_NAK_GEN: u32 = 4096;
    pub const IBV_DEVICE_SRQ_RESIZE: u32 = 8192;
    pub const IBV_DEVICE_N_NOTIFY_CQ: u32 = 16384;
    pub const IBV_DEVICE_MEM_WINDOW: u32 = 131072;
    pub const IBV_DEVICE_UD_IP_CSUM: u32 = 262144;
    pub const IBV_DEVICE_XRC: u32 = 1048576;
    pub const IBV_DEVICE_MEM_MGT_EXTENSIONS: u32 = 2097152;
    pub const IBV_DEVICE_MEM_WINDOW_TYPE_2A: u32 = 8388608;
    pub const IBV_DEVICE_MEM_WINDOW_TYPE_2B: u32 = 16777216;
    pub const IBV_DEVICE_RC_IP_CSUM: u32 = 33554432;
    pub const IBV_DEVICE_RAW_IP_CSUM: u32 = 67108864;
    pub const IBV_DEVICE_MANAGED_FLOW_STEERING: u32 = 536870912;
}
pub mod ibv_atomic_cap {
    pub const IBV_ATOMIC_NONE: u32 = 0;
    pub const IBV_ATOMIC_HCA: u32 = 1;
    pub const IBV_ATOMIC_GLOB: u32 = 2;
}
