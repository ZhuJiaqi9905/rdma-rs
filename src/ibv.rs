use std::ffi::{CStr, CString};
use std::io::Error as IOError;
use std::os::raw::c_int;
use std::ptr::NonNull;
use std::slice;

use libc::c_void;
use num_enum::TryFromPrimitive;

use crate::error::IbvContextError;
use crate::ffi;

pub type IbvDeviceAttr = ffi::ibv_device_attr;
pub type IbvPortAttr = ffi::ibv_port_attr;
pub type IbvGid = ffi::ibv_gid;
pub type IbvWc = ffi::ibv_wc;
#[derive(Clone)]
pub struct IbvContext {
    ibv_context: NonNull<ffi::ibv_context>,
}

impl IbvContext {
    pub fn new(dev_name: Option<&str>) -> Result<Self, IbvContextError> {
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
        unsafe {
            Ok(Self {
                ibv_context: NonNull::new_unchecked(ibv_context),
            })
        }
    }
    pub fn query_device(&self) -> Result<IbvDeviceAttr, IOError> {
        let mut device_attr = unsafe { std::mem::zeroed::<IbvDeviceAttr>() };
        let ret = unsafe { ffi::ibv_query_device(self.ibv_context.as_ptr(), &mut device_attr) };
        if ret != 0 {
            return Err(IOError::last_os_error());
        }
        Ok(device_attr)
    }
    // pub fn get_lid(&self, port_num: u8) -> Result<u16, IOError> {
    //     let mut port_attr = unsafe { std::mem::zeroed::<IbvPortAttr>() };
    //     let ret = unsafe {
    //         ffi::ibv_query_port(
    //             self.ibv_context.as_ptr(),
    //             port_num,
    //             &mut port_attr as *mut _ as *mut ffi::_compat_ibv_port_attr,
    //         )
    //     };
    //     if ret == -1 {
    //         return Err(IOError::last_os_error());
    //     }
    //     Ok(port_attr.lid)
    // }
    pub fn query_port(&self, port_num: u8) -> Result<IbvPortAttr, IOError> {
        let mut port_attr = unsafe { std::mem::zeroed::<IbvPortAttr>() };
        let ret = unsafe {
            ffi::ibv_query_port(
                self.ibv_context.as_ptr(),
                port_num,
                &mut port_attr as *mut _ as *mut ffi::_compat_ibv_port_attr,
            )
        };
        if ret != 0 {
            return Err(IOError::last_os_error());
        }
        Ok(port_attr)
    }
    pub fn query_gid(&self, port_num: u8, index: i32) -> Result<IbvGid, IOError> {
        let mut gid = IbvGid { raw: [0; 16] };
        let ret = unsafe {
            ffi::ibv_query_gid(
                self.ibv_context.as_ptr(),
                port_num,
                index,
                &mut gid as *mut _,
            )
        };
        if ret != 0 {
            return Err(IOError::last_os_error());
        }
        Ok(gid)
    }
    pub fn query_pkey(&self, port_num: u8, index: i32) -> Result<u16, IOError> {
        let mut pkey = 0_u16;
        let ret = unsafe {
            ffi::ibv_query_pkey(
                self.ibv_context.as_ptr(),
                port_num,
                index,
                &mut pkey as *mut _,
            )
        };
        if ret != 0 {
            return Err(IOError::last_os_error());
        }
        Ok(pkey)
    }
}

impl Drop for IbvContext {
    fn drop(&mut self) {
        let ret = unsafe { ffi::ibv_close_device(self.ibv_context.as_ptr()) };
        if ret != 0 {
            panic!("ibv_close_device(). errno: {}", IOError::last_os_error());
        }
    }
}
unsafe impl Send for IbvContext {}
unsafe impl Sync for IbvContext {}

#[derive(Clone)]
pub struct IbvPd {
    pub ibv_pd: NonNull<ffi::ibv_pd>,
}

impl IbvPd {
    pub fn new(context: &IbvContext) -> Result<Self, IOError> {
        let ibv_pd = unsafe { ffi::ibv_alloc_pd(context.ibv_context.as_ptr()) };
        if ibv_pd.is_null() {
            return Err(IOError::last_os_error());
        }
        unsafe {
            Ok(Self {
                ibv_pd: NonNull::new_unchecked(ibv_pd),
            })
        }
    }
}

impl Drop for IbvPd {
    fn drop(&mut self) {
        let ret = unsafe { ffi::ibv_dealloc_pd(self.ibv_pd.as_ptr()) };
        if ret != 0 {
            panic!("ibv_dealloc_pd(). errno: {}", IOError::last_os_error());
        }
    }
}
unsafe impl Send for IbvPd {}
unsafe impl Sync for IbvPd {}

#[derive(Clone)]
pub struct IbvCq {
    ibv_cq: NonNull<ffi::ibv_cq>,
}

impl IbvCq {
    pub fn new<T>(
        context: &IbvContext,
        cqe: i32,
        cq_context: Option<NonNull<T>>,
        channel: Option<&IbvCompChannel>,
        comp_vector: i32,
    ) -> Result<Self, IOError> {
        let cq_context = match cq_context {
            Some(p) => p.as_ptr(),
            None => std::ptr::null_mut(),
        };
        let channel = match channel {
            Some(p) => p.ibv_comp_channel.as_ptr(),
            None => std::ptr::null_mut(),
        };

        let ibv_cq = unsafe {
            ffi::ibv_create_cq(
                context.ibv_context.as_ptr(),
                cqe,
                cq_context as *mut c_void,
                channel,
                comp_vector,
            )
        };
        if ibv_cq.is_null() {
            return Err(IOError::last_os_error());
        }
        unsafe {
            Ok(Self {
                ibv_cq: NonNull::new_unchecked(ibv_cq),
            })
        }
    }

    pub fn poll<'a>(&self, cqe_arr: &'a mut [IbvWc]) -> Result<&'a [IbvWc], ()> {
        let n = unsafe {
            let ibv_poll_cq = (*(*self.ibv_cq.as_ptr()).context).ops.poll_cq.unwrap();
            ibv_poll_cq(
                self.ibv_cq.as_ptr(),
                cqe_arr.len() as i32,
                cqe_arr.as_mut_ptr(),
            )
        };
        if n < 0 {
            return Err(());
        }
        Ok(&mut cqe_arr[0..n as usize])
    }

    pub fn resize(&self, cqe: i32) -> Result<(), IOError> {
        let ret = unsafe { ffi::ibv_resize_cq(self.ibv_cq.as_ptr(), cqe) };
        if ret != 0 {
            return Err(IOError::last_os_error());
        }
        Ok(())
    }
}

impl Drop for IbvCq {
    fn drop(&mut self) {
        let ret = unsafe { ffi::ibv_destroy_cq(self.ibv_cq.as_ptr()) };
        if ret != -1 {
            panic!("ibv_destroy_cq(). errno: {}", IOError::last_os_error());
        }
    }
}

unsafe impl Send for IbvCq {}
unsafe impl Sync for IbvCq {}

#[derive(Clone)]
pub struct IbvCompChannel {
    ibv_comp_channel: NonNull<ffi::ibv_comp_channel>,
}
impl IbvCompChannel {
    pub fn new(context: &IbvContext) -> Result<Self, IOError> {
        let ibv_comp_channel =
            unsafe { ffi::ibv_create_comp_channel(context.ibv_context.as_ptr()) };
        if ibv_comp_channel.is_null() {
            return Err(IOError::last_os_error());
        }
        unsafe {
            Ok(Self {
                ibv_comp_channel: NonNull::new_unchecked(ibv_comp_channel),
            })
        }
    }
}
impl Drop for IbvCompChannel {
    fn drop(&mut self) {
        let ret = unsafe { ffi::ibv_destroy_comp_channel(self.ibv_comp_channel.as_ptr()) };
        if ret != 0 {
            panic!(
                "ibv_destroy_comp_channel(). errno: {}",
                IOError::last_os_error()
            );
        }
    }
}

#[derive(Clone)]
pub struct IbvMr {
    ibv_mr: NonNull<ffi::ibv_mr>,
}

impl IbvMr {
    pub fn new(pd: &IbvPd, region: &[u8], access: i32) -> Result<IbvMr, IOError> {
        let ibv_mr = unsafe {
            ffi::ibv_reg_mr(
                pd.ibv_pd.as_ptr(),
                region.as_ptr() as *mut c_void,
                region.len() as u64,
                access,
            )
        };
        if ibv_mr.is_null() {
            return Err(IOError::last_os_error());
        }
        unsafe {
            Ok(IbvMr {
                ibv_mr: NonNull::new_unchecked(ibv_mr),
            })
        }
    }
    pub fn new_raw(
        pd: &IbvPd,
        addr: *mut c_void,
        length: usize,
        access: i32,
    ) -> Result<IbvMr, IOError> {
        let ibv_mr = unsafe { ffi::ibv_reg_mr(pd.ibv_pd.as_ptr(), addr, length as u64, access) };
        if ibv_mr.is_null() {
            return Err(IOError::last_os_error());
        }
        unsafe {
            Ok(IbvMr {
                ibv_mr: NonNull::new_unchecked(ibv_mr),
            })
        }
    }
    #[inline(always)]
    pub fn get_rkey(&self) -> u32 {
        return unsafe { self.ibv_mr.as_ref().rkey };
    }
    #[inline(always)]
    pub fn get_lkey(&self) -> u32 {
        return unsafe { self.ibv_mr.as_ref().lkey };
    }
}

impl Drop for IbvMr {
    fn drop(&mut self) {
        let ret = unsafe { ffi::ibv_dereg_mr(self.ibv_mr.as_ptr()) };
        if ret != 0 {
            panic!("ibv_dereg_mr(). errno: {}", IOError::last_os_error());
        }
    }
}
unsafe impl Send for IbvMr {}
unsafe impl Sync for IbvMr {}

#[derive(Clone)]
pub struct IbvQp {
    ibv_qp: NonNull<ffi::ibv_qp>,
}
impl IbvQp {
    pub fn new(
        pd: &IbvPd,
        send_cq: &IbvCq,
        recv_cq: &IbvCq,
        sq_sig_all: i32,
        max_send_wr: u32,
        max_recv_wr: u32,
        max_send_sge: u32,
        max_recv_sge: u32,
        max_inline_data: u32,
    ) -> Result<Self, IOError> {
        let mut qp_init_attr = unsafe { std::mem::zeroed::<ffi::ibv_qp_init_attr>() };
        qp_init_attr.qp_type = ibv_qp_type::IBV_QPT_RC;
        qp_init_attr.sq_sig_all = sq_sig_all; // set to 0 to avoid CQE for every SR
        qp_init_attr.send_cq = send_cq.ibv_cq.as_ptr();
        qp_init_attr.recv_cq = recv_cq.ibv_cq.as_ptr();
        qp_init_attr.cap.max_send_wr = max_send_wr;
        qp_init_attr.cap.max_recv_wr = max_recv_wr;
        qp_init_attr.cap.max_send_sge = max_send_sge;
        qp_init_attr.cap.max_recv_sge = max_recv_sge;
        qp_init_attr.cap.max_inline_data = max_inline_data;
        qp_init_attr.srq = std::ptr::null_mut();
        let ibv_qp = unsafe { ffi::ibv_create_qp(pd.ibv_pd.as_ptr(), &mut qp_init_attr as *mut _) };
        if ibv_qp.is_null() {
            return Err(IOError::last_os_error());
        }
        unsafe {
            Ok(Self {
                ibv_qp: NonNull::new_unchecked(ibv_qp),
            })
        }
    }
    pub fn modify_reset2init(&self, port_num: u8) -> Result<(), IOError> {
        let mut qp_attr = unsafe { std::mem::zeroed::<ffi::ibv_qp_attr>() };
        qp_attr.qp_state = IBV_QPS_INIT;
        qp_attr.pkey_index = 0;
        qp_attr.port_num = port_num;
        qp_attr.qp_access_flags = ibv_access_flags::IBV_ACCESS_LOCAL_WRITE
            | ibv_access_flags::IBV_ACCESS_REMOTE_READ
            | ibv_access_flags::IBV_ACCESS_REMOTE_WRITE;
        let ret = unsafe {
            ffi::ibv_modify_qp(
                self.p_ibv_qp.as_ptr(),
                &mut qp_attr as *mut _,
                (ibv_qp_attr_mask::IBV_QP_STATE.0
                    | ibv_qp_attr_mask::IBV_QP_PKEY_INDEX.0
                    | ibv_qp_attr_mask::IBV_QP_PORT.0
                    | ibv_qp_attr_mask::IBV_QP_ACCESS_FLAGS.0) as i32,
            )
        };
        if ret == -1 {
            return Err(IOError::last_os_error());
        }
        Ok(())
    }
    pub fn modify_init2rtr(
        &self,

        sl: u8,
        port_num: u8,
        remote_qpn: u32,
        remote_psn: u32,
        remote_lid: u16,
    ) -> Result<(), IOError> {
        let mut qp_attr = unsafe { std::mem::zeroed::<ffi::ibv_qp_attr>() };
        qp_attr.qp_state = ibv_qp_state::IBV_QPS_RTR;
        qp_attr.path_mtu = IBV_MTU_1024;
        qp_attr.dest_qp_num = remote_qpn;
        qp_attr.rq_psn = remote_psn;
        qp_attr.max_dest_rd_atomic = 1;
        qp_attr.min_rnr_timer = 12;
        qp_attr.ah_attr.is_global = 0;
        qp_attr.ah_attr.dlid = remote_lid;
        qp_attr.ah_attr.sl = sl;
        qp_attr.ah_attr.src_path_bits = 0;
        qp_attr.ah_attr.port_num = port_num;
        let ret = unsafe {
            ffi::ibv_modify_qp(
                self.ibv_qp.as_ptr(),
                &mut qp_attr as *mut _,
                (ibv_qp_attr_mask::IBV_QP_STATE.0
                    | ibv_qp_attr_mask::IBV_QP_AV.0
                    | ibv_qp_attr_mask::IBV_QP_PATH_MTU.0
                    | ibv_qp_attr_mask::IBV_QP_DEST_QPN.0
                    | ibv_qp_attr_mask::IBV_QP_RQ_PSN.0
                    | ibv_qp_attr_mask::IBV_QP_MAX_DEST_RD_ATOMIC.0
                    | ibv_qp_attr_mask::IBV_QP_MIN_RNR_TIMER.0) as i32,
            )
        };
        if ret == -1 {
            return Err(IOError::last_os_error());
        }
        Ok(())
    }

    pub fn modify_rtr2rts(&self, psn: u32) -> Result<(), IOError> {
        let mut qp_attr = unsafe { std::mem::zeroed::<ffi::ibv_qp_attr>() };
        qp_attr.qp_state = ibv_qp_state::IBV_QPS_RTS;
        qp_attr.timeout = 14;
        qp_attr.retry_cnt = 7;
        qp_attr.rnr_retry = 7;
        qp_attr.sq_psn = psn;
        qp_attr.max_rd_atomic = 1;
        let ret = unsafe {
            ffi::ibv_modify_qp(
                self.ibv_qp.as_ptr(),
                &mut qp_attr as *mut _,
                (ibv_qp_attr_mask::IBV_QP_STATE.0
                    | ibv_qp_attr_mask::IBV_QP_TIMEOUT.0
                    | ibv_qp_attr_mask::IBV_QP_RETRY_CNT.0
                    | ibv_qp_attr_mask::IBV_QP_RNR_RETRY.0
                    | ibv_qp_attr_mask::IBV_QP_SQ_PSN.0
                    | ibv_qp_attr_mask::IBV_QP_MAX_QP_RD_ATOMIC.0) as i32,
            )
        };
        if ret == -1 {
            return Err(IOError::last_os_error());
        }
        Ok(())
    }
    #[inline(always)]
    pub fn get_qpn(&self) -> u32 {
        unsafe { self.ibv_qp.as_ref().qp_num }
    }
}
impl Drop for IbvQp {
    fn drop(&mut self) {
        let ret = unsafe { ffi::ibv_destroy_qp(self.ibv_qp.as_ptr()) };
        if ret == -1 {
            panic!("ibv_destroy_qp() error");
        }
    }
}
unsafe impl Send for IbvQp {}
unsafe impl Sync for IbvQp {}

impl IbvDeviceAttr {
    #[inline(always)]
    pub fn get_fw_ver(&self) -> &str {
        let mut i = 0;
        while i < self.fw_ver.len() {
            if self.fw_ver[i] as u8 == b'\0' {
                break;
            }
            i += 1;
        }
        let s = unsafe { slice::from_raw_parts(self.fw_ver.as_ptr() as *const u8, i + 1) };
        let cstr = CStr::from_bytes_with_nul(s).unwrap();
        cstr.to_str().unwrap()
    }
    #[inline(always)]
    pub fn get_node_guid(&self) -> u64 {
        self.node_guid
    }
    #[inline(always)]
    pub fn get_sys_image_guid(&self) -> u64 {
        self.sys_image_guid
    }
    #[inline(always)]
    pub fn get_max_mr_size(&self) -> u64 {
        self.max_mr_size
    }
    #[inline(always)]
    pub fn get_page_size_cap(&self) -> u64 {
        self.page_size_cap
    }
    #[inline(always)]
    pub fn get_vendor_id(&self) -> u32 {
        self.vendor_id
    }
    #[inline(always)]
    pub fn get_vendor_part_id(&self) -> u32 {
        self.vendor_part_id
    }
    #[inline(always)]
    pub fn get_hw_ver(&self) -> u32 {
        self.hw_ver
    }
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

impl IbvPortAttr {
    #[inline(always)]
    pub fn get_state(&self) -> IbvPortState {
        IbvPortState::try_from(self.state).unwrap()
    }
    #[inline(always)]
    pub fn get_max_mtu(&self) -> IbvMtu {
        IbvMtu::try_from(self.max_mtu).unwrap()
    }
    #[inline(always)]
    pub fn get_active_mtu(&self) -> IbvMtu {
        IbvMtu::try_from(self.active_mtu).unwrap()
    }
    #[inline(always)]
    pub fn get_gid_tbl_len(&self) -> i32 {
        self.gid_tbl_len
    }
    #[inline(always)]
    pub fn get_port_cap_flags(&self) -> u32 {
        self.port_cap_flags
    }
    #[inline(always)]
    pub fn get_max_msg_sz(&self) -> u32 {
        self.max_msg_sz
    }
    #[inline(always)]
    pub fn get_bad_pkey_cntr(&self) -> u32 {
        self.bad_pkey_cntr
    }
    #[inline(always)]
    pub fn get_qkey_viol_cntr(&self) -> u32 {
        self.qkey_viol_cntr
    }
    #[inline(always)]
    pub fn get_lid(&self) -> u16 {
        self.lid
    }
    #[inline(always)]
    pub fn get_sm_lid(&self) -> u16 {
        self.sm_lid
    }
    #[inline(always)]
    pub fn get_lmc(&self) -> u8 {
        self.lmc
    }
    #[inline(always)]
    pub fn get_max_vl_num(&self) -> u8 {
        self.max_vl_num
    }
    #[inline(always)]
    pub fn get_sm_sl(&self) -> u8 {
        self.sm_sl
    }
    #[inline(always)]
    pub fn get_subnet_timeout(&self) -> u8 {
        self.subnet_timeout
    }
    #[inline(always)]
    pub fn get_init_type_reply(&self) -> u8 {
        self.init_type_reply
    }
    #[inline(always)]
    pub fn get_active_width(&self) -> u8 {
        self.active_width
    }
    #[inline(always)]
    pub fn get_active_speed(&self) -> u8 {
        self.active_speed
    }
    #[inline(always)]
    pub fn getphys_state(&self) -> u8 {
        self.phys_state
    }
    #[inline(always)]
    pub fn get_link_layer(&self) -> u8 {
        self.link_layer
    }
    #[inline(always)]
    pub fn get_flags(&self) -> u8 {
        self.flags
    }
    #[inline(always)]
    pub fn get_port_cap_flags2(&self) -> u16 {
        self.port_cap_flags2
    }
}

#[derive(TryFromPrimitive)]
#[repr(u32)]
pub enum IbvPortState {
    IbvPortNop = 0,
    IbvPortDown = 1,
    IbvPortInit = 2,
    IbvPortArmed = 3,
    IbvPortActive = 4,
    IbvPortActiveDefer = 5,
}

#[derive(TryFromPrimitive)]
#[repr(u32)]
pub enum IbvMtu {
    IbvMtu256 = 1,
    IbvMtu512 = 2,
    IbvMtu1024 = 3,
    IbvMtu2048 = 4,
    IbvMtu4096 = 5,
}
pub mod ibv_port_cap_flags {
    pub const IBV_PORT_SM: u32 = 2;
    pub const IBV_PORT_NOTICE_SUP: u32 = 4;
    pub const IBV_PORT_TRAP_SUP: u32 = 8;
    pub const IBV_PORT_OPT_IPD_SUP: u32 = 16;
    pub const IBV_PORT_AUTO_MIGR_SUP: u32 = 32;
    pub const IBV_PORT_SL_MAP_SUP: u32 = 64;
    pub const IBV_PORT_MKEY_NVRAM: u32 = 128;
    pub const IBV_PORT_PKEY_NVRAM: u32 = 256;
    pub const IBV_PORT_LED_INFO_SUP: u32 = 512;
    pub const IBV_PORT_SYS_IMAGE_GUID_SUP: u32 = 2048;
    pub const IBV_PORT_PKEY_SW_EXT_PORT_TRAP_SUP: u32 = 4096;
    pub const IBV_PORT_EXTENDED_SPEEDS_SUP: u32 = 16384;
    pub const IBV_PORT_CAP_MASK2_SUP: u32 = 32768;
    pub const IBV_PORT_CM_SUP: u32 = 65536;
    pub const IBV_PORT_SNMP_TUNNEL_SUP: u32 = 131072;
    pub const IBV_PORT_REINIT_SUP: u32 = 262144;
    pub const IBV_PORT_DEVICE_MGMT_SUP: u32 = 524288;
    pub const IBV_PORT_VENDOR_CLASS_SUP: u32 = 1048576;
    pub const IBV_PORT_DR_NOTICE_SUP: u32 = 2097152;
    pub const IBV_PORT_CAP_MASK_NOTICE_SUP: u32 = 4194304;
    pub const IBV_PORT_BOOT_MGMT_SUP: u32 = 8388608;
    pub const IBV_PORT_LINK_LATENCY_SUP: u32 = 16777216;
    pub const IBV_PORT_CLIENT_REG_SUP: u32 = 33554432;
    pub const IBV_PORT_IP_BASED_GIDS: u32 = 67108864;
}
pub mod ibv_port_cap_flags2 {
    pub const IBV_PORT_SET_NODE_DESC_SUP: u16 = 1;
    pub const IBV_PORT_INFO_EXT_SUP: u16 = 2;
    pub const IBV_PORT_VIRT_SUP: u16 = 4;
    pub const IBV_PORT_SWITCH_PORT_STATE_TABLE_SUP: u16 = 8;
    pub const IBV_PORT_LINK_WIDTH_2X_SUP: u16 = 16;
    pub const IBV_PORT_LINK_SPEED_HDR_SUP: u16 = 32;
}
impl IbvGid {
    #[inline(always)]
    pub fn get_subnet_prefix(&self) -> u64 {
        unsafe { self.global.subnet_prefix }
    }
    #[inline(always)]
    pub fn get_interface_id(&self) -> u64 {
        unsafe { self.global.interface_id }
    }
}

pub mod ibv_access_flags {
    use crate::ffi;

    pub const IBV_ACCESS_LOCAL_WRITE: u32 = ffi::ibv_access_flags_IBV_ACCESS_LOCAL_WRITE;
    pub const IBV_ACCESS_REMOTE_WRITE: u32 = ffi::ibv_access_flags_IBV_ACCESS_REMOTE_WRITE;
    pub const IBV_ACCESS_REMOTE_READ: u32 = ffi::ibv_access_flags_IBV_ACCESS_REMOTE_READ;
    pub const IBV_ACCESS_REMOTE_ATOMIC: u32 = ffi::ibv_access_flags_IBV_ACCESS_REMOTE_ATOMIC;
    pub const IBV_ACCESS_MW_BIND: u32 = ffi::ibv_access_flags_IBV_ACCESS_MW_BIND;
    pub const IBV_ACCESS_ZERO_BASED: u32 = ffi::ibv_access_flags_IBV_ACCESS_ZERO_BASED;
    pub const IBV_ACCESS_ON_DEMAND: u32 = ffi::ibv_access_flags_IBV_ACCESS_ON_DEMAND;
    pub const IBV_ACCESS_HUGETLB: u32 = ffi::ibv_access_flags_IBV_ACCESS_HUGETLB;
    pub const IBV_ACCESS_RELAXED_ORDERING: u32 = ffi::ibv_access_flags_IBV_ACCESS_RELAXED_ORDERING;
}
pub mod ibv_qp_type {
    use crate::ffi;
    pub const IBV_QPT_RC: u32 = ffi::ibv_qp_type_IBV_QPT_RC;
    pub const IBV_QPT_UC: u32 = ffi::ibv_qp_type_IBV_QPT_UC;
    pub const IBV_QPT_UD: u32 = ffi::ibv_qp_type_IBV_QPT_UD;
    pub const IBV_QPT_RAW_PACKET: u32 = ffi::ibv_qp_type_IBV_QPT_RAW_PACKET;
    pub const IBV_QPT_XRC_SEND: u32 = ffi::ibv_qp_type_IBV_QPT_XRC_SEND;
    pub const IBV_QPT_XRC_RECV: u32 = ffi::ibv_qp_type_IBV_QPT_XRC_RECV;
    pub const IBV_QPT_DRIVER: u32 = ffi::ibv_qp_type_IBV_QPT_DRIVER;
}

pub fn ibv_fork_init() -> Result<(), IOError> {
    let ret = unsafe { ffi::ibv_fork_init() };
    if ret != 0 {
        return Err(IOError::last_os_error());
    }
    Ok(())
}
