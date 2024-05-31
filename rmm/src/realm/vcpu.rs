use crate::gic;
use crate::realm::context::Context;
use crate::realm::rd::Rd;
use crate::realm::registry::VMID_SET;
use crate::realm::timer;
use crate::rmi::error::Error;

use aarch64_cpu::registers::*;
use alloc::sync::{Arc, Weak};
use armv9a::bits_in_reg;
use spin::Mutex;

extern crate alloc;

#[repr(C)]
#[derive(Debug)]
pub struct VCPU {
    pub context: Context,
    pub state: State,
    pub pcpu: Option<usize>,
    me: Weak<Mutex<Self>>,
}

impl VCPU {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::<Mutex<Self>>::new_cyclic(|me| {
            Mutex::new(Self {
                me: me.clone(),
                state: State::Ready,
                context: Context::new(),
                pcpu: None,
            })
        })
    }

    pub fn into_current(&mut self) {
        unsafe {
            Context::into_current(self);

            core::mem::forget(self.me.upgrade().unwrap());
        }
        self.state = State::Running;
    }

    pub fn from_current(&mut self) {
        unsafe {
            Context::from_current(self);

            let ptr = Arc::into_raw(self.me.upgrade().unwrap());
            Arc::decrement_strong_count(ptr);
            Arc::from_raw(ptr);
        }
        self.state = State::Ready;
    }

    pub fn is_realm_dead(&self) -> bool {
        // XXX: is this function necessary?
        false
    }
}

impl Drop for VCPU {
    fn drop(&mut self) {
        info!("VCPU dropeed!");
    }
}

#[derive(Copy, Clone, Debug)]
pub enum State {
    Null = 0,
    Ready = 1,
    Running = 2,
}

pub unsafe fn current() -> Option<&'static mut VCPU> {
    match TPIDR_EL2.get() {
        0 => None,
        current => Some(&mut *(current as *mut VCPU)),
    }
}

pub fn create_vcpu(rd: &mut Rd, mpidr: u64) -> Result<usize, Error> {
    let page_table = rd.s2_table().lock().get_base_address() as u64;
    let vttbr = bits_in_reg(
        VTTBR_EL2::VMID.mask << VTTBR_EL2::VMID.shift,
        rd.id() as u64,
    ) | bits_in_reg(
        VTTBR_EL2::BADDR.mask << VTTBR_EL2::BADDR.shift,
        page_table >> 1,
    );

    let vcpu = VCPU::new();
    vcpu.lock().context.sys_regs.vttbr = vttbr;
    vcpu.lock().context.sys_regs.vmpidr = mpidr | (MPIDR_EL1::RES1.mask << MPIDR_EL1::RES1.shift);
    timer::init_timer(&mut vcpu.lock());
    gic::init_gic(&mut vcpu.lock());

    rd.vcpus.push(vcpu);
    let vcpuid = rd.vcpus.len() - 1;
    Ok(vcpuid)
}

pub fn remove(id: usize) -> Result<(), Error> {
    VMID_SET
        .lock()
        .remove(&id)
        .then_some(())
        .ok_or(Error::RmiErrorInput)
}
