//! `Lua::Binding::BindModule` (`0x82aaf0b8`) over the generated tables: each
//! module becomes a global table of natives and enum tables. A native that is
//! the shared empty function returns nothing, as it does natively. A function
//! the port has not reconstructed is bound to `unported`, which raises a Lua
//! error naming it, so a script never silently gets a fabricated result.

use super::{
    binding_table::{EMPTY_NATIVE, Entry, MESSAGE_EXPORTERS, MODULES},
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
};

/// `li r3, 0; blr`.
fn empty<H>(_: &mut Lua<H>, _: &mut H, _: &NativeCall) -> LuaResult<Vec<Value>> {
    Ok(Vec::new())
}

fn unported<H>(lua: &mut Lua<H>, _: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let name = call.upvalues.first().map(|v| lua.display(v)).unwrap_or_default();
    Err(lua.error(format!("unported native {name}")))
}

/// Binds the named modules in `PerformLuaCBinding` order. `natives` supplies
/// reconstructed functions as (module, function, implementation).
pub(crate) fn bind_modules<H>(lua: &mut Lua<H>, modules: &[&str], natives: &[(&str, &str, NativeFn<H>)]) {
    for &module in modules {
        let Some(binding) = MODULES.iter().find(|m| m.name == Some(module)) else {
            panic!("binding table has no module {module}");
        };
        let table = lua.create_table();
        for entry in binding.entries {
            match entry {
                Entry::Function(name, address) => {
                    let native = natives.iter().find(|(m, f, _)| *m == module && f == name);
                    let value = match native {
                        Some((_, _, function)) => lua.create_native(name, *function, Vec::new()),
                        None if *address == EMPTY_NATIVE => lua.create_native(name, empty::<H>, Vec::new()),
                        None => {
                            let full = Value::string(&format!("{module}.{name}"));
                            lua.create_native(name, unported::<H>, vec![full])
                        }
                    };
                    lua.table_mut(table).set_str(name, value);
                }
                Entry::NumberEnum(name, values) => {
                    let e = lua.create_table();
                    for (key, value) in *values {
                        lua.table_mut(e).set_str(key, Value::Number(*value));
                    }
                    lua.table_mut(table).set_str(name, Value::Table(e));
                }
                Entry::StringEnum(name, values) => {
                    let e = lua.create_table();
                    for (key, value) in *values {
                        lua.table_mut(e).set_str(key, Value::string(value));
                    }
                    lua.table_mut(table).set_str(name, Value::Table(e));
                }
            }
        }
        lua.set_global(module, Value::Table(table));
    }
}

/// `BindAllMessageExportersToLua` (`0x828cf970`): global `cMsg*` posting
/// functions. `exporters` supplies the reconstructed ones.
pub(crate) fn bind_message_exporters<H>(lua: &mut Lua<H>, exporters: &[(&str, NativeFn<H>)]) {
    for name in MESSAGE_EXPORTERS {
        let native = match exporters.iter().find(|(exporter, _)| exporter == name) {
            Some((_, function)) => lua.create_native(name, *function, Vec::new()),
            None => lua.create_native(name, unported::<H>, vec![Value::string(name)]),
        };
        lua.set_global(name, native);
    }
}
