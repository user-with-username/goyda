pub struct MethodSpec { pub name: &'static str, pub descriptor: &'static str }
pub struct ListenerSpec { pub class_name: &'static str, pub interface_name: &'static str, pub methods: &'static [MethodSpec] }

inventory::collect!(ListenerSpec);

struct Cp<'a> {
    buf: Vec<u8>,
    strings: Vec<(&'a str, u16)>,
    current_index: u16,
}

impl<'a> Cp<'a> {
    fn new() -> Self { 
        Self { 
            buf: Vec::new(), 
            strings: Vec::new(), 
            current_index: 1,
        } 
    }
    
    fn u16(&mut self, v: u16) { self.buf.extend_from_slice(&v.to_be_bytes()); }

    fn tag(&mut self, t: u8, a: u16, b: u16) -> u16 { 
        self.buf.push(t); 
        self.u16(a); 
        if t > 7 { self.u16(b); } 
        
        let index = self.current_index;
        self.current_index += 1;
        index
    }

    fn utf8(&mut self, s: &'a str) -> u16 {
        if let Some(&(_, index)) = self.strings.iter().find(|&&(x, _)| x == s) { 
            return index; 
        }
        
        self.buf.push(1);
        self.u16(s.len() as u16);
        self.buf.extend_from_slice(s.as_bytes());
        
        let index = self.current_index;
        self.strings.push((s, index));
        self.current_index += 1;
        index
    }
}

pub fn build_listener_class(spec: &ListenerSpec) -> Vec<u8> {
    let mut cp = Cp::new();

    let s_obj = cp.utf8("java/lang/Object");
    let c_obj = cp.tag(7, s_obj, 0);

    let s_this = cp.utf8(spec.class_name);
    let c_this = cp.tag(7, s_this, 0);

    let s_iface = cp.utf8(spec.interface_name);
    let c_iface = cp.tag(7, s_iface, 0);
    
    let s_nat_ptr = cp.utf8("nativePtr");
    let s_j_desc = cp.utf8("J");
    let nat_ptr_nat = cp.tag(12, s_nat_ptr, s_j_desc);
    let f_ref = cp.tag(9, c_this, nat_ptr_nat);

    let s_init = cp.utf8("<init>");
    let s_void_desc = cp.utf8("()V");
    let init_nat = cp.tag(12, s_init, s_void_desc);
    let m_ref = cp.tag(10, c_obj, init_nat);
    
    let attr_code = cp.utf8("Code");
    let init_desc = cp.utf8("(J)V");
    let init_name = cp.utf8("<init>");

    let methods: Vec<(u16, u16)> = spec.methods.iter().map(|m| (cp.utf8(m.name), cp.utf8(m.descriptor))).collect();
    
    let cp_count = cp.current_index;

    let mut out = Vec::with_capacity(512);
    out.extend_from_slice(&0xCAFEBABEu32.to_be_bytes());
    out.extend_from_slice(&[0, 0, 0, 52]);
    out.extend_from_slice(&cp_count.to_be_bytes());
    out.extend_from_slice(&cp.buf);

    {
        let mut w = |v: u32, size: usize| { out.extend_from_slice(&v.to_be_bytes()[4 - size..]); };

        w(0x0021, 2); w(c_this as u32, 2); w(c_obj as u32, 2); 
        w(1, 2); w(c_iface as u32, 2); 

        w(1, 2); w(0x0002, 2); w(s_nat_ptr as u32, 2); w(s_j_desc as u32, 2); w(0, 2);

        w(spec.methods.len() as u32 + 1, 2);

        w(0x0001, 2); w(init_name as u32, 2); w(init_desc as u32, 2); w(1, 2); 
        w(attr_code as u32, 2); w(22, 4); 
        w(3, 2); w(3, 2); w(10, 4); 
        
        w(0x2a, 1); w(0xb7, 1); w(m_ref as u32, 2);
        w(0x2a, 1); w(0x1f, 1); w(0xb5, 1); w(f_ref as u32, 2);
        w(0xb1, 1);

        w(0, 2); w(0, 2); 

        for (name, desc) in methods {
            w(0x0101, 2); w(name as u32, 2); w(desc as u32, 2); w(0, 2); 
        }

        w(0, 2); 
    }

    out
}
