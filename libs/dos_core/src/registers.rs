// Ver: 1

#[derive(Debug, Clone, Default)]
pub(crate) struct Registers {
    eax: u32,
    ebx: u32,
    ecx: u32,
    edx: u32,
    esi: u32,
    edi: u32,
    ebp: u32,
    esp: u32,
    ip: u16,
    cs: u16,
    ds: u16,
    es: u16,
    ss: u16,
    fs: u16,
    gs: u16,
    flags: u16,
    eflags: u16,
    rflags: u32,
}

impl Registers {
    pub(crate) fn step(&mut self, step: Option<u16>) {
        if let Some(step) = step {
            self.ip = self.ip.wrapping_add(step);
        } else {
            self.ip = self.ip.wrapping_add(1);
        }
    }
}

// eax
impl Registers {
    pub(crate) fn ah(&self) -> u8 {
        (self.ax() >> 8) as u8
    }
    pub(crate) fn al(&self) -> u8 {
        self.ax() as u8
    }

    pub(crate) fn ax(&self) -> u16 {
        self.eax as u16
    }

    pub(crate) fn eax(&self) -> u32 {
        self.eax
    }

    pub(crate) fn set_eax(&mut self, eax: u32) {
        self.eax = eax;
    }
    pub(crate) fn set_ax(&mut self, val: u16) {
        self.eax = (self.eax & 0xFFFF0000) | (val as u32);
    }

    pub(crate) fn set_al(&mut self, val: u8) {
        self.set_ax((self.ax() & 0xFF00) | (val as u16));
    }
    pub(crate) fn set_ah(&mut self, val: u8) {
        self.set_ax(((val as u16) << 8) | (self.ax() & 0x00FF));
    }
}

// ebx
impl Registers {
    pub(crate) fn bh(&self) -> u8 {
        (self.bx() >> 8) as u8
    }

    pub(crate) fn bl(&self) -> u8 {
        (self.bx() >> 0) as u8
    }

    pub(crate) fn bx(&self) -> u16 {
        self.ebx as u16
    }

    pub(crate) fn ebx(&self) -> u32 {
        self.ebx
    }

    pub(crate) fn set_ebx(&mut self, ebx: u32) {
        self.ebx = ebx;
    }
    pub(crate) fn set_bx(&mut self, val: u16) {
        self.ebx = (self.ebx & 0xFFFF0000) | (val as u32);
    }

    pub(crate) fn set_bl(&mut self, val: u8) {
        self.set_bx((self.bx() & 0xFF00) | (val as u16));
    }
    pub(crate) fn set_bh(&mut self, val: u8) {
        self.set_bx(((val as u16) << 8) | (self.bx() & 0x00FF));
    }
}

// ecx
impl Registers {
    pub(crate) fn ch(&self) -> u8 {
        (self.cx() >> 8) as u8
    }
    pub(crate) fn cl(&self) -> u8 {
        self.cx() as u8
    }

    pub(crate) fn cx(&self) -> u16 {
        self.ecx as u16
    }

    pub(crate) fn ecx(&self) -> u32 {
        self.ecx
    }

    pub(crate) fn set_ecx(&mut self, ecx: u32) {
        self.ecx = ecx;
    }
    pub(crate) fn set_cx(&mut self, val: u16) {
        self.ecx = (self.ecx & 0xFFFF0000) | (val as u32);
    }

    pub(crate) fn set_cl(&mut self, val: u8) {
        self.set_cx((self.cx() & 0xFF00) | (val as u16));
    }
    pub(crate) fn set_ch(&mut self, val: u8) {
        self.set_cx(((val as u16) << 8) | (self.cx() & 0x00FF));
    }
}

// edx
impl Registers {
    pub(crate) fn dh(&self) -> u8 {
        (self.dx() >> 8) as u8
    }
    pub(crate) fn dl(&self) -> u8 {
        self.dx() as u8
    }

    pub(crate) fn dx(&self) -> u16 {
        self.edx as u16
    }

    pub(crate) fn edx(&self) -> u32 {
        self.edx
    }

    pub(crate) fn set_edx(&mut self, edx: u32) {
        self.edx = edx;
    }
    pub(crate) fn set_dx(&mut self, val: u16) {
        self.edx = (self.edx & 0xFFFF0000) | (val as u32);
    }

    pub(crate) fn set_dl(&mut self, val: u8) {
        self.set_dx((self.dx() & 0xFF00) | (val as u16));
    }
    pub(crate) fn set_dh(&mut self, val: u8) {
        self.set_dx(((val as u16) << 8) | (self.dx() & 0x00FF));
    }
}

// esi
impl Registers {
    pub(crate) fn si(&self) -> u16 {
        self.esi as u16
    }

    pub(crate) fn esi(&self) -> u32 {
        self.esi
    }

    pub(crate) fn set_esi(&mut self, esi: u32) {
        self.esi = esi;
    }
    pub(crate) fn set_si(&mut self, val: u16) {
        self.esi = (self.esi & 0xFFFF0000) | (val as u32);
    }
}

// edi
impl Registers {
    pub(crate) fn di(&self) -> u16 {
        self.edi as u16
    }

    pub(crate) fn edi(&self) -> u32 {
        self.edi
    }

    pub(crate) fn set_edi(&mut self, edi: u32) {
        self.edi = edi;
    }
    pub(crate) fn set_di(&mut self, val: u16) {
        self.edi = (self.edi & 0xFFFF0000) | (val as u32);
    }
}

// ebp
impl Registers {
    pub(crate) fn bp(&self) -> u16 {
        self.ebp as u16
    }

    pub(crate) fn ebp(&self) -> u32 {
        self.ebp
    }

    pub(crate) fn set_ebp(&mut self, ebp: u32) {
        self.ebp = ebp;
    }
    pub(crate) fn set_bp(&mut self, val: u16) {
        self.ebp = (self.ebp & 0xFFFF0000) | (val as u32);
    }
}

// esp
impl Registers {
    pub(crate) fn sp(&self) -> u16 {
        self.esp as u16
    }

    pub(crate) fn esp(&self) -> u32 {
        self.esp
    }

    pub(crate) fn set_esp(&mut self, esp: u32) {
        self.esp = esp;
    }
    pub(crate) fn set_sp(&mut self, val: u16) {
        self.esp = (self.esp & 0xFFFF0000) | (val as u32);
    }
}

// eip
impl Registers {
    pub(crate) fn ip(&self) -> u16 {
        self.ip as u16
    }

    /*pub(crate) fn eip(&self) -> u32 {
        self.eip
    }

    pub(crate) fn set_eip(&mut self, eip: u32) {
        self.eip = eip;
    }
    */
    pub(crate) fn set_ip(&mut self, ip: u16) {
        //self.eip = (self.eip & 0xFFFF0000) | (val as u32);
        self.ip = ip;
    }
}

impl Registers {
    pub(crate) fn ss(&self) -> u16 {
        self.ss
    }

    pub(crate) fn cs(&self) -> u16 {
        self.cs
    }

    pub(crate) fn es(&self) -> u16 {
        self.es
    }

    pub(crate) fn ds(&self) -> u16 {
        self.ds
    }

    pub(crate) fn gs(&self) -> u16 {
        self.gs
    }

    pub(crate) fn flags(&self) -> u16 {
        self.flags
    }

    pub(crate) fn set_cs(&mut self, cs: u16) {
        self.cs = cs;
    }

    pub(crate) fn set_ds(&mut self, ds: u16) {
        self.ds = ds;
    }

    pub(crate) fn set_es(&mut self, es: u16) {
        self.es = es;
    }

    pub(crate) fn set_ss(&mut self, ss: u16) {
        self.ss = ss;
    }

    pub(crate) fn set_flags(&mut self, flags: u16) {
        self.flags = flags;
    }

    pub(crate) fn fs(&self) -> u16 {
        self.fs
    }

    pub(crate) fn set_fs(&mut self, fs: u16) {
        self.fs = fs;
    }

    pub(crate) fn set_gs(&mut self, gs: u16) {
        self.gs = gs;
    }
}
