#[derive(Debug, Clone, Default)]
pub struct Registers {
    eax: u32,
    ebx: u32,
    cx: u16,
    dx: u16,
    si: u16,
    di: u16,
    bp: u16,
    sp: u16,
    ip: u16,
    cs: u16,
    ds: u16,
    es: u16,
    ss: u16,
    flags: u16,
    eflags: u16,
    rflags: u32,
}

impl Registers {
    pub fn step(&mut self, step: Option<u16>) {
        if let Some(step) = step {
            self.ip = self.ip.wrapping_add(step);
        } else {
            self.ip = self.ip.wrapping_add(1);
        }
    }
    pub fn ah(&self) -> u8 {
        (self.ax() >> 8) as u8
    }
    pub fn al(&self) -> u8 {
        self.ax() as u8
    }

    pub fn ax(&self) -> u16 {
        self.eax as u16
    }

    pub fn eax(&self) -> u32 {
        self.eax
    }

    pub fn bh(&self) -> u8 {
        (self.bx() >> 8) as u8
    }

    pub fn bl(&self) -> u8 {
        (self.bx() >> 0) as u8
    }

    pub fn bx(&self) -> u16 {
        self.ebx as u16
    }

    pub fn ebx(&self) -> u32 {
        self.ebx
    }

    pub fn set_eax(&mut self, eax: u32) {
        self.eax = eax;
    }
    pub fn set_ax(&mut self, val: u16) {
        self.eax = (self.eax & 0xFFFF0000) | (val as u32);
    }

    pub fn set_al(&mut self, val: u8) {
        self.set_ax((self.ax() & 0xFF00) | (val as u16));
    }
    pub fn set_ah(&mut self, val: u8) {
        self.set_ax(((val as u16) << 8) | (self.ax() & 0x00FF));
    }

    pub fn set_ebx(&mut self, ebx: u32) {
        self.ebx = ebx;
    }
    pub fn set_bx(&mut self, val: u16) {
        self.ebx = (self.ebx & 0xFFFF0000) | (val as u32);
    }

    pub fn set_bl(&mut self, val: u8) {
        self.set_bx((self.bx() & 0xFF00) | (val as u16));
    }
    pub fn set_bh(&mut self, val: u8) {
        self.set_bx(((val as u16) << 8) | (self.bx() & 0x00FF));
    }

    pub fn set_dx(&mut self, dx: u16) {
        self.dx = dx;
    }

    pub fn cl(&self) -> u8 {
        (self.cx >> 0) as u8
    }
    pub fn dl(&self) -> u8 {
        (self.dx >> 0) as u8
    }

    pub fn ch(&self) -> u8 {
        (self.cx >> 8) as u8
    }
    pub fn dh(&self) -> u8 {
        (self.dx >> 8) as u8
    }

    pub fn set_cl(&mut self, val: u8) {
        self.cx = (self.cx & 0xFF00) | (val as u16);
    }

    pub fn set_ch(&mut self, val: u8) {
        self.cx = ((val as u16) << 8) | (self.cx & 0x00FF);
    }

    pub fn set_dl(&mut self, val: u8) {
        self.dx = (self.dx & 0xFF00) | (val as u16);
    }

    pub fn set_dh(&mut self, val: u8) {
        self.dx = ((val as u16) << 8) | (self.dx & 0x00FF);
    }
    pub fn ip(&self) -> u16 {
        self.ip
    }

    pub fn ss(&self) -> u16 {
        self.ss
    }

    pub fn cs(&self) -> u16 {
        self.cs
    }

    pub fn cx(&self) -> u16 {
        self.cx
    }

    pub fn dx(&self) -> u16 {
        self.dx
    }

    pub fn sp(&self) -> u16 {
        self.sp
    }

    pub fn bp(&self) -> u16 {
        self.bp
    }

    pub fn si(&self) -> u16 {
        self.si
    }

    pub fn di(&self) -> u16 {
        self.di
    }

    pub fn es(&self) -> u16 {
        self.es
    }

    pub fn ds(&self) -> u16 {
        self.ds
    }

    pub fn flags(&self) -> u16 {
        self.flags
    }

    pub fn set_cs(&mut self, cs: u16) {
        self.cs = cs;
    }

    pub fn set_ds(&mut self, ds: u16) {
        self.ds = ds;
    }

    pub fn set_cx(&mut self, cx: u16) {
        self.cx = cx;
    }

    pub fn set_bp(&mut self, bp: u16) {
        self.bp = bp;
    }

    pub fn set_si(&mut self, si: u16) {
        self.si = si;
    }

    pub fn set_di(&mut self, di: u16) {
        self.di = di;
    }

    pub fn set_es(&mut self, es: u16) {
        self.es = es;
    }

    pub fn set_ip(&mut self, ip: u16) {
        self.ip = ip;
    }

    pub fn set_ss(&mut self, ss: u16) {
        self.ss = ss;
    }

    pub fn set_flags(&mut self, flags: u16) {
        self.flags = flags;
    }

    pub fn set_sp(&mut self, sp: u16) {
        self.sp = sp;
    }
}
