macro_rules! anything_to_nothing {
    ($($tt:tt)*) => { _ }
}

macro_rules! dyn_call {
    ( $str:literal ($($arg:expr),* $(,)?)) => {{
        let func: fn($(anything_to_nothing!($arg)),*) -> _ = unsafe { core::mem::transmute(get_sym($str)) };
        func($($arg),*)
    }};
    
    ($name:ident ($($arg:expr),* $(,)?)) => {{
        let func: fn($(anything_to_nothing!($arg)),*) -> _ = unsafe { core::mem::transmute(get_sym($name)) };
        func($($arg),*)
    }};
}

use goblin::Object;

fn get_sym_offset(name: &str) -> usize {
    let argv0 = std::env::args().nth(0).unwrap();
    let executable = std::fs::read(argv0).unwrap();

    match Object::parse(&executable).unwrap() {
        Object::Elf(elf) => {
            let sym = elf.dynsyms.iter().find(|sym| elf.dynstrtab.get(sym.st_name).unwrap().unwrap() == name);
            let sym = match sym {
                Some(sym) => sym,
                None => panic!("Symbol '{}' not found. Be sure you're using #[no_mangle].", name)
            };

            sym.st_value as usize
        }
        _ => todo!("Only linux is supported currently")
    }
}

fn get_sym(name: &str) -> *const () {
    (((indicator as usize) - get_sym_offset("indicator")) + get_sym_offset(name)) as *const ()
}

#[no_mangle] pub fn indicator() {}

#[no_mangle]
pub extern "Rust" fn test() -> u32 {
    3
}

#[no_mangle]
pub extern "Rust" fn bar(x: u32) -> u32 {
    2 * x + 1
}

fn main() {
    let x: u32 = dyn_call!(
        "test"()
    );

    let func_name = "bar";

    let y: u32 = dyn_call!(
        func_name(5)
    );

    dbg!(x);
    dbg!(y);
}
