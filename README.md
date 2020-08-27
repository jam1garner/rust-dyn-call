# rust-dyn-call
Do not use. Do not use. Do not use. (Calls a function dynamically)

Example:
```rust
    let x: u32 = dyn_call!(
        "test"()
    );

    let func_name = "bar";

    let y: u32 = dyn_call!(
        func_name(5)
    );

```

## Why?

![](https://cdn.discordapp.com/attachments/376971848555954187/748397660582707234/unknown.png)

## How does it work?

The interface for it is a rather simple macro:

```rust
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
```
It basically just parses the function name (either a string literal or an ident for an `&str` variable) and the args in the function call syntax.
Then it takes the function name and passes it to `get_sym`, which is just a function for reading the executable from
the first arg passed and getting a pointer to a dynsym by name. It then transmutes it to a function pointer so we can call
it. However since the user doesn't have to pass which types they are calling the function with, we need to have them inferred.
In order to do that we use the following macro:

```rust
macro_rules! anything_to_nothing {
    ($($tt:tt)*) => { _ }
}
```

All this does is consume whatever we pass into it and outputs a `_` (for those who are unfamiliar, this is the syntax in
Rust for "infer this type". We can then pass each of our arguments into this macro in order to conver them to a type to
be inferred. So for example...

```rust
dyn_call!(func_name(3, "test"));
```

becomes

```rust
let func: fn(anything_to_nothing!(3)), anything_to_nothing!("test")) -> _ = unsafe { core::mem::transmute(get_sym(func_name)) };
func(3, "test")
```

which then further expands into

```rust
let func: fn(_, _) -> _ = unsafe { core::mem::transmute(get_sym(func_name)) };
func(3, "test")
```

and since `fn(_, _) -> _` is Rust syntax for "a function pointer with a return type and two arguments who types should
be inferred", this allows us to transmute out dynamic symbol into a callable function pointer.

Now you might be asking "jam, why need the macro for the underscores?", which is a good question! The reasoning is that
since we need the amount of underscores to match the number of args, we need to somehow pass each argument *as* its type,
and since we obviously can't have the types be the args themselves, we need a macro to convert these tokens to a valid type.
And since we don't know the types, we need to convert them to underscores for later inferring!

## How `get_sym` works

```rust
fn get_sym(name: &str) -> *const () {
    (((indicator as usize) - get_sym_offset("indicator")) + get_sym_offset(name)) as *const ()
}
```

`get_sym` is rather simple, we just get an offset for a known function (indicator) and the symbol address for it. Since the
function will be located at `base address + offset`, we can subtrace the offset of our known function from the pointer to
it in order to get the base address. We then add the offset for the function we want to know and voila! We have a pointer
to our function, properly relocated to account for [ASLR](https://en.wikipedia.org/wiki/Address_space_layout_randomization).

## How `get_sym_offset` works

This one is probably the least interesting function in the implementation but hey! learning!

```rust
use goblin::Object;

fn get_sym_offset(name: &str) -> usize {
    let argv0 = std::env::args().nth(0).unwrap(); // get first arg
    let executable = std::fs::read(argv0).unwrap(); // read file into memory

    // parse file using goblin
    match Object::parse(&executable).unwrap() {
        Object::Elf(elf) => {
            // Find the symbol with a matching name
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
```

For this, I let [goblin](https://docs.rs/goblin) do 99% of the work. It parses the file for me and I just have to use iterators to search the dynamic
exports for a symbol with a matching name then get the `st_value` for it (which, in this context, is an offset from the base address).
[Recommended reading on what st_name and st_value do](https://refspecs.linuxbase.org/elf/gabi4+/ch4.symtab.html).
No point memorizing information that is just a google away!

(Feel free to PR support for your platform if you want an excuse to learn globlin! It's a great crate and seems like a great way to learn about executable formats if you aren't already familiar with them!)

Oh! I almost forgot. In `.cargo/config` we need a linker flag to tell it to mass export symbols. There are other
ways to get around this, but frankly nobody should ever be doing this so we'll just do it the dirty and easy way:

```toml
[build]
rustflags = ["-C", "link-args=-export-dynamic"] 
```
