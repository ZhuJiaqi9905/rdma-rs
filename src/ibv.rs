use std::ffi::{CStr, CString};
use std::io::Error as IOError;
use std::os::raw::c_int;
use std::ptr::NonNull;
use std::slice;

use libc::c_void;

use crate::error::IbvContextError;
use crate::ffi;
use crate::ffi::ibv_access_flags;
pub type IbvDeviceAttr = ffi::ibv_device_attr;
pub type IbvPortAttr = ffi::ibv_port_attr;
pub type IbvGid = ffi::ibv_gid;
pub type IbvWc = ffi::ibv_wc;
pub type IbvQpInitAttr = ffi::ibv_qp_init_attr;
pub type IbvQpAttr = ffi::ibv_qp_attr;
pub type IbvRecvWr = ffi::ibv_recv_wr;
pub type IbvSendWr = ffi::ibv_send_wr;
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
            None => std::ptr::null_mut::<T>(),
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
        if ret != 0 {
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
    pub fn new(pd: &IbvPd, region: &[u8], access: ibv_access_flags) -> Result<IbvMr, IOError> {
        let ibv_mr = unsafe {
            ffi::ibv_reg_mr(
                pd.ibv_pd.as_ptr(),
                region.as_ptr() as *mut c_void,
                region.len() as u64,
                access.0 as i32,
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
    pub fn rkey(&self) -> u32 {
        unsafe { self.ibv_mr.as_ref().rkey }
    }
    #[inline(always)]
    pub fn lkey(&self) -> u32 {
        unsafe { self.ibv_mr.as_ref().lkey }
    }
    #[inline(always)]
    pub fn length(&self) -> u64 {
        unsafe { self.ibv_mr.as_ref().length }
    }
    #[inline(always)]
    pub fn handle(&self) -> u32 {
        unsafe { self.ibv_mr.as_ref().handle }
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
        qp_init_attr.qp_type = ffi::ibv_qp_type::IBV_QPT_RC;
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
    pub fn with_attr(pd: &IbvPd, qp_init_attr: &mut IbvQpInitAttr) -> Result<Self, IOError> {
        let ibv_qp = unsafe { ffi::ibv_create_qp(pd.ibv_pd.as_ptr(), qp_init_attr as *mut _) };
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
        qp_attr.qp_state = ffi::ibv_qp_state::IBV_QPS_INIT;
        qp_attr.pkey_index = 0;
        qp_attr.port_num = port_num;
        qp_attr.qp_access_flags = ffi::ibv_access_flags::IBV_ACCESS_LOCAL_WRITE.0
            | ffi::ibv_access_flags::IBV_ACCESS_REMOTE_READ.0
            | ffi::ibv_access_flags::IBV_ACCESS_REMOTE_WRITE.0;

        let ret = unsafe {
            ffi::ibv_modify_qp(
                self.ibv_qp.as_ptr(),
                &mut qp_attr as *mut _,
                (ffi::ibv_qp_attr_mask::IBV_QP_STATE.0
                    | ffi::ibv_qp_attr_mask::IBV_QP_PKEY_INDEX.0
                    | ffi::ibv_qp_attr_mask::IBV_QP_PORT.0
                    | ffi::ibv_qp_attr_mask::IBV_QP_ACCESS_FLAGS.0) as i32,
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
        qp_attr.qp_state = ffi::ibv_qp_state::IBV_QPS_RTR;
        qp_attr.path_mtu = ffi::ibv_mtu::IBV_MTU_1024;
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
                (ffi::ibv_qp_attr_mask::IBV_QP_STATE.0
                    | ffi::ibv_qp_attr_mask::IBV_QP_AV.0
                    | ffi::ibv_qp_attr_mask::IBV_QP_PATH_MTU.0
                    | ffi::ibv_qp_attr_mask::IBV_QP_DEST_QPN.0
                    | ffi::ibv_qp_attr_mask::IBV_QP_RQ_PSN.0
                    | ffi::ibv_qp_attr_mask::IBV_QP_MAX_DEST_RD_ATOMIC.0
                    | ffi::ibv_qp_attr_mask::IBV_QP_MIN_RNR_TIMER.0) as i32,
            )
        };
        if ret == -1 {
            return Err(IOError::last_os_error());
        }
        Ok(())
    }

    pub fn modify_rtr2rts(&self, psn: u32) -> Result<(), IOError> {
        let mut qp_attr = unsafe { std::mem::zeroed::<ffi::ibv_qp_attr>() };
        qp_attr.qp_state = ffi::ibv_qp_state::IBV_QPS_RTS;
        qp_attr.timeout = 14;
        qp_attr.retry_cnt = 7;
        qp_attr.rnr_retry = 7;
        qp_attr.sq_psn = psn;
        qp_attr.max_rd_atomic = 1;
        let ret = unsafe {
            ffi::ibv_modify_qp(
                self.ibv_qp.as_ptr(),
                &mut qp_attr as *mut _,
                (ffi::ibv_qp_attr_mask::IBV_QP_STATE.0
                    | ffi::ibv_qp_attr_mask::IBV_QP_TIMEOUT.0
                    | ffi::ibv_qp_attr_mask::IBV_QP_RETRY_CNT.0
                    | ffi::ibv_qp_attr_mask::IBV_QP_RNR_RETRY.0
                    | ffi::ibv_qp_attr_mask::IBV_QP_SQ_PSN.0
                    | ffi::ibv_qp_attr_mask::IBV_QP_MAX_QP_RD_ATOMIC.0) as i32,
            )
        };
        if ret == -1 {
            return Err(IOError::last_os_error());
        }
        Ok(())
    }
    #[inline(always)]
    pub fn qpn(&self) -> u32 {
        unsafe { self.ibv_qp.as_ref().qp_num }
    }
    pub fn query(&self, attr_mask: u32) -> Result<(IbvQpAttr, IbvQpInitAttr), IOError> {
        let mut ibv_qp_attr = unsafe { std::mem::zeroed::<ffi::ibv_qp_attr>() };
        let mut ibv_qp_init_attr = unsafe { std::mem::zeroed::<ffi::ibv_qp_init_attr>() };
        let ret = unsafe {
            ffi::ibv_query_qp(
                self.ibv_qp.as_ptr(),
                &mut ibv_qp_attr as *mut _,
                attr_mask as i32,
                &mut ibv_qp_init_attr as *mut _,
            )
        };
        if ret != 0 {
            return Err(IOError::last_os_error());
        }
        return Ok((ibv_qp_attr, ibv_qp_init_attr));
    }
    pub fn post_send(
        &self,
        wr: &IbvSendWr,
        bad_wr: *const *const IbvSendWr,
    ) -> Result<(), IOError> {
        let ibv_post_send = unsafe { (*(*self.ibv_qp.as_ptr()).context).ops.post_send.unwrap() };
        let ret = unsafe {
            ibv_post_send(
                self.ibv_qp.as_ptr(),
                wr as *const _ as *mut _,
                bad_wr as *mut _,
            )
        };
        if ret == -1 {
            return Err(IOError::last_os_error());
        }
        Ok(())
    }
    pub fn post_recv(
        &self,
        wr: &IbvRecvWr,
        bad_wr: *const *const IbvRecvWr,
    ) -> Result<(), IOError> {
        let ibv_post_recv = unsafe { (*(*self.ibv_qp.as_ptr()).context).ops.post_recv.unwrap() };
        let ret = unsafe {
            ibv_post_recv(
                self.ibv_qp.as_ptr(),
                wr as *const _ as *mut _,
                bad_wr as *mut _,
            )
        };
        if ret == -1 {
            return Err(IOError::last_os_error());
        }
        Ok(())
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
    pub fn fw_ver(&self) -> &str {
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
    pub fn node_guid(&self) -> u64 {
        self.node_guid
    }
    #[inline(always)]
    pub fn sys_image_guid(&self) -> u64 {
        self.sys_image_guid
    }
    #[inline(always)]
    pub fn max_mr_size(&self) -> u64 {
        self.max_mr_size
    }
    #[inline(always)]
    pub fn page_size_cap(&self) -> u64 {
        self.page_size_cap
    }
    #[inline(always)]
    pub fn vendor_id(&self) -> u32 {
        self.vendor_id
    }
    #[inline(always)]
    pub fn vendor_part_id(&self) -> u32 {
        self.vendor_part_id
    }
    #[inline(always)]
    pub fn hw_ver(&self) -> u32 {
        self.hw_ver
    }
    #[inline(always)]
    pub fn max_qp(&self) -> i32 {
        self.max_qp
    }
    #[inline(always)]
    pub fn max_qp_wr(&self) -> i32 {
        self.max_qp_wr
    }
    #[inline(always)]
    pub fn device_cap_flags(&self) -> u32 {
        self.device_cap_flags
    }
    #[inline(always)]
    pub fn max_sge(&self) -> i32 {
        self.max_sge
    }
    #[inline(always)]
    pub fn max_sge_rd(&self) -> i32 {
        self.max_sge_rd
    }
    #[inline(always)]
    pub fn max_cq(&self) -> i32 {
        self.max_cq
    }
    #[inline(always)]
    pub fn max_cqe(&self) -> i32 {
        self.max_cqe
    }
    #[inline(always)]
    pub fn max_mr(&self) -> i32 {
        self.max_mr
    }
    #[inline(always)]
    pub fn max_pd(&self) -> i32 {
        self.max_pd
    }
    #[inline(always)]
    pub fn max_qp_rd_atom(&self) -> i32 {
        self.max_qp_rd_atom
    }
    #[inline(always)]
    pub fn max_ee_rd_atom(&self) -> i32 {
        self.max_ee_rd_atom
    }
    #[inline(always)]
    pub fn max_res_rd_atom(&self) -> i32 {
        self.max_res_rd_atom
    }
    #[inline(always)]
    pub fn max_qp_init_rd_atom(&self) -> i32 {
        self.max_qp_init_rd_atom
    }
    #[inline(always)]
    pub fn max_ee_init_rd_atom(&self) -> i32 {
        self.max_ee_init_rd_atom
    }
    #[inline(always)]
    pub fn atomic_cap(&self) -> u32 {
        self.atomic_cap
    }
    #[inline(always)]
    pub fn max_ee(&self) -> i32 {
        self.max_ee
    }
    #[inline(always)]
    pub fn max_rdd(&self) -> i32 {
        self.max_rdd
    }
    #[inline(always)]
    pub fn max_mw(&self) -> i32 {
        self.max_mw
    }
    #[inline(always)]
    pub fn max_raw_ipv6_pq(&self) -> i32 {
        self.max_raw_ipv6_qp
    }
    #[inline(always)]
    pub fn max_raw_ethy_qp(&self) -> i32 {
        self.max_raw_ethy_qp
    }
    #[inline(always)]
    pub fn max_mcast_grp(&self) -> i32 {
        self.max_mcast_grp
    }
    #[inline(always)]
    pub fn max_mcast_qp_attach(&self) -> i32 {
        self.max_mcast_qp_attach
    }
    #[inline(always)]
    pub fn max_total_mcast_qp_attach(&self) -> i32 {
        self.max_total_mcast_qp_attach
    }
    #[inline(always)]
    pub fn max_ah(&self) -> i32 {
        self.max_ah
    }
    #[inline(always)]
    pub fn max_fmr(&self) -> i32 {
        self.max_fmr
    }
    #[inline(always)]
    pub fn max_map_per_fmr(&self) -> i32 {
        self.max_map_per_fmr
    }
    #[inline(always)]
    pub fn max_srq(&self) -> i32 {
        self.max_srq
    }
    #[inline(always)]
    pub fn max_srq_wr(&self) -> i32 {
        self.max_srq_wr
    }
    #[inline(always)]
    pub fn max_srq_sge(&self) -> i32 {
        self.max_srq_sge
    }
    #[inline(always)]
    pub fn max_pkeys(&self) -> u16 {
        self.max_pkeys
    }
    #[inline(always)]
    pub fn local_ca_ack_delay(&self) -> u8 {
        self.local_ca_ack_delay
    }
    #[inline(always)]
    pub fn phys_port_cnt(&self) -> u8 {
        self.phys_port_cnt
    }
}

impl IbvPortAttr {
    #[inline(always)]
    pub fn state(&self) -> u32 {
        self.state
    }
    #[inline(always)]
    pub fn max_mtu(&self) -> u32 {
        self.max_mtu
    }
    #[inline(always)]
    pub fn active_mtu(&self) -> u32 {
        self.active_mtu
    }
    #[inline(always)]
    pub fn gid_tbl_len(&self) -> i32 {
        self.gid_tbl_len
    }
    #[inline(always)]
    pub fn port_cap_flags(&self) -> u32 {
        self.port_cap_flags
    }
    #[inline(always)]
    pub fn max_msg_sz(&self) -> u32 {
        self.max_msg_sz
    }
    #[inline(always)]
    pub fn bad_pkey_cntr(&self) -> u32 {
        self.bad_pkey_cntr
    }
    #[inline(always)]
    pub fn qkey_viol_cntr(&self) -> u32 {
        self.qkey_viol_cntr
    }
    #[inline(always)]
    pub fn pkey_tbl_len(&self) -> u16 {
        self.pkey_tbl_len
    }
    #[inline(always)]
    pub fn lid(&self) -> u16 {
        self.lid
    }
    #[inline(always)]
    pub fn sm_lid(&self) -> u16 {
        self.sm_lid
    }
    #[inline(always)]
    pub fn lmc(&self) -> u8 {
        self.lmc
    }
    #[inline(always)]
    pub fn max_vl_num(&self) -> u8 {
        self.max_vl_num
    }
    #[inline(always)]
    pub fn sm_sl(&self) -> u8 {
        self.sm_sl
    }
    #[inline(always)]
    pub fn subnet_timeout(&self) -> u8 {
        self.subnet_timeout
    }
    #[inline(always)]
    pub fn init_type_reply(&self) -> u8 {
        self.init_type_reply
    }
    #[inline(always)]
    pub fn active_width(&self) -> u8 {
        self.active_width
    }
    #[inline(always)]
    pub fn active_speed(&self) -> u8 {
        self.active_speed
    }
    #[inline(always)]
    pub fn getphys_state(&self) -> u8 {
        self.phys_state
    }
    #[inline(always)]
    pub fn link_layer(&self) -> u8 {
        self.link_layer
    }
    #[inline(always)]
    pub fn flags(&self) -> u8 {
        self.flags
    }
    #[inline(always)]
    pub fn port_cap_flags2(&self) -> u16 {
        self.port_cap_flags2
    }
}

impl IbvGid {
    #[inline(always)]
    pub fn subnet_prefix(&self) -> u64 {
        unsafe { self.global.subnet_prefix }
    }
    #[inline(always)]
    pub fn interface_id(&self) -> u64 {
        unsafe { self.global.interface_id }
    }
}

impl IbvQpInitAttr {
    #[inline(always)]
    pub fn set_send_cq(&mut self, send_cq: &IbvCq) {
        self.send_cq = send_cq.ibv_cq.as_ptr();
    }
    #[inline(always)]
    pub fn set_recv_cq(&mut self, recv_cq: &IbvCq) {
        self.recv_cq = recv_cq.ibv_cq.as_ptr();
    }
    #[inline(always)]
    pub fn set_max_send_wr(&mut self, max_send_wr: u32) {
        self.cap.max_send_wr = max_send_wr;
    }
    #[inline(always)]
    pub fn set_max_recv_wr(&mut self, max_recv_wr: u32) {
        self.cap.max_recv_wr = max_recv_wr;
    }
    #[inline(always)]
    pub fn set_max_send_sge(&mut self, max_send_sge: u32) {
        self.cap.max_send_sge = max_send_sge;
    }
    #[inline(always)]
    pub fn set_max_recv_sge(&mut self, max_recv_sge: u32) {
        self.cap.max_recv_sge = max_recv_sge;
    }
    #[inline(always)]
    pub fn set_max_inine_data(&mut self, max_inline_data: u32) {
        self.cap.max_inline_data = max_inline_data;
    }
    #[inline(always)]
    pub fn set_qp_type(&mut self, qp_type: u32) {
        self.qp_type = qp_type;
    }
    #[inline(always)]
    pub fn set_sq_sig_all(&mut self, sq_sig_all: i32) {
        self.sq_sig_all = sq_sig_all;
    }
}
pub fn ibv_fork_init() -> Result<(), IOError> {
    let ret = unsafe { ffi::ibv_fork_init() };
    if ret != 0 {
        return Err(IOError::last_os_error());
    }
    Ok(())
}
