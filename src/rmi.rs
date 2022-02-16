use alloc::rc::Rc;
use core::cell::RefCell;
use core::cmp::Ordering;

use crate::smc;

use realm_management_monitor::call;
use realm_management_monitor::communication::{self, Error, ErrorKind};

extern crate alloc;

const RMM_VERSION: usize = 0xc000_0000;
const RMM_GRANULE_DELEGATE: usize = 0xc000_0001;
const RMM_GRANULE_UNDELEGATE: usize = 0xc000_0002;
const RMM_REQ_COMPLETE: usize = 0xc000_0010;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Code {
    Version,
    RequestComplete,
    GranuleDelegate,
    GranuleUndelegate,
    Unknown(usize),
}

impl From<Code> for usize {
    fn from(origin: Code) -> Self {
        match origin {
            Code::Version => RMM_VERSION,
            Code::RequestComplete => RMM_REQ_COMPLETE,
            Code::GranuleDelegate => RMM_GRANULE_DELEGATE,
            Code::GranuleUndelegate => RMM_GRANULE_UNDELEGATE,
            Code::Unknown(remain) => remain,
        }
    }
}

impl From<usize> for Code {
    fn from(origin: usize) -> Self {
        match origin {
            RMM_VERSION => Code::Version,
            RMM_REQ_COMPLETE => Code::RequestComplete,
            RMM_GRANULE_DELEGATE => Code::GranuleDelegate,
            RMM_GRANULE_UNDELEGATE => Code::GranuleUndelegate,
            remain => Code::Unknown(remain),
        }
    }
}

impl Default for Code {
    fn default() -> Self {
        Code::Unknown(0)
    }
}

impl PartialOrd for Code {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Code {
    fn cmp(&self, other: &Self) -> Ordering {
        usize::from(*self).cmp(&usize::from(*other))
    }
}

pub type Argument = [usize; 4];
pub type Return = usize;
pub type Call = call::Context<Code, Argument, Return>;

pub struct Receiver {
    sender: Rc<Sender>,
}

impl Receiver {
    pub fn new() -> Self {
        Self {
            sender: Rc::new(Sender::new()),
        }
    }
}

impl communication::Receiver for Receiver {
    type Event = Call;

    fn recv(&self) -> Result<Call, Error> {
        let mut input = [usize::from(Code::RequestComplete), 0, 0, 0, 0];
        input[1..].copy_from_slice(&self.sender.pop());

        let ret = smc::call(input);

        let code = ret[0];
        let mut args = [0usize; 4];
        args.copy_from_slice(&ret[1..5]);

        Ok(Call::new(Code::from(code), args, self.sender.clone()))
    }
}

pub struct Sender {
    data: RefCell<(usize, [Return; 4])>,
}

impl Sender {
    const fn new() -> Self {
        Self {
            data: RefCell::new((0usize, [0usize; 4])),
        }
    }

    fn pop(&self) -> [Return; 4] {
        let mut d = self.data.borrow_mut();
        let ret = d.1;
        *d = (0usize, [0usize; 4]);
        ret
    }

    fn push(&self, data: usize) -> Result<(), Error> {
        let mut d = self.data.borrow_mut();
        let pos = d.0;
        if pos < 4 {
            d.1[pos] = data;
            d.0 += 1;
            Ok(())
        } else {
            Err(Error::new(ErrorKind::StorageFull))
        }
    }
}

impl communication::Sender for Sender {
    type Event = Return;

    fn send(&self, event: Return) -> Result<(), Error> {
        self.push(event)
    }
}
